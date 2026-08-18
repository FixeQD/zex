//! Persistent index of parsed applications, ts stored in SQLite

use super::discover::xdg_app_dirs;
use super::model::AppInfo;
use rusqlite::{Connection, OptionalExtension, params};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::debug;

/// Bump when the stored layout changes
pub const SCHEMA_VERSION: i64 = 1;

/// Modification times of the current XDG application directories
pub fn dir_mtimes() -> HashMap<PathBuf, SystemTime> {
    let mut mtimes = HashMap::new();
    for dir in xdg_app_dirs() {
        if let Ok(metadata) = fs::metadata(&dir) {
            if let Ok(mtime) = metadata.modified() {
                mtimes.insert(dir, mtime);
            }
        }
    }
    mtimes
}

/// An opened application index.
pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn load(path: &Path) -> rusqlite::Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let conn = match Connection::open(path) {
            Ok(conn) => conn,
            // Missing or unreadable index
            Err(_) => return Ok(None),
        };
        init(&conn)?;
        let version: i64 = conn
            .query_row("SELECT value FROM meta WHERE key = 'version'", [], |row| {
                row.get(0)
            })
            .optional()?
            .and_then(|value: String| value.parse().ok())
            .unwrap_or(0);
        if version != SCHEMA_VERSION {
            debug!("index schema mismatch (found {version}, want {SCHEMA_VERSION})");
            return Ok(None);
        }
        Ok(Some(Self { conn }))
    }

    /// Check whether the index still reflects the current sources
    pub fn fresh(&self, mtimes: &HashMap<PathBuf, SystemTime>) -> bool {
        let current = normalize(mtimes);
        let stored: HashMap<String, i64> = self
            .conn
            .prepare("SELECT dir_path, mtime FROM sources")
            .and_then(|mut stmt| {
                stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                    .map(|rows| rows.filter_map(|row| row.ok()).collect())
            })
            .unwrap_or_default();
        if stored != current {
            debug!("index invalidated: directory mtimes changed");
            return false;
        }

        let files: Vec<(String, i64)> = self
            .conn
            .prepare("SELECT source, mtime FROM apps")
            .and_then(|mut stmt| {
                stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                    .map(|rows| rows.filter_map(|row| row.ok()).collect())
            })
            .unwrap_or_default();
        for (source, stored_mtime) in files {
            let actual = fs::metadata(&source)
                .and_then(|metadata| metadata.modified())
                .map(to_millis)
                .ok();
            if actual != Some(stored_mtime) {
                debug!("index invalidated: {source} was modified");
                return false;
            }
        }
        true
    }

    /// Read all indexed applications
    pub fn snapshot(&self) -> rusqlite::Result<Vec<AppInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, command, icon_name, icon_file, summary, tags, wants_terminal, source \
             FROM apps",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, bool>(7)?,
                row.get::<_, String>(8)?,
            ))
        })?;
        let mut apps = Vec::new();
        for row in rows {
            let (id, title, command, icon_name, icon_file, summary, tags, wants_terminal, source) =
                row?;
            apps.push(AppInfo {
                id,
                title,
                command,
                icon_name,
                icon_file: icon_file.map(PathBuf::from),
                summary,
                tags: serde_json::from_str(&tags).unwrap_or_default(),
                wants_terminal,
                source: PathBuf::from(source),
            });
        }
        Ok(apps)
    }

    /// Persist a fresh scan replacing any previous content
    pub fn write(
        path: &Path,
        apps: &[AppInfo],
        mtimes: &HashMap<PathBuf, SystemTime>,
    ) -> rusqlite::Result<()> {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let conn = Connection::open(path)?;
        init(&conn)?;
        let tx = conn.unchecked_transaction()?;
        tx.execute("DELETE FROM apps", [])?;
        tx.execute("DELETE FROM sources", [])?;
        for (dir, mtime) in normalize(mtimes) {
            tx.execute(
                "INSERT INTO sources (dir_path, mtime) VALUES (?1, ?2)",
                params![dir, mtime],
            )?;
        }
        for app in apps {
            let mtime = fs::metadata(&app.source)
                .and_then(|metadata| metadata.modified())
                .map(to_millis)
                .unwrap_or(0);
            tx.execute(
                "INSERT INTO apps \
                 (id, title, command, icon_name, icon_file, summary, tags, wants_terminal, source, mtime) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    app.id,
                    app.title,
                    app.command,
                    app.icon_name,
                    app.icon_file.as_ref().map(|path| path.to_string_lossy().to_string()),
                    app.summary,
                    serde_json::to_string(&app.tags).unwrap_or_default(),
                    app.wants_terminal,
                    app.source.to_string_lossy().to_string(),
                    mtime,
                ],
            )?;
        }
        tx.commit()
    }
}

fn init(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS apps (
             id TEXT PRIMARY KEY,
             title TEXT NOT NULL,
             command TEXT NOT NULL,
             icon_name TEXT,
             icon_file TEXT,
             summary TEXT,
             tags TEXT NOT NULL,
             wants_terminal INTEGER NOT NULL,
             source TEXT NOT NULL,
             mtime INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS sources (
             dir_path TEXT PRIMARY KEY,
             mtime INTEGER NOT NULL
         );",
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO meta (key, value) VALUES ('version', ?1)",
        params![SCHEMA_VERSION],
    )?;
    Ok(())
}

fn to_millis(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

fn normalize(mtimes: &HashMap<PathBuf, SystemTime>) -> HashMap<String, i64> {
    mtimes
        .iter()
        .map(|(dir, time)| (dir.to_string_lossy().to_string(), to_millis(*time)))
        .collect()
}
