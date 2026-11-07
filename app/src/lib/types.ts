export interface AudioTrack {
  file_path: string;
  file_name: string;
  title: string;
  artist: string;
  album: string;
  duration_ms: number;
  track_number: number | null;
  has_artwork: boolean;
}

export interface FolderItem {
  name: string;
  path: string;
  is_directory: boolean;
  audio_count: number;
  track: AudioTrack | null;
}

export interface FolderContent {
  current_path: string;
  parent_path: string | null;
  items: FolderItem[];
}

export interface AlbumSummary {
  album_name: string;
  artist_name: string;
  folder_path: string;
  track_count: number;
  first_track_path: string;
  has_artwork: boolean;
}

export interface PlaybackState {
  current_track: AudioTrack | null;
  is_playing: boolean;
  position_ms: number;
  duration_ms: number;
  volume: number;
  shuffle_mode: boolean;
  repeat_mode: number; // 0 = off, 1 = one, 2 = all
  queue_version: number;
  speed: number;
}

export interface AlbumResume {
  album_path: string;
  last_file_path: string;
  position_ms: number;
  updated_at: number;
}

export interface TrackResume {
  file_path: string;
  album_path: string;
  position_ms: number;
  duration_ms: number;
  is_completed: boolean;
  updated_at: number;
}

export interface Playlist {
  id: number;
  name: string;
  track_count: number;
  created_at: number;
}

export interface TagUpdateRequest {
  file_paths: string[];
  title: string | null;
  artist: string | null;
  album: string | null;
  track_number: number | null;
  clear_track_number: boolean;
  auto_number_tracks: boolean;
  filename_to_title: boolean;
  artwork_data_url: string | null;
  clear_artwork: boolean;
}

export interface TagUpdateFailure {
  file_path: string;
  message: string;
}

export interface TagUpdateResult {
  updated: AudioTrack[];
  failed: TagUpdateFailure[];
}

export type ViewMode = "FOLDER" | "ARTWORK" | "PLAYLIST";

export type ThemePreset =
  | "nord"
  | "default"
  | "monokai"
  | "catppuccin"
  | "solarized"
  | "light";

export type ThemeSetting = ThemePreset | "guif";

export interface Bookmark {
  file_path: string;
  album_path: string;
  title: string;
  album: string;
  position_ms: number;
  duration_ms: number;
  created_at: number;
}
