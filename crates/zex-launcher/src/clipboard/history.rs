use crate::clipboard::{Content, Entry};
use crate::engine::Matcher;
use anyhow::{Context, Result};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tracing::warn;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub limit: usize,
    pub keep_passwords: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            limit: 500,
            keep_passwords: false,
        }
    }
}

#[derive(Debug)]
pub struct History {
    held: VecDeque<Entry>,
    limit: usize,
    db: Option<rusqlite::Connection>,
}

impl History {
    pub fn default_path() -> Option<PathBuf> {
        dirs::cache_dir().map(|dir| dir.join("zex").join("clipboard.sqlite"))
    }

    /// Pass `None` for a purely in-memory session.
    pub fn open(path: Option<&Path>, settings: Settings) -> Result<Self> {
        let (db, held) = match path {
            Some(file) => {
                if let Some(parent) = file.parent() {
                    std::fs::create_dir_all(parent).context("create clipboard cache directory")?;
                }
                let mut conn =
                    rusqlite::Connection::open(file).context("open clipboard database")?;
                conn.execute_batch(
                    "CREATE TABLE IF NOT EXISTS clips (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        kind TEXT NOT NULL,
                        body TEXT,
                        width INTEGER,
                        height INTEGER,
                        data BLOB,
                        plain TEXT,
                        html TEXT,
                        paths TEXT,
                        stamp INTEGER NOT NULL
                    );",
                )
                .context("create clipboard schema")?;
                let loaded = load_rows(&mut conn, settings.limit)?;
                (Some(conn), loaded)
            }
            None => (None, VecDeque::new()),
        };
        Ok(Self {
            held,
            limit: settings.limit,
            db,
        })
    }

    /// Remember a capture. Consecutive duplicates and entries over `limit`are dropped
    /// Returns whether the entry was actually kept
    pub fn push(&mut self, entry: Entry) -> bool {
        if self
            .held
            .front()
            .is_some_and(|top| top.content.signature() == entry.content.signature())
        {
            return false;
        }
        if let Some(conn) = self.db.as_mut() {
            if let Err(e) = insert_row(conn, &entry) {
                warn!("clipboard row not persisted: {e}");
            }
        }
        self.held.push_front(entry);
        while self.held.len() > self.limit {
            self.held.pop_back();
        }
        true
    }

    /// Entries newest-first, optionally narrowed by fuzzy text match
    pub fn browse(&self, matcher: &Matcher, query: &str) -> Vec<Entry> {
        let query = query.trim();
        if query.is_empty() {
            return self.held.iter().cloned().collect();
        }
        let mut scored: Vec<(Entry, i64)> = self
            .held
            .iter()
            .filter_map(|entry| {
                matcher
                    .score(&entry.body(), query)
                    .map(|score| (entry.clone(), score))
            })
            .collect();
        scored.sort_by(|a, b| b.1.cmp(&a.1));
        scored.into_iter().map(|(entry, _)| entry).collect()
    }

    pub fn len(&self) -> usize {
        self.held.len()
    }

    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }

    /// Forget every capture, including the mirrored rows.
    pub fn clear(&mut self) {
        self.held.clear();
        if let Some(conn) = self.db.as_mut() {
            let _ = conn.execute("DELETE FROM clips", []);
        }
    }
}

fn load_rows(conn: &mut rusqlite::Connection, limit: usize) -> Result<VecDeque<Entry>> {
    let mut stmt = conn
        .prepare(
            "SELECT kind, body, width, height, data, plain, html, paths, stamp
             FROM clips ORDER BY stamp DESC, id DESC",
        )
        .context("prepare clipboard read")?;
    let mut held = VecDeque::new();
    let rows = stmt
        .query_map([], |row| {
            let kind: String = row.get("kind")?;
            let stamp_i: i64 = row.get("stamp")?;
            let content = match kind.as_str() {
                "text" => Content::Text(row.get("body")?),
                "image" => Content::Image {
                    width: row.get::<_, i64>("width")? as usize,
                    height: row.get::<_, i64>("height")? as usize,
                    rgba: row.get("data")?,
                },
                "files" => Content::Files(
                    serde_json::from_str(&row.get::<_, String>("paths")?).unwrap_or_default(),
                ),
                "snippet" => Content::Snippet {
                    plain: row.get("plain")?,
                    html: row.get("html")?,
                },
                _ => {
                    return Err(rusqlite::Error::InvalidColumnName(
                        "unknown clip kind".into(),
                    ));
                }
            };
            let stamp = SystemTime::UNIX_EPOCH
                .checked_add(std::time::Duration::from_secs(stamp_i.max(0) as u64))
                .unwrap_or(SystemTime::UNIX_EPOCH);
            Ok(Entry { content, stamp })
        })
        .context("read clipboard rows")?;
    for row in rows.filter_map(Result::ok) {
        if held.len() >= limit {
            break;
        }
        held.push_back(row);
    }
    Ok(held)
}

fn insert_row(conn: &mut rusqlite::Connection, entry: &Entry) -> Result<()> {
    let stamp = entry
        .stamp
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    match &entry.content {
        Content::Text(text) => {
            conn.execute(
                "INSERT INTO clips (kind, body, stamp) VALUES ('text', ?1, ?2)",
                rusqlite::params![text, stamp],
            )?;
        }
        Content::Image {
            width,
            height,
            rgba,
        } => {
            conn.execute(
                "INSERT INTO clips (kind, width, height, data, stamp) VALUES ('image', ?1, ?2, ?3, ?4)",
                rusqlite::params![*width as i64, *height as i64, rgba, stamp],
            )?;
        }
        Content::Files(paths) => {
            let payload = serde_json::to_string(paths)?;
            conn.execute(
                "INSERT INTO clips (kind, paths, stamp) VALUES ('files', ?1, ?2)",
                rusqlite::params![payload, stamp],
            )?;
        }
        Content::Snippet { plain, html } => {
            conn.execute(
                "INSERT INTO clips (kind, plain, html, stamp) VALUES ('snippet', ?1, ?2, ?3)",
                rusqlite::params![plain, html, stamp],
            )?;
        }
    }
    Ok(())
}
