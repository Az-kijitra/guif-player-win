use crate::metadata::{is_supported_audio, parse_track};
use crate::models::{AlbumSummary, FolderContent, FolderItem};
use std::fs;
use std::path::{Path, PathBuf};

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

/// What the album view shows for `root_dir`: what is directly inside it, in name order.
/// A folder with tracks is an album, one without is a folder to open. The folder itself comes
/// first as an album when it holds tracks, so an album's own folder is not shown empty.
pub fn list_albums(root_dir: &str) -> Result<Vec<AlbumSummary>, String> {
    let root = Path::new(root_dir);
    let mut folders = Vec::new();
    let mut has_tracks = false;
    for entry in fs::read_dir(root).map_err(|e| e.to_string())?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            folders.push(path);
        } else if is_supported_audio(&path) {
            has_tracks = true;
        }
    }
    folders.sort_by_key(|p| name_of(p).to_lowercase());

    let mut result = Vec::new();
    if has_tracks {
        result.extend(album_of(root));
    }
    for folder in folders {
        let entry = album_of(&folder).unwrap_or_else(|| AlbumSummary {
            album_name: name_of(&folder),
            artist_name: String::new(),
            folder_path: folder.to_string_lossy().to_string(),
            track_count: 0,
            first_track_path: String::new(),
            has_artwork: false,
            is_folder: true,
        });
        result.push(entry);
    }
    Ok(result)
}

fn name_of(path: &Path) -> String {
    path.file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

/// The folder as an album, if it holds tracks. The card shows the first track in name order,
/// so only that one's tags are read.
fn album_of(dir: &Path) -> Option<AlbumSummary> {
    let mut tracks: Vec<PathBuf> = fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_supported_audio(p))
        .collect();
    tracks.sort_by_key(|p| name_of(p).to_lowercase());
    let first = tracks.first()?;
    let track = parse_track(first);
    Some(AlbumSummary {
        album_name: if track.album != "Unknown Album" {
            track.album.clone()
        } else {
            name_of(dir)
        },
        artist_name: track.artist.clone(),
        folder_path: dir.to_string_lossy().to_string(),
        track_count: tracks.len(),
        first_track_path: first.to_string_lossy().to_string(),
        has_artwork: track.has_artwork,
        is_folder: false,
    })
}

/// The paths that no longer point at a file.
pub fn missing_files(paths: &[String]) -> Vec<String> {
    paths
        .iter()
        .filter(|path| !Path::new(path).is_file())
        .cloned()
        .collect()
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

        let albums = list_albums(&dir.path().to_string_lossy()).unwrap();

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

        let albums = list_albums(&dir.path().to_string_lossy()).unwrap();

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

        let albums = list_albums(&dir.path().to_string_lossy()).unwrap();

        let names: Vec<_> = albums.iter().map(|a| a.album_name.as_str()).collect();
        assert_eq!(names, ["A", "b", "c"]);
    }

    #[test]
    fn the_album_view_lists_what_is_directly_inside() {
        let dir = tempfile::tempdir().unwrap();
        for album in ["Jazz/Blue Night", "Single"] {
            let path = dir.path().join(album);
            fs::create_dir_all(&path).unwrap();
            fs::write(path.join("1.wav"), wav_bytes()).unwrap();
        }
        fs::create_dir(dir.path().join("empty")).unwrap();

        let albums = list_albums(&dir.path().to_string_lossy()).unwrap();

        let shown: Vec<_> = albums
            .iter()
            .map(|a| (a.album_name.as_str(), a.is_folder))
            .collect();
        assert_eq!(
            shown,
            [("empty", true), ("Jazz", true), ("Single", false)],
            "folders without tracks are folders, deeper albums are not listed"
        );
    }

    #[test]
    fn a_folder_with_tracks_shows_itself_first() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("1.wav"), wav_bytes()).unwrap();
        fs::write(dir.path().join("2.wav"), wav_bytes()).unwrap();
        fs::create_dir(dir.path().join("Bonus")).unwrap();
        fs::write(dir.path().join("Bonus").join("1.wav"), wav_bytes()).unwrap();

        let albums = list_albums(&dir.path().to_string_lossy()).unwrap();

        assert_eq!(albums.len(), 2);
        assert_eq!(albums[0].folder_path, dir.path().to_string_lossy());
        assert_eq!(albums[0].track_count, 2);
        assert_eq!(albums[1].album_name, "Bonus");
    }

    #[test]
    fn missing_files_are_reported() {
        let dir = tempfile::tempdir().unwrap();
        let here = dir.path().join("here.wav");
        fs::write(&here, wav_bytes()).unwrap();
        let gone = dir.path().join("gone.wav").to_string_lossy().to_string();
        let folder = dir.path().to_string_lossy().to_string();

        let missing = missing_files(&[
            here.to_string_lossy().to_string(),
            gone.clone(),
            folder.clone(),
        ]);

        assert_eq!(missing, [gone, folder]);
    }
}
