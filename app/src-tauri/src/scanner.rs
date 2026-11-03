use crate::metadata::{is_supported_audio, parse_track};
use crate::models::{AlbumSummary, FolderContent, FolderItem};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

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

pub fn list_folder(folder_path: &str) -> Result<FolderContent, String> {
    let path = Path::new(folder_path);
    let mut items = Vec::new();

    let parent_path = path.parent().map(|p| p.to_string_lossy().to_string());

    {
        let entries = fs::read_dir(path).map_err(|e| e.to_string())?;
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
    items.sort_by(|a, b| match (a.is_directory, b.is_directory) {
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
    });

    Ok(FolderContent {
        current_path: folder_path.to_string(),
        parent_path,
        items,
    })
}

pub fn list_albums(root_dir: &str) -> Vec<AlbumSummary> {
    let mut album_map: HashMap<PathBuf, (String, String, usize, String, bool)> = HashMap::new();

    // Scan max 3 levels deep from root_dir for fast response
    for entry in WalkDir::new(root_dir)
        .max_depth(3)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let p = entry.path();
        if is_supported_audio(p) {
            if let Some(parent) = p.parent() {
                // The card shows the first track of the folder, so only that one's tags are read
                let album = album_map.entry(parent.to_path_buf()).or_insert_with(|| {
                    let track = parse_track(p);
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
                album.2 += 1; // Increment track count
            }
        }
    }

    let mut result: Vec<AlbumSummary> = album_map
        .into_iter()
        .map(
            |(path, (album_name, artist_name, track_count, first_track_path, has_artwork))| {
                AlbumSummary {
                    album_name,
                    artist_name,
                    folder_path: path.to_string_lossy().to_string(),
                    track_count,
                    first_track_path,
                    has_artwork,
                }
            },
        )
        .collect();

    result.sort_by_key(|a| a.album_name.to_lowercase());
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::update_tags;
    use crate::models::TagUpdateRequest;
    use crate::test_audio::wav_bytes;

    fn tag(path: &Path, album: &str, track_number: u32) {
        let result = update_tags(&TagUpdateRequest {
            file_paths: vec![path.to_string_lossy().to_string()],
            title: None,
            artist: None,
            album: Some(album.to_string()),
            track_number: Some(track_number),
            clear_track_number: false,
            auto_number_tracks: false,
            filename_to_title: false,
            artwork_data_url: None,
            clear_artwork: false,
        })
        .unwrap();
        assert!(result.failed.is_empty());
    }

    #[test]
    fn a_missing_folder_is_an_error_not_an_empty_list() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("gone");
        assert!(list_folder(&missing.to_string_lossy()).is_err());
    }

    #[test]
    fn folders_come_first_then_tracks_by_number() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("zfolder")).unwrap();
        fs::write(dir.path().join("zfolder/a.mp3"), b"x").unwrap();
        let (first, second) = (dir.path().join("b.wav"), dir.path().join("a.wav"));
        fs::write(&first, wav_bytes()).unwrap();
        fs::write(&second, wav_bytes()).unwrap();
        tag(&first, "Album", 1);
        tag(&second, "Album", 2);
        fs::write(dir.path().join("note.txt"), b"x").unwrap();

        let content = list_folder(&dir.path().to_string_lossy()).unwrap();

        let names: Vec<_> = content.items.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, ["zfolder", "b.wav", "a.wav"]);
        assert_eq!(content.items[0].audio_count, 1);
    }

    #[test]
    fn an_album_uses_the_first_track_in_name_order_and_counts_all() {
        let dir = tempfile::tempdir().unwrap();
        let album = dir.path().join("folder name");
        fs::create_dir(&album).unwrap();
        for name in ["2.wav", "1.wav", "3.wav"] {
            fs::write(album.join(name), wav_bytes()).unwrap();
        }
        tag(&album.join("1.wav"), "First Album", 1);
        tag(&album.join("2.wav"), "Other Album", 2);

        let albums = list_albums(&dir.path().to_string_lossy());

        assert_eq!(albums.len(), 1);
        assert_eq!(albums[0].album_name, "First Album");
        assert_eq!(albums[0].track_count, 3);
        assert!(albums[0].first_track_path.ends_with("1.wav"));
    }

    #[test]
    fn an_album_without_a_tag_is_named_after_its_folder() {
        let dir = tempfile::tempdir().unwrap();
        let album = dir.path().join("folder name");
        fs::create_dir(&album).unwrap();
        fs::write(album.join("1.wav"), wav_bytes()).unwrap();

        let albums = list_albums(&dir.path().to_string_lossy());

        assert_eq!(albums[0].album_name, "folder name");
    }

    #[test]
    fn albums_are_sorted_by_name_ignoring_case() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["b", "A", "c"] {
            let album = dir.path().join(name);
            fs::create_dir(&album).unwrap();
            fs::write(album.join("1.wav"), wav_bytes()).unwrap();
        }

        let albums = list_albums(&dir.path().to_string_lossy());

        let names: Vec<_> = albums.iter().map(|a| a.album_name.as_str()).collect();
        assert_eq!(names, ["A", "b", "c"]);
    }
}
