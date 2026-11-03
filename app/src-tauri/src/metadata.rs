use std::path::Path;
use lofty::config::WriteOptions;
use lofty::file::AudioFile;
use lofty::picture::{MimeType, Picture, PictureType};
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::Tag;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use crate::models::{AudioTrack, TagUpdateRequest};

pub fn is_supported_audio(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        matches!(
            ext.to_lowercase().as_str(),
            "mp3" | "flac" | "m4a" | "aac" | "ogg" | "opus" | "wav" | "alac" | "wma"
        )
    } else {
        false
    }
}

pub fn parse_track(path: &Path) -> AudioTrack {
    let file_path = path.to_string_lossy().to_string();
    let file_name = path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let mut title = file_name.clone();
    let mut artist = "Unknown Artist".to_string();
    let mut album = "Unknown Album".to_string();
    let mut duration_ms = 0u64;
    let mut track_number = None;
    let mut has_artwork = false;

    if let Ok(tagged_file) = Probe::open(path).and_then(|p| p.read()) {
        let props = tagged_file.properties();
        duration_ms = props.duration().as_millis() as u64;

        if let Some(tag) = tagged_file.primary_tag().or_else(|| tagged_file.first_tag()) {
            if let Some(t) = tag.title() {
                if !t.trim().is_empty() {
                    title = t.trim().to_string();
                }
            }
            if let Some(a) = tag.artist() {
                if !a.trim().is_empty() {
                    artist = a.trim().to_string();
                }
            }
            if let Some(al) = tag.album() {
                if !al.trim().is_empty() {
                    album = al.trim().to_string();
                }
            }
            track_number = tag.track();
            has_artwork = !tag.pictures().is_empty();
        }
    }

    if !has_artwork {
        // Check if directory has cover image
        if let Some(parent) = path.parent() {
            has_artwork = find_folder_artwork(parent).is_some();
        }
    }

    AudioTrack {
        file_path,
        file_name,
        title,
        artist,
        album,
        duration_ms,
        track_number,
        has_artwork,
    }
}

pub fn extract_artwork(path: &Path) -> Option<String> {
    // 1. Try embedded tag
    if let Ok(tagged_file) = Probe::open(path).and_then(|p| p.read()) {
        if let Some(tag) = tagged_file.primary_tag().or_else(|| tagged_file.first_tag()) {
            if let Some(pic) = tag.pictures().first() {
                let mime = pic.mime_type().map(|m| m.as_str()).unwrap_or("image/jpeg");
                let b64 = BASE64.encode(pic.data());
                return Some(format!("data:{};base64,{}", mime, b64));
            }
        }
    }

    // 2. Try parent folder cover
    if let Some(parent) = path.parent() {
        if let Some(cover_path) = find_folder_artwork(parent) {
            if let Ok(data) = std::fs::read(&cover_path) {
                let ext = cover_path.extension().and_then(|e| e.to_str()).unwrap_or("jpeg").to_lowercase();
                let mime = match ext.as_str() {
                    "png" => "image/png",
                    "webp" => "image/webp",
                    _ => "image/jpeg",
                };
                let b64 = BASE64.encode(&data);
                return Some(format!("data:{};base64,{}", mime, b64));
            }
        }
    }

    None
}

pub fn find_folder_artwork(dir: &Path) -> Option<std::path::PathBuf> {
    let candidate_names = [
        "cover.jpg", "cover.jpeg", "cover.png",
        "folder.jpg", "folder.jpeg", "folder.png",
        "front.jpg", "front.jpeg", "front.png",
        "artwork.jpg", "artwork.png",
    ];

    for name in candidate_names {
        let p = dir.join(name);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

fn parse_data_url(data_url: &str) -> Result<(MimeType, Vec<u8>), String> {
    let comma_idx = data_url.find(',').ok_or("Invalid data URL")?;
    let header = &data_url[..comma_idx];
    let b64_part = &data_url[comma_idx + 1..];

    let mime_type = if header.contains("image/png") {
        MimeType::Png
    } else if header.contains("image/gif") {
        MimeType::Gif
    } else if header.contains("image/bmp") {
        MimeType::Bmp
    } else if header.contains("image/tiff") {
        MimeType::Tiff
    } else {
        MimeType::Jpeg
    };

    let bytes = BASE64
        .decode(b64_part.trim())
        .map_err(|e| format!("Failed to decode artwork base64: {}", e))?;

    Ok((mime_type, bytes))
}

pub fn update_tags(req: &TagUpdateRequest) -> Result<Vec<AudioTrack>, String> {
    let parsed_artwork = if let Some(ref data_url) = req.artwork_data_url {
        Some(parse_data_url(data_url)?)
    } else {
        None
    };

    let mut updated_tracks = Vec::new();

    for (index, file_path_str) in req.file_paths.iter().enumerate() {
        let path = Path::new(file_path_str);
        let mut tagged_file = Probe::open(path)
            .and_then(|p| p.read())
            .map_err(|e| format!("Failed to read audio file '{}': {}", file_path_str, e))?;

        let tag = if tagged_file.primary_tag_mut().is_some() {
            tagged_file.primary_tag_mut().unwrap()
        } else if tagged_file.first_tag_mut().is_some() {
            tagged_file.first_tag_mut().unwrap()
        } else {
            let tag_type = tagged_file.primary_tag_type();
            tagged_file.insert_tag(Tag::new(tag_type));
            tagged_file.primary_tag_mut().unwrap()
        };

        // 1. Title
        if req.filename_to_title {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                tag.set_title(stem.to_string());
            }
        } else if let Some(ref title) = req.title {
            tag.set_title(title.clone());
        }

        // 2. Artist
        if let Some(ref artist) = req.artist {
            tag.set_artist(artist.clone());
        }

        // 3. Album
        if let Some(ref album) = req.album {
            tag.set_album(album.clone());
        }

        // 4. Track Number
        if req.auto_number_tracks {
            tag.set_track((index + 1) as u32);
        } else if req.clear_track_number {
            tag.remove_track();
        } else if let Some(num) = req.track_number {
            tag.set_track(num);
        }

        // 5. Artwork
        if req.clear_artwork {
            while !tag.pictures().is_empty() {
                tag.remove_picture(0);
            }
        } else if let Some((ref mime, ref bytes)) = parsed_artwork {
            while !tag.pictures().is_empty() {
                tag.remove_picture(0);
            }
            let pic = Picture::new_unchecked(
                PictureType::CoverFront,
                Some(mime.clone()),
                None,
                bytes.clone(),
            );
            tag.push_picture(pic);
        }

        tagged_file
            .save_to_path(path, WriteOptions::default())
            .map_err(|e| format!("Failed to save tags to '{}': {}", file_path_str, e))?;

        updated_tracks.push(parse_track(path));
    }

    Ok(updated_tracks)
}

