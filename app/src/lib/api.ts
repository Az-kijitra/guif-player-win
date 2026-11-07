import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import type {
  AlbumResume,
  AlbumSummary,
  AudioTrack,
  Bookmark,
  FolderContent,
  PlaybackState,
  Playlist,
  TagUpdateRequest,
  TagUpdateResult,
  TrackResume,
} from "./types";

export const api = {
  getDefaultPath: () => invoke<string>("get_default_path"),
  listDrives: () => invoke<string[]>("list_drives"),
  listFolder: (path: string) => invoke<FolderContent>("list_folder", { path }),
  listAlbums: (rootPath: string) => invoke<AlbumSummary[]>("list_albums", { rootPath }),
  getArtwork: (filePath: string) => invoke<string | null>("get_artwork", { filePath }),
  updateTracksMetadata: (request: TagUpdateRequest) =>
    invoke<TagUpdateResult>("update_tracks_metadata", { request }),

  // Playback
  onPlaybackError: (handler: (message: string) => void) =>
    listen<string>("playback-error", (event) => handler(event.payload)),
  playTrack: (track: AudioTrack, initialPosMs?: number) =>
    invoke<void>("play_track", { track, initialPosMs }),
  // shuffle: undefined keeps the current setting
  playQueue: (queue: AudioTrack[], startIndex: number, initialPosMs?: number, shuffle?: boolean) =>
    invoke<void>("play_queue", { queue, startIndex, initialPosMs, shuffle }),
  togglePlayPause: () => invoke<void>("toggle_play_pause"),
  pause: () => invoke<void>("pause"),
  resume: () => invoke<void>("resume"),
  seek: (positionMs: number) => invoke<void>("seek", { positionMs }),
  setVolume: (volume: number) => invoke<void>("set_volume", { volume }),
  playNext: () => invoke<void>("play_next"),
  playPrevious: () => invoke<void>("play_previous"),
  toggleShuffle: () => invoke<boolean>("toggle_shuffle"),
  toggleRepeat: () => invoke<number>("toggle_repeat"),
  getPlaybackState: () => invoke<PlaybackState>("get_playback_state"),
  getQueue: () => invoke<AudioTrack[]>("get_queue"),
  setPlaybackSpeed: (speed: number) => invoke<number>("set_playback_speed", { speed }),

  // Resumes
  saveAlbumResume: (albumPath: string, lastFilePath: string, positionMs: number) =>
    invoke<void>("save_album_resume", { albumPath, lastFilePath, positionMs }),
  getAlbumResume: (albumPath: string) =>
    invoke<AlbumResume | null>("get_album_resume", { albumPath }),
  getLatestAlbumResume: () =>
    invoke<AlbumResume | null>("get_latest_album_resume"),
  saveTrackResume: (
    filePath: string,
    albumPath: string,
    positionMs: number,
    durationMs: number,
    isCompleted: boolean,
  ) =>
    invoke<void>("save_track_resume", {
      filePath,
      albumPath,
      positionMs,
      durationMs,
      isCompleted,
    }),
  getTrackResume: (filePath: string) =>
    invoke<TrackResume | null>("get_track_resume", { filePath }),
  getAllTrackResumes: () => invoke<TrackResume[]>("get_all_track_resumes"),

  // Playlists
  getPlaylists: () => invoke<Playlist[]>("get_playlists"),
  createPlaylist: (name: string) => invoke<Playlist>("create_playlist", { name }),
  findMissingFiles: (paths: string[]) => invoke<string[]>("find_missing_files", { paths }),

  // Text size
  getFontScale: () => invoke<string>("get_font_scale"),
  saveFontScale: (fontScale: string) => invoke<string>("save_font_scale", { fontScale }),
  setZoom: (scale: number) => getCurrentWebview().setZoom(scale),

  // Bookmarks
  addBookmark: () => invoke<Bookmark>("add_bookmark"),
  getBookmarks: () => invoke<Bookmark[]>("get_bookmarks"),
  deleteBookmark: (filePath: string) => invoke<void>("delete_bookmark", { filePath }),
  deletePlaylist: (id: number) => invoke<void>("delete_playlist", { id }),
  addToPlaylist: (playlistId: number, track: AudioTrack) =>
    invoke<void>("add_to_playlist", { playlistId, track }),
  getPlaylistTracks: (playlistId: number) =>
    invoke<AudioTrack[]>("get_playlist_tracks", { playlistId }),
  removeFromPlaylist: (playlistId: number, filePath: string) =>
    invoke<void>("remove_from_playlist", { playlistId, filePath }),

  // Theme / Appearance
  getThemeSetting: () => invoke<string>("get_theme_setting"),
  getGuifThemePreset: () => invoke<string>("get_guif_theme_preset"),
  saveThemeSetting: (theme: string) =>
    invoke<string>("save_theme_setting", { theme }),
};
