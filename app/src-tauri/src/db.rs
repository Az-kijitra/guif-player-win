use std::path::PathBuf;
use std::sync::Mutex;
use rusqlite::{params, Connection, Result};
use crate::models::{AlbumResume, AudioTrack, Playlist, TrackResume};

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new() -> Result<Self> {
        let db_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("guif-player");
        
        let _ = std::fs::create_dir_all(&db_dir);
        let db_path = db_dir.join("guif_player.db");

        let conn = Connection::open(db_path)?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS album_resume (
                album_path TEXT PRIMARY KEY,
                last_file_path TEXT NOT NULL,
                position_ms INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS track_resume (
                file_path TEXT PRIMARY KEY,
                album_path TEXT NOT NULL,
                position_ms INTEGER NOT NULL,
                duration_ms INTEGER NOT NULL,
                is_completed INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS playlists (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS playlist_tracks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                playlist_id INTEGER NOT NULL,
                file_path TEXT NOT NULL,
                title TEXT NOT NULL,
                artist TEXT NOT NULL,
                album TEXT NOT NULL,
                duration_ms INTEGER NOT NULL,
                sort_order INTEGER NOT NULL,
                FOREIGN KEY (playlist_id) REFERENCES playlists(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            "#,
        )?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM app_settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO app_settings (key, value)
            VALUES (?1, ?2)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value
            "#,
            params![key, value],
        )?;
        Ok(())
    }

    pub fn save_album_resume(&self, album_path: &str, last_file_path: &str, position_ms: u64) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO album_resume (album_path, last_file_path, position_ms, updated_at)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(album_path) DO UPDATE SET
                last_file_path = excluded.last_file_path,
                position_ms = excluded.position_ms,
                updated_at = excluded.updated_at
            "#,
            params![album_path, last_file_path, position_ms, now],
        )?;
        Ok(())
    }

    pub fn get_album_resume(&self, album_path: &str) -> Result<Option<AlbumResume>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT album_path, last_file_path, position_ms, updated_at FROM album_resume WHERE album_path = ?1"
        )?;
        let mut rows = stmt.query(params![album_path])?;
        if let Some(row) = rows.next()? {
            Ok(Some(AlbumResume {
                album_path: row.get(0)?,
                last_file_path: row.get(1)?,
                position_ms: row.get(2)?,
                updated_at: row.get(3)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn get_latest_album_resume(&self) -> Result<Option<AlbumResume>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT album_path, last_file_path, position_ms, updated_at FROM album_resume ORDER BY updated_at DESC LIMIT 1"
        )?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            Ok(Some(AlbumResume {
                album_path: row.get(0)?,
                last_file_path: row.get(1)?,
                position_ms: row.get(2)?,
                updated_at: row.get(3)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn save_track_resume(
        &self,
        file_path: &str,
        album_path: &str,
        position_ms: u64,
        duration_ms: u64,
        is_completed: bool,
    ) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO track_resume (file_path, album_path, position_ms, duration_ms, is_completed, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(file_path) DO UPDATE SET
                album_path = excluded.album_path,
                position_ms = excluded.position_ms,
                duration_ms = excluded.duration_ms,
                is_completed = excluded.is_completed,
                updated_at = excluded.updated_at
            "#,
            params![file_path, album_path, position_ms, duration_ms, if is_completed { 1 } else { 0 }, now],
        )?;
        Ok(())
    }

    pub fn get_track_resume(&self, file_path: &str) -> Result<Option<TrackResume>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT file_path, album_path, position_ms, duration_ms, is_completed, updated_at FROM track_resume WHERE file_path = ?1"
        )?;
        let mut rows = stmt.query(params![file_path])?;
        if let Some(row) = rows.next()? {
            let is_comp_int: i32 = row.get(4)?;
            Ok(Some(TrackResume {
                file_path: row.get(0)?,
                album_path: row.get(1)?,
                position_ms: row.get(2)?,
                duration_ms: row.get(3)?,
                is_completed: is_comp_int != 0,
                updated_at: row.get(5)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn get_all_track_resumes(&self) -> Result<Vec<TrackResume>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT file_path, album_path, position_ms, duration_ms, is_completed, updated_at FROM track_resume"
        )?;
        let rows = stmt.query_map([], |row| {
            let is_comp_int: i32 = row.get(4)?;
            Ok(TrackResume {
                file_path: row.get(0)?,
                album_path: row.get(1)?,
                position_ms: row.get(2)?,
                duration_ms: row.get(3)?,
                is_completed: is_comp_int != 0,
                updated_at: row.get(5)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn get_playlists(&self) -> Result<Vec<Playlist>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT p.id, p.name, p.created_at, COUNT(pt.id) as track_count
            FROM playlists p
            LEFT JOIN playlist_tracks pt ON p.id = pt.playlist_id
            GROUP BY p.id
            ORDER BY p.name ASC
            "#
        )?;
        let rows = stmt.query_map([], |row| {
            let track_count: i64 = row.get(3)?;
            Ok(Playlist {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
                track_count: track_count as usize,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn create_playlist(&self, name: &str) -> Result<Playlist> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO playlists (name, created_at) VALUES (?1, ?2)",
            params![name, now],
        )?;
        let id = conn.last_insert_rowid();

        Ok(Playlist {
            id,
            name: name.to_string(),
            track_count: 0,
            created_at: now,
        })
    }

    pub fn delete_playlist(&self, playlist_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM playlist_tracks WHERE playlist_id = ?1", params![playlist_id])?;
        conn.execute("DELETE FROM playlists WHERE id = ?1", params![playlist_id])?;
        Ok(())
    }

    pub fn add_track_to_playlist(&self, playlist_id: i64, track: &AudioTrack) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let max_order: Option<i64> = conn
            .query_row(
                "SELECT MAX(sort_order) FROM playlist_tracks WHERE playlist_id = ?1",
                params![playlist_id],
                |r| r.get(0),
            )
            .unwrap_or(None);

        let next_order = max_order.unwrap_or(0) + 1;
        conn.execute(
            r#"
            INSERT INTO playlist_tracks (playlist_id, file_path, title, artist, album, duration_ms, sort_order)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                playlist_id,
                track.file_path,
                track.title,
                track.artist,
                track.album,
                track.duration_ms,
                next_order
            ],
        )?;
        Ok(())
    }

    pub fn get_playlist_tracks(&self, playlist_id: i64) -> Result<Vec<AudioTrack>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT file_path, title, artist, album, duration_ms
            FROM playlist_tracks
            WHERE playlist_id = ?1
            ORDER BY sort_order ASC
            "#
        )?;
        let rows = stmt.query_map(params![playlist_id], |row| {
            let file_path: String = row.get(0)?;
            let file_name = std::path::Path::new(&file_path)
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            Ok(AudioTrack {
                file_path,
                file_name,
                title: row.get(1)?,
                artist: row.get(2)?,
                album: row.get(3)?,
                duration_ms: row.get(4)?,
                track_number: None,
                has_artwork: false,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn remove_track_from_playlist(&self, playlist_id: i64, file_path: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM playlist_tracks WHERE playlist_id = ?1 AND file_path = ?2",
            params![playlist_id, file_path],
        )?;
        Ok(())
    }

    pub fn update_playlist_track_metadata(&self, track: &AudioTrack) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE playlist_tracks SET title = ?1, artist = ?2, album = ?3 WHERE file_path = ?4",
            params![track.title, track.artist, track.album, track.file_path],
        )?;
        Ok(())
    }
}
