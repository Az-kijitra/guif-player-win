use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioTrack {
    pub file_path: String,
    pub file_name: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
    pub track_number: Option<u32>,
    pub has_artwork: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderItem {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub audio_count: usize,
    pub track: Option<AudioTrack>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderContent {
    pub current_path: String,
    pub parent_path: Option<String>,
    pub items: Vec<FolderItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumSummary {
    pub album_name: String,
    pub artist_name: String,
    pub folder_path: String,
    pub track_count: usize,
    pub first_track_path: String,
    pub has_artwork: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackStateDto {
    pub current_track: Option<AudioTrack>,
    pub is_playing: bool,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub volume: f32,
    pub shuffle_mode: bool,
    pub repeat_mode: u8, // 0 = off, 1 = one, 2 = all
    pub queue: Vec<AudioTrack>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumResume {
    pub album_path: String,
    pub last_file_path: String,
    pub position_ms: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackResume {
    pub file_path: String,
    pub album_path: String,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub is_completed: bool,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub id: i64,
    pub name: String,
    pub track_count: usize,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagUpdateRequest {
    pub file_paths: Vec<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub track_number: Option<u32>,
    pub clear_track_number: bool,
    pub auto_number_tracks: bool,
    pub filename_to_title: bool,
    pub artwork_data_url: Option<String>,
    pub clear_artwork: bool,
}

