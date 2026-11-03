use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use crate::metadata::{is_supported_audio, parse_track};
use crate::models::{AlbumSummary, FolderContent, FolderItem};

pub fn get_default_music_dir() -> String {
    dirs::audio_dir()
        .or_else(dirs::home_dir)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "C:\\".to_string())
}

#[cfg(windows)]
pub fn list_drives() -> Vec<String> {
    let mut drives = Vec::new();
    unsafe {
        let mask = windows_sys::Win32::Storage::FileSystem::GetLogicalDrives();
        for i in 0..26 {
            if (mask & (1 << i)) != 0 {
                let letter = (b'A' + i) as char;
                drives.push(format!("{}:\\", letter));
            }
        }
    }
    if drives.is_empty() {
        drives.push("C:\\".to_string());
    }
    drives
}

#[cfg(not(windows))]
pub fn list_drives() -> Vec<String> {
    vec!["/".to_string()]
}

pub fn list_folder(folder_path: &str) -> FolderContent {
    let path = Path::new(folder_path);
    let mut items = Vec::new();

    let parent_path = path.parent().map(|p| p.to_string_lossy().to_string());

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            if entry_path.is_dir() {
                // Count direct audio files in this subfolder
                let mut audio_count = 0;
                if let Ok(sub_entries) = fs::read_dir(&entry_path) {
                    for sub in sub_entries.flatten() {
                        if is_supported_audio(&sub.path()) {
                            audio_count += 1;
                        }
                    }
                }

                items.push(FolderItem {
                    name,
                    path: entry_path.to_string_lossy().to_string(),
                    is_directory: true,
                    audio_count,
                    track: None,
                });
            } else if is_supported_audio(&entry_path) {
                let track = parse_track(&entry_path);
                items.push(FolderItem {
                    name,
                    path: entry_path.to_string_lossy().to_string(),
                    is_directory: false,
                    audio_count: 0,
                    track: Some(track),
                });
            }
        }
    }

    // Sort: directories first (alphabetical), then files (by track number if present, else title)
    items.sort_by(|a, b| {
        match (a.is_directory, b.is_directory) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            (true, true) => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            (false, false) => {
                let track_a_num = a.track.as_ref().and_then(|t| t.track_number);
                let track_b_num = b.track.as_ref().and_then(|t| t.track_number);
                match (track_a_num, track_b_num) {
                    (Some(na), Some(nb)) if na != nb => na.cmp(&nb),
                    _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                }
            }
        }
    });

    FolderContent {
        current_path: folder_path.to_string(),
        parent_path,
        items,
    }
}

pub fn list_albums(root_dir: &str) -> Vec<AlbumSummary> {
    let mut album_map: HashMap<PathBuf, (String, String, usize, String, bool)> = HashMap::new();

    // Scan max 3 levels deep from root_dir for fast response
    for entry in WalkDir::new(root_dir)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let p = entry.path();
        if is_supported_audio(p) {
            if let Some(parent) = p.parent() {
                let track = parse_track(p);
                let entry = album_map.entry(parent.to_path_buf()).or_insert_with(|| {
                    (
                        if track.album != "Unknown Album" {
                            track.album.clone()
                        } else {
                            parent
                                .file_name()
                                .map(|f| f.to_string_lossy().to_string())
                                .unwrap_or_else(|| "Unknown".to_string())
                        },
                        track.artist.clone(),
                        0,
                        p.to_string_lossy().to_string(),
                        track.has_artwork,
                    )
                });
                entry.2 += 1; // Increment track count
                if !entry.4 && track.has_artwork {
                    entry.4 = true;
                }
            }
        }
    }

    let mut result: Vec<AlbumSummary> = album_map
        .into_iter()
        .map(|(path, (album_name, artist_name, track_count, first_track_path, has_artwork))| {
            AlbumSummary {
                album_name,
                artist_name,
                folder_path: path.to_string_lossy().to_string(),
                track_count,
                first_track_path,
                has_artwork,
            }
        })
        .collect();

    result.sort_by(|a, b| a.album_name.to_lowercase().cmp(&b.album_name.to_lowercase()));
    result
}
