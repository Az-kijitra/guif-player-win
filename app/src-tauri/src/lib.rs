pub mod db;
pub mod metadata;
pub mod models;
pub mod player;
pub mod queue;
pub mod scanner;
#[cfg(test)]
mod test_audio;

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tauri::{Emitter, Manager, State};

use crate::db::Database;
use crate::models::{
    AlbumResume, AlbumSummary, AudioTrack, FolderContent, PlaybackStateDto, Playlist,
    TagUpdateRequest, TagUpdateResult, TrackResume,
};
use crate::player::{AudioPlayer, SharedPlayer};
use crate::queue::REPEAT_ONE;

pub struct AppState {
    pub player: SharedPlayer,
    pub db: Arc<Database>,
}

// ----------------------------------------------------
// Folder & Albums Commands
// ----------------------------------------------------

#[tauri::command]
fn get_default_path(state: State<'_, AppState>) -> String {
    if let Ok(Some(resume)) = state.db.get_latest_album_resume() {
        let p = Path::new(&resume.album_path);
        if p.is_dir() {
            return resume.album_path;
        }
    }
    scanner::get_default_music_dir()
}

#[tauri::command]
fn list_drives() -> Vec<String> {
    scanner::list_drives()
}

#[tauri::command]
fn list_folder(path: String) -> FolderContent {
    scanner::list_folder(&path)
}

#[tauri::command]
fn list_albums(root_path: String) -> Vec<AlbumSummary> {
    scanner::list_albums(&root_path)
}

// ----------------------------------------------------
// Artwork Command
// ----------------------------------------------------

#[tauri::command]
fn get_artwork(file_path: String) -> Option<String> {
    metadata::extract_artwork(Path::new(&file_path))
}

#[tauri::command]
fn update_tracks_metadata(
    state: State<'_, AppState>,
    request: TagUpdateRequest,
) -> Result<TagUpdateResult, String> {
    // 1. Temporarily release file handle if the currently playing track is being modified
    let suspended = {
        let mut player = state.player.lock().unwrap();
        player.suspend_if_playing(&request.file_paths)
    };

    // 2. Perform tag writing on disk
    let result = metadata::update_tags(&request);

    // 3. Restore playback & update in-memory queue and SQLite playlists
    match result {
        Ok(outcome) => {
            {
                let mut player = state.player.lock().unwrap();
                player.restore_after_tag_update(&outcome.updated, suspended);
            }
            for track in &outcome.updated {
                let _ = state.db.update_playlist_track_metadata(track);
            }
            Ok(outcome)
        }
        Err(err) => {
            // Even on failure, restore playback if it was suspended
            if let Some(s) = suspended {
                let mut player = state.player.lock().unwrap();
                player.restore_after_tag_update(&[], Some(s));
            }
            Err(err)
        }
    }
}

// ----------------------------------------------------
// Playback Commands
// ----------------------------------------------------

fn record_current_track_resume(player: &AudioPlayer, db: &Database) {
    let dto = player.get_state_dto();
    if let Some(cur) = dto.current_track {
        if let Some(parent) = Path::new(&cur.file_path).parent() {
            let album_path = parent.to_string_lossy().to_string();
            let _ = db.save_album_resume(&album_path, &cur.file_path, dto.position_ms);
        }
    }
}

#[tauri::command]
fn play_track(
    state: State<'_, AppState>,
    track: AudioTrack,
    initial_pos_ms: Option<u64>,
) -> Result<(), String> {
    let mut player = state.player.lock().unwrap();
    player.play_track(track, initial_pos_ms.unwrap_or(0))?;
    record_current_track_resume(&player, &state.db);
    Ok(())
}

#[tauri::command]
fn play_queue(
    state: State<'_, AppState>,
    queue: Vec<AudioTrack>,
    start_index: usize,
    initial_pos_ms: Option<u64>,
) -> Result<(), String> {
    let mut player = state.player.lock().unwrap();
    player.set_queue(queue, start_index, initial_pos_ms.unwrap_or(0))?;
    record_current_track_resume(&player, &state.db);
    Ok(())
}

#[tauri::command]
fn toggle_play_pause(state: State<'_, AppState>) -> Result<(), String> {
    let mut player = state.player.lock().unwrap();
    player.toggle_play_pause()?;
    record_current_track_resume(&player, &state.db);
    Ok(())
}

#[tauri::command]
fn pause(state: State<'_, AppState>) {
    let mut player = state.player.lock().unwrap();
    player.pause();
    record_current_track_resume(&player, &state.db);
}

#[tauri::command]
fn resume(state: State<'_, AppState>) {
    let mut player = state.player.lock().unwrap();
    player.resume();
}

#[tauri::command]
fn seek(state: State<'_, AppState>, position_ms: u64) {
    let mut player = state.player.lock().unwrap();
    player.seek(position_ms);
    record_current_track_resume(&player, &state.db);
}

#[tauri::command]
fn set_volume(state: State<'_, AppState>, volume: f32) {
    let mut player = state.player.lock().unwrap();
    player.set_volume(volume);
}

#[tauri::command]
fn play_next(state: State<'_, AppState>) -> Result<(), String> {
    let mut player = state.player.lock().unwrap();
    player.play_next()?;
    record_current_track_resume(&player, &state.db);
    Ok(())
}

#[tauri::command]
fn play_previous(state: State<'_, AppState>) -> Result<(), String> {
    let mut player = state.player.lock().unwrap();
    player.play_previous()?;
    record_current_track_resume(&player, &state.db);
    Ok(())
}

#[tauri::command]
fn toggle_shuffle(state: State<'_, AppState>) -> bool {
    let mut player = state.player.lock().unwrap();
    player.toggle_shuffle();
    player.get_state_dto().shuffle_mode
}

#[tauri::command]
fn toggle_repeat(state: State<'_, AppState>) -> u8 {
    let mut player = state.player.lock().unwrap();
    player.toggle_repeat();
    player.get_state_dto().repeat_mode
}

#[tauri::command]
fn get_playback_state(state: State<'_, AppState>) -> PlaybackStateDto {
    let player = state.player.lock().unwrap();
    player.get_state_dto()
}

// ----------------------------------------------------
// Database / Resume Commands
// ----------------------------------------------------

#[tauri::command]
fn save_album_resume(
    state: State<'_, AppState>,
    album_path: String,
    last_file_path: String,
    position_ms: u64,
) -> Result<(), String> {
    state
        .db
        .save_album_resume(&album_path, &last_file_path, position_ms)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_album_resume(
    state: State<'_, AppState>,
    album_path: String,
) -> Result<Option<AlbumResume>, String> {
    state
        .db
        .get_album_resume(&album_path)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_latest_album_resume(state: State<'_, AppState>) -> Result<Option<AlbumResume>, String> {
    state
        .db
        .get_latest_album_resume()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn save_track_resume(
    state: State<'_, AppState>,
    file_path: String,
    album_path: String,
    position_ms: u64,
    duration_ms: u64,
    is_completed: bool,
) -> Result<(), String> {
    state
        .db
        .save_track_resume(&file_path, &album_path, position_ms, duration_ms, is_completed)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_track_resume(
    state: State<'_, AppState>,
    file_path: String,
) -> Result<Option<TrackResume>, String> {
    state
        .db
        .get_track_resume(&file_path)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_all_track_resumes(state: State<'_, AppState>) -> Result<Vec<TrackResume>, String> {
    state.db.get_all_track_resumes().map_err(|e| e.to_string())
}

// ----------------------------------------------------
// Playlists Commands
// ----------------------------------------------------

#[tauri::command]
fn get_playlists(state: State<'_, AppState>) -> Result<Vec<Playlist>, String> {
    state.db.get_playlists().map_err(|e| e.to_string())
}

#[tauri::command]
fn create_playlist(state: State<'_, AppState>, name: String) -> Result<Playlist, String> {
    state.db.create_playlist(&name).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_playlist(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    state.db.delete_playlist(id).map_err(|e| e.to_string())
}

#[tauri::command]
fn add_to_playlist(
    state: State<'_, AppState>,
    playlist_id: i64,
    track: AudioTrack,
) -> Result<(), String> {
    state
        .db
        .add_track_to_playlist(playlist_id, &track)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_playlist_tracks(
    state: State<'_, AppState>,
    playlist_id: i64,
) -> Result<Vec<AudioTrack>, String> {
    state
        .db
        .get_playlist_tracks(playlist_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_from_playlist(
    state: State<'_, AppState>,
    playlist_id: i64,
    file_path: String,
) -> Result<(), String> {
    state
        .db
        .remove_track_from_playlist(playlist_id, &file_path)
        .map_err(|e| e.to_string())
}

// ----------------------------------------------------
// Theme & Appearance Commands (guif aligned)
// ----------------------------------------------------

fn normalize_preset_name(name: &str) -> String {
    let normalized = name
        .trim()
        .to_ascii_lowercase()
        .replace(['-', '_', ' '], "");
    match normalized.as_str() {
        "default" => "default".to_string(),
        "nord" => "nord".to_string(),
        "monokai" => "monokai".to_string(),
        "catppuccin" | "catppuccinmocha" => "catppuccin".to_string(),
        "solarized" | "solarizeddark" => "solarized".to_string(),
        "light" => "light".to_string(),
        _ => "nord".to_string(),
    }
}

fn read_guif_config_preset() -> Option<String> {
    let config_dir = dirs::config_dir()?;
    let config_path = config_dir.join("dev.guif.app").join("config.toml");
    let content = std::fs::read_to_string(config_path).ok()?;
    let mut in_theme_section = false;
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_theme_section = line == "[theme]";
            continue;
        }
        if in_theme_section {
            if let Some((k, v)) = line.split_once('=') {
                let key = k.trim();
                if matches!(key, "preset" | "name" | "theme") {
                    let val = v
                        .split('#')
                        .next()
                        .unwrap_or("")
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'');
                    if !val.is_empty() {
                        return Some(normalize_preset_name(val));
                    }
                }
            }
        }
    }
    None
}

#[tauri::command]
fn get_theme_setting(state: State<'_, AppState>) -> String {
    if let Ok(Some(saved)) = state.db.get_setting("theme") {
        if saved == "guif" {
            return "guif".to_string();
        }
        return normalize_preset_name(&saved);
    }
    if read_guif_config_preset().is_some() {
        return "guif".to_string();
    }
    "nord".to_string()
}

#[tauri::command]
fn get_guif_theme_preset() -> String {
    read_guif_config_preset().unwrap_or_else(|| "nord".to_string())
}

#[tauri::command]
fn save_theme_setting(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
    theme: String,
) -> Result<String, String> {
    let saved_value = if theme == "guif" {
        "guif".to_string()
    } else {
        normalize_preset_name(&theme)
    };
    state
        .db
        .set_setting("theme", &saved_value)
        .map_err(|e| e.to_string())?;

    let resolved = if saved_value == "guif" {
        read_guif_config_preset().unwrap_or_else(|| "nord".to_string())
    } else {
        saved_value
    };

    let win_theme = if resolved == "light" {
        tauri::Theme::Light
    } else {
        tauri::Theme::Dark
    };
    let _ = window.set_theme(Some(win_theme));

    Ok(resolved)
}

// ----------------------------------------------------
// App Entrypoint
// ----------------------------------------------------

// The window does not exist yet at this point, and release builds have no console
#[cfg(windows)]
fn show_startup_error(message: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

    let wide = |text: &str| -> Vec<u16> { text.encode_utf16().chain(std::iter::once(0)).collect() };
    let (text, title) = (wide(message), wide("guif-player"));
    unsafe {
        MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), MB_OK | MB_ICONERROR);
    }
}

#[cfg(not(windows))]
fn show_startup_error(message: &str) {
    eprintln!("{}", message);
}

pub fn run() {
    let player = AudioPlayer::new();
    let db = match Database::new() {
        Ok(d) => Arc::new(d),
        Err(e) => {
            show_startup_error(&format!("データを開けないため、起動できません。\n\n{}", e));
            return;
        }
    };

    let player_clone = Arc::clone(&player);
    let db_clone = Arc::clone(&db);

    tauri::Builder::default()
        .setup(move |app| {
            if let Some(window) = app.get_webview_window("main") {
                if let Some(icon) = app.default_window_icon().cloned() {
                    let _ = window.set_icon(icon);
                }
                let saved_theme = db_clone
                    .get_setting("theme")
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| "guif".to_string());
                let resolved_theme = if saved_theme == "guif" {
                    read_guif_config_preset().unwrap_or_else(|| "nord".to_string())
                } else {
                    normalize_preset_name(&saved_theme)
                };
                let win_theme = if resolved_theme == "light" {
                    tauri::Theme::Light
                } else {
                    tauri::Theme::Dark
                };
                let _ = window.set_theme(Some(win_theme));
            }

            let app_handle = app.handle().clone();
            let thread_player = Arc::clone(&player_clone);
            let thread_db = Arc::clone(&db_clone);

            // Auto-advance & Periodic Save Background Loop
            std::thread::spawn(move || loop {
                std::thread::sleep(Duration::from_millis(400));

                let mut p = thread_player.lock().unwrap();
                let state_dto = p.get_state_dto();

                // Check track end
                if p.check_track_end() {
                    if state_dto.repeat_mode != REPEAT_ONE {
                        // Mark current completed in DB
                        if let Some(cur) = &state_dto.current_track {
                            if let Some(parent) = Path::new(&cur.file_path).parent() {
                                let _ = thread_db.save_track_resume(
                                    &cur.file_path,
                                    &parent.to_string_lossy(),
                                    cur.duration_ms,
                                    cur.duration_ms,
                                    true,
                                );
                            }
                        }
                    }
                    if let Err(e) = p.handle_track_end() {
                        let _ = app_handle.emit("playback-error", e);
                    }
                    record_current_track_resume(&p, &thread_db);
                    let new_state = p.get_state_dto();
                    let _ = app_handle.emit("playback-state-changed", new_state);
                } else if state_dto.is_playing {
                    // Periodic resume position save every 3 seconds
                    if state_dto.position_ms > 1000 {
                        if let Some(cur) = &state_dto.current_track {
                            if let Some(parent) = Path::new(&cur.file_path).parent() {
                                let album_path = parent.to_string_lossy().to_string();
                                let is_completed = state_dto.duration_ms > 0
                                    && state_dto.position_ms + 3000 >= state_dto.duration_ms;
                                let _ = thread_db.save_album_resume(
                                    &album_path,
                                    &cur.file_path,
                                    state_dto.position_ms,
                                );
                                let _ = thread_db.save_track_resume(
                                    &cur.file_path,
                                    &album_path,
                                    state_dto.position_ms,
                                    state_dto.duration_ms,
                                    is_completed,
                                );
                            }
                        }
                    }
                }
            });

            Ok(())
        })
        .manage(AppState {
            player: Arc::clone(&player),
            db: Arc::clone(&db),
        })
        .invoke_handler(tauri::generate_handler![
            get_default_path,
            list_drives,
            list_folder,
            list_albums,
            get_artwork,
            update_tracks_metadata,
            play_track,
            play_queue,
            toggle_play_pause,
            pause,
            resume,
            seek,
            set_volume,
            play_next,
            play_previous,
            toggle_shuffle,
            toggle_repeat,
            get_playback_state,
            save_album_resume,
            get_album_resume,
            get_latest_album_resume,
            save_track_resume,
            get_track_resume,
            get_all_track_resumes,
            get_playlists,
            create_playlist,
            delete_playlist,
            add_to_playlist,
            get_playlist_tracks,
            remove_from_playlist,
            get_theme_setting,
            get_guif_theme_preset,
            save_theme_setting,
        ])
        .run(tauri::generate_context!())
        .expect("error while running guif-player");
}
