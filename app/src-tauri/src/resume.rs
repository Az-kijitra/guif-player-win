//! When and where playback positions are saved. The rules match the Android version (ResumePolicy).

use std::path::Path;

use crate::db::Database;
use crate::models::PlaybackStateDto;

/// A track played to within this much of its end counts as finished
pub const COMPLETED_MARGIN_MS: u64 = 3_000;
/// Positions this early are not worth coming back to
pub const MIN_SAVED_POSITION_MS: u64 = 1_000;

pub fn is_track_completed(position_ms: u64, duration_ms: u64) -> bool {
    duration_ms > 0 && position_ms + COMPLETED_MARGIN_MS >= duration_ms
}

/// The folder a track belongs to, which is its album.
pub fn album_of(file_path: &str) -> String {
    Path::new(file_path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Saves where the current track is, for the track and for its album, in one go.
pub fn record_position(state: &PlaybackStateDto, db: &Database) {
    let Some(track) = &state.current_track else {
        return;
    };
    if state.position_ms < MIN_SAVED_POSITION_MS || state.duration_ms == 0 {
        return;
    }
    let _ = db.save_resume(
        &album_of(&track.file_path),
        &track.file_path,
        state.position_ms,
        state.duration_ms,
        is_track_completed(state.position_ms, state.duration_ms),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::AudioTrack;

    fn state(file_path: &str, position_ms: u64, duration_ms: u64) -> PlaybackStateDto {
        PlaybackStateDto {
            current_track: Some(AudioTrack {
                file_path: file_path.to_string(),
                file_name: "a.mp3".to_string(),
                title: "A".to_string(),
                artist: "Artist".to_string(),
                album: "Album".to_string(),
                duration_ms,
                track_number: None,
                has_artwork: false,
            }),
            is_playing: true,
            position_ms,
            duration_ms,
            volume: 1.0,
            shuffle_mode: false,
            repeat_mode: 0,
            queue_version: 0,
        }
    }

    fn database() -> (tempfile::TempDir, Database) {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open_at(&dir.path().join("a.db")).unwrap();
        (dir, db)
    }

    #[test]
    fn a_track_is_completed_within_the_last_three_seconds() {
        assert!(is_track_completed(57_000, 60_000));
        assert!(is_track_completed(60_000, 60_000));
        assert!(!is_track_completed(56_999, 60_000));
        assert!(!is_track_completed(0, 0));
    }

    #[test]
    fn leaving_a_track_records_its_position_under_its_own_album() {
        let (_dir, db) = database();
        let path = Path::new("C:\\Music\\Album A\\01.mp3")
            .to_string_lossy()
            .to_string();

        record_position(&state(&path, 42_000, 200_000), &db);

        let track = db.get_track_resume(&path).unwrap().unwrap();
        assert_eq!(track.position_ms, 42_000);
        assert!(!track.is_completed);
        let album = db.get_album_resume(&album_of(&path)).unwrap().unwrap();
        assert_eq!(album.last_file_path, path);
        assert_eq!(album.position_ms, 42_000);
    }

    #[test]
    fn a_track_left_near_its_end_counts_as_finished() {
        let (_dir, db) = database();

        record_position(&state("C:\\A\\01.mp3", 198_000, 200_000), &db);

        assert!(
            db.get_track_resume("C:\\A\\01.mp3")
                .unwrap()
                .unwrap()
                .is_completed
        );
    }

    #[test]
    fn the_first_second_of_a_track_is_not_recorded() {
        let (_dir, db) = database();

        record_position(&state("C:\\A\\01.mp3", 999, 200_000), &db);
        record_position(&state("C:\\A\\02.mp3", 5_000, 0), &db);

        assert!(db.get_track_resume("C:\\A\\01.mp3").unwrap().is_none());
        assert!(db.get_track_resume("C:\\A\\02.mp3").unwrap().is_none());
        assert!(db.get_album_resume("C:\\A").unwrap().is_none());
    }
}
