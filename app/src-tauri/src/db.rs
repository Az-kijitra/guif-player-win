use std::path::{Path, PathBuf};
use std::sync::Mutex;
use rusqlite::{params, Connection, Result};
use crate::models::{AlbumResume, AudioTrack, Playlist, TrackResume};

// Same as the identifier in tauri.conf.json, so the uninstaller's "delete app data" covers it
const DATA_DIR_NAME: &str = "dev.guif.player";
// Up to 0.1.1 the database sat here, which is also the default install folder
const LEGACY_DATA_DIR_NAME: &str = "guif-player";
const DB_FILE_NAME: &str = "guif_player.db";

fn data_base_dir() -> PathBuf {
    // Lets tests run the real app without touching the user's data (debug builds only)
    #[cfg(debug_assertions)]
    if let Some(dir) = std::env::var_os("GUIF_PLAYER_DATA_DIR") {
        let dir = PathBuf::from(dir);
        eprintln!("[db] override dir: {}", dir.display());
        return dir;
    }
    dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."))
}

/// Finds the database under `base_dir`, moving one left in the legacy folder.
/// If the move fails (an old version still running, say), the legacy file stays in use
/// until the next start, so the data never looks lost behind a new empty database.
fn resolve_db_path(base_dir: &Path) -> PathBuf {
    let db_dir = base_dir.join(DATA_DIR_NAME);
    let db_path = db_dir.join(DB_FILE_NAME);
    let legacy_path = base_dir.join(LEGACY_DATA_DIR_NAME).join(DB_FILE_NAME);

    if db_path.exists() || !legacy_path.exists() {
        return db_path;
    }

    let moved = std::fs::create_dir_all(&db_dir).and_then(|_| std::fs::rename(&legacy_path, &db_path));
    match moved {
        Ok(()) => db_path,
        Err(e) => {
            eprintln!("Failed to move the database to {}: {}", db_path.display(), e);
            legacy_path
        }
    }
}

// One entry per schema version; PRAGMA user_version records how many have run.
// Never change an entry once it has shipped: add a new one.
// The first uses IF NOT EXISTS because databases from before versioning already have these tables.
const MIGRATIONS: &[&str] = &[
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
];

/// Brings the schema up to date. Each step commits together with its version number, so a failure
/// leaves the data as the last good step left it.
fn run_migrations(conn: &mut Connection, migrations: &[&str]) -> std::result::Result<(), String> {
    let describe = |e: rusqlite::Error| format!("データを更新できません: {}", e);

    let version: usize = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(describe)?;
    if version > migrations.len() {
        return Err("より新しい版の guif-player で作られたデータです。新しい版を使ってください".to_string());
    }

    for (index, migration) in migrations.iter().enumerate().skip(version) {
        let tx = conn.transaction().map_err(describe)?;
        tx.execute_batch(migration).map_err(describe)?;
        tx.pragma_update(None, "user_version", index + 1).map_err(describe)?;
        tx.commit().map_err(describe)?;
    }
    Ok(())
}

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new() -> std::result::Result<Self, String> {
        Self::open_at(&resolve_db_path(&data_base_dir()))
    }

    fn open_at(db_path: &Path) -> std::result::Result<Self, String> {
        if let Some(db_dir) = db_path.parent() {
            let _ = std::fs::create_dir_all(db_dir);
        }

        let mut conn = Connection::open(db_path)
            .map_err(|e| format!("{}: {}", db_path.display(), e))?;
        run_migrations(&mut conn, MIGRATIONS)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_legacy_db(base: &Path, playlist_name: &str) -> PathBuf {
        let path = base.join(LEGACY_DATA_DIR_NAME).join(DB_FILE_NAME);
        let db = Database::open_at(&path).unwrap();
        db.create_playlist(playlist_name).unwrap();
        path
    }

    #[test]
    fn a_fresh_install_uses_the_new_folder() {
        let base = tempfile::tempdir().unwrap();
        let path = resolve_db_path(base.path());
        assert_eq!(path, base.path().join(DATA_DIR_NAME).join(DB_FILE_NAME));
        assert!(!path.exists());
    }

    #[test]
    fn the_legacy_database_moves_with_its_data() {
        let base = tempfile::tempdir().unwrap();
        let legacy = write_legacy_db(base.path(), "kept");

        let path = resolve_db_path(base.path());

        assert_eq!(path, base.path().join(DATA_DIR_NAME).join(DB_FILE_NAME));
        assert!(!legacy.exists());
        let playlists = Database::open_at(&path).unwrap().get_playlists().unwrap();
        assert_eq!(playlists[0].name, "kept");
    }

    #[test]
    fn other_files_in_the_legacy_folder_are_left_alone() {
        let base = tempfile::tempdir().unwrap();
        let legacy = write_legacy_db(base.path(), "kept");
        let exe = legacy.with_file_name("guif-player.exe");
        fs::write(&exe, b"exe").unwrap();

        resolve_db_path(base.path());

        assert!(exe.exists());
    }

    #[test]
    fn an_existing_new_database_is_never_overwritten() {
        let base = tempfile::tempdir().unwrap();
        let legacy = write_legacy_db(base.path(), "old");
        let new_path = base.path().join(DATA_DIR_NAME).join(DB_FILE_NAME);
        Database::open_at(&new_path).unwrap().create_playlist("new").unwrap();

        assert_eq!(resolve_db_path(base.path()), new_path);

        assert!(legacy.exists());
        let playlists = Database::open_at(&new_path).unwrap().get_playlists().unwrap();
        assert_eq!(playlists[0].name, "new");
    }

    fn user_version(path: &Path) -> usize {
        Connection::open(path)
            .unwrap()
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap()
    }

    #[test]
    fn a_new_database_gets_the_latest_schema() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.db");

        let db = Database::open_at(&path).unwrap();
        db.create_playlist("works").unwrap();
        db.set_setting("theme", "nord").unwrap();
        drop(db);

        assert_eq!(user_version(&path), MIGRATIONS.len());
    }

    #[test]
    fn a_database_from_before_versioning_keeps_its_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.db");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(MIGRATIONS[0]).unwrap();
        conn.execute("INSERT INTO playlists (name, created_at) VALUES ('kept', 1)", [])
            .unwrap();
        drop(conn);
        assert_eq!(user_version(&path), 0);

        let db = Database::open_at(&path).unwrap();

        assert_eq!(db.get_playlists().unwrap()[0].name, "kept");
        drop(db);
        assert_eq!(user_version(&path), MIGRATIONS.len());
    }

    #[test]
    fn opening_twice_changes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.db");
        Database::open_at(&path).unwrap().create_playlist("kept").unwrap();

        let db = Database::open_at(&path).unwrap();

        assert_eq!(db.get_playlists().unwrap().len(), 1);
    }

    #[test]
    fn a_database_from_a_newer_version_is_refused_and_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.db");
        Database::open_at(&path).unwrap().create_playlist("kept").unwrap();
        let newer = MIGRATIONS.len() + 1;
        Connection::open(&path)
            .unwrap()
            .pragma_update(None, "user_version", newer)
            .unwrap();

        let message = Database::open_at(&path).err().unwrap();

        assert!(message.contains("新しい版"), "{message}");
        assert_eq!(user_version(&path), newer);
    }

    #[test]
    fn a_failing_migration_rolls_back_and_keeps_the_earlier_steps() {
        let dir = tempfile::tempdir().unwrap();
        let mut conn = Connection::open(dir.path().join("a.db")).unwrap();
        let steps = [
            "CREATE TABLE first (id INTEGER);",
            "CREATE TABLE second (id INTEGER); THIS IS NOT SQL;",
        ];

        assert!(run_migrations(&mut conn, &steps).is_err());

        let version: usize = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, 1);
        let count_tables = |name: &str| -> i64 {
            conn.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name = ?1",
                params![name],
                |row| row.get(0),
            )
            .unwrap()
        };
        assert_eq!(count_tables("first"), 1);
        assert_eq!(count_tables("second"), 0);
    }

    #[test]
    fn a_later_migration_runs_on_an_older_database() {
        let dir = tempfile::tempdir().unwrap();
        let mut conn = Connection::open(dir.path().join("a.db")).unwrap();
        run_migrations(&mut conn, &["CREATE TABLE first (id INTEGER);"]).unwrap();
        conn.execute("INSERT INTO first VALUES (1)", []).unwrap();

        run_migrations(
            &mut conn,
            &[
                "CREATE TABLE first (id INTEGER);",
                "ALTER TABLE first ADD COLUMN extra TEXT;",
            ],
        )
        .unwrap();

        let kept: i64 = conn.query_row("SELECT id FROM first", [], |r| r.get(0)).unwrap();
        assert_eq!(kept, 1);
    }

    #[cfg(windows)]
    #[test]
    fn a_locked_legacy_database_stays_in_use() {
        use std::os::windows::fs::OpenOptionsExt;

        let base = tempfile::tempdir().unwrap();
        let legacy = write_legacy_db(base.path(), "kept");
        let lock = fs::OpenOptions::new().read(true).share_mode(0).open(&legacy).unwrap();

        assert_eq!(resolve_db_path(base.path()), legacy);

        drop(lock);
        assert!(legacy.exists());
        // The next start moves it
        assert_eq!(
            resolve_db_path(base.path()),
            base.path().join(DATA_DIR_NAME).join(DB_FILE_NAME)
        );
    }
}
