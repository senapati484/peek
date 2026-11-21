//! SQLite persistence for Peek.
//!
//! One database per profile. See `docs/schema.md` for the schema and the
//! reasoning behind each table.

use std::path::Path;

use peek_ipc::TabId;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: i64 = 1;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("stored value for {0} is not valid JSON: {1}")]
    Json(String, String),
}

pub type Result<T> = std::result::Result<T, StoreError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub url: String,
    pub title: String,
    pub visit_count: i64,
    pub last_visit: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredTab {
    pub id: TabId,
    pub url: String,
    pub title: String,
    pub favicon: Option<String>,
    pub scroll_y: f64,
    pub position: i64,
    pub last_active: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bookmark {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub created_at: i64,
    pub position: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Session {
    pub active_tab: Option<TabId>,
    pub window_layout: String,
    pub created_at: i64,
}

pub struct Store {
    conn: Connection,
}

impl Store {
    /// Open (and migrate) the database at `path`, creating parent directories.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        Self::from_connection(conn)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(conn: Connection) -> Result<Self> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let mut store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&mut self) -> Result<()> {
        let version: i64 = self
            .conn
            .pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version > SCHEMA_VERSION {
            return Err(StoreError::Json(
                "user_version".into(),
                format!("database is newer than this build ({version} > {SCHEMA_VERSION})"),
            ));
        }
        if version < 1 {
            self.conn.execute_batch(MIGRATION_V1)?;
            self.conn
                .pragma_update(None, "user_version", SCHEMA_VERSION)?;
        }
        Ok(())
    }

    // -- history ---------------------------------------------------------

    /// Record a visit. Revisiting a URL bumps `visit_count` and `last_visit`
    /// and refreshes the title when a better one is available.
    pub fn record_visit(&self, url: &str, title: &str, now: i64) -> Result<()> {
        self.conn.execute(
            "INSERT INTO history (url, title, visit_count, last_visit)
             VALUES (?1, ?2, 1, ?3)
             ON CONFLICT(url) DO UPDATE SET
               visit_count = visit_count + 1,
               last_visit = excluded.last_visit,
               title = CASE WHEN excluded.title <> '' THEN excluded.title ELSE history.title END",
            params![url, title, now],
        )?;
        Ok(())
    }

    pub fn history_for(&self, url: &str) -> Result<Option<HistoryEntry>> {
        Ok(self
            .conn
            .query_row(
                "SELECT url, title, visit_count, last_visit FROM history WHERE url = ?1",
                params![url],
                map_history,
            )
            .optional()?)
    }

    /// Full-text search over history title and URL. `query` is sanitized into
    /// quoted prefix terms so FTS5 operators typed by a user cannot error.
    pub fn search_history(&self, query: &str, limit: usize) -> Result<Vec<HistoryEntry>> {
        let Some(match_query) = fts_prefix_query(query) else {
            return Ok(Vec::new());
        };
        let mut stmt = self.conn.prepare(
            "SELECT h.url, h.title, h.visit_count, h.last_visit
             FROM history_fts f
             JOIN history h ON h.id = f.rowid
             WHERE history_fts MATCH ?1
             ORDER BY h.visit_count DESC, h.last_visit DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![match_query, limit as i64], map_history)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn recent_history(&self, limit: usize) -> Result<Vec<HistoryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT url, title, visit_count, last_visit FROM history
             ORDER BY last_visit DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], map_history)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn delete_history_url(&self, url: &str) -> Result<usize> {
        Ok(self
            .conn
            .execute("DELETE FROM history WHERE url = ?1", params![url])?)
    }

    pub fn clear_history(&self) -> Result<()> {
        self.conn.execute("DELETE FROM history", [])?;
        Ok(())
    }

    // -- session tabs ----------------------------------------------------

    /// Replace the saved session with `tabs`. Called on quit and periodically.
    pub fn save_tabs(&mut self, tabs: &[StoredTab]) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM tabs", [])?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO tabs (id, url, title, favicon, scroll_y, position, last_active)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )?;
            for tab in tabs {
                stmt.execute(params![
                    tab.id as i64,
                    tab.url,
                    tab.title,
                    tab.favicon,
                    tab.scroll_y,
                    tab.position,
                    tab.last_active
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn load_tabs(&self) -> Result<Vec<StoredTab>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, url, title, favicon, scroll_y, position, last_active
             FROM tabs ORDER BY position ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(StoredTab {
                id: row.get::<_, i64>(0)? as TabId,
                url: row.get(1)?,
                title: row.get(2)?,
                favicon: row.get(3)?,
                scroll_y: row.get(4)?,
                position: row.get(5)?,
                last_active: row.get(6)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn save_session(&self, session: &Session) -> Result<()> {
        self.conn.execute(
            "INSERT INTO sessions (id, created_at, active_tab, window_layout)
             VALUES (1, ?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET
               created_at = excluded.created_at,
               active_tab = excluded.active_tab,
               window_layout = excluded.window_layout",
            params![
                session.created_at,
                session.active_tab.map(|id| id as i64),
                session.window_layout
            ],
        )?;
        Ok(())
    }

    pub fn load_session(&self) -> Result<Option<Session>> {
        Ok(self
            .conn
            .query_row(
                "SELECT active_tab, window_layout, created_at FROM sessions WHERE id = 1",
                [],
                |row| {
                    Ok(Session {
                        active_tab: row.get::<_, Option<i64>>(0)?.map(|id| id as TabId),
                        window_layout: row.get(1)?,
                        created_at: row.get(2)?,
                    })
                },
            )
            .optional()?)
    }

    // -- bookmarks -------------------------------------------------------

    pub fn add_bookmark(&self, url: &str, title: &str, now: i64) -> Result<i64> {
        let position: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM bookmarks",
            [],
            |row| row.get(0),
        )?;
        self.conn.execute(
            "INSERT INTO bookmarks (url, title, created_at, position) VALUES (?1, ?2, ?3, ?4)",
            params![url, title, now, position],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn remove_bookmark(&self, id: i64) -> Result<usize> {
        Ok(self
            .conn
            .execute("DELETE FROM bookmarks WHERE id = ?1", params![id])?)
    }

    pub fn bookmarks(&self) -> Result<Vec<Bookmark>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, url, title, created_at, position FROM bookmarks ORDER BY position ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Bookmark {
                id: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                created_at: row.get(3)?,
                position: row.get(4)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn search_bookmarks(&self, query: &str, limit: usize) -> Result<Vec<Bookmark>> {
        let like = format!("%{}%", escape_like(query));
        let mut stmt = self.conn.prepare(
            "SELECT id, url, title, created_at, position FROM bookmarks
             WHERE title LIKE ?1 ESCAPE '\\' OR url LIKE ?1 ESCAPE '\\'
             ORDER BY position ASC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![like, limit as i64], |row| {
            Ok(Bookmark {
                id: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                created_at: row.get(3)?,
                position: row.get(4)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    // -- settings --------------------------------------------------------

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM settings WHERE key = ?1", params![key], |row| {
                row.get(0)
            })
            .optional()?)
    }

    pub fn get_setting_or<T: serde::de::DeserializeOwned>(
        &self,
        key: &str,
        default: T,
    ) -> Result<T> {
        match self.get_setting(key)? {
            Some(raw) => serde_json::from_str(&raw)
                .map_err(|e| StoreError::Json(key.to_string(), e.to_string())),
            None => Ok(default),
        }
    }

    pub fn set_setting_json<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let raw = serde_json::to_string(value)
            .map_err(|e| StoreError::Json(key.to_string(), e.to_string()))?;
        self.set_setting(key, &raw)
    }

    // -- per-site rules --------------------------------------------------

    pub fn set_site_adblock(&self, host: &str, enabled: bool) -> Result<()> {
        self.conn.execute(
            "INSERT INTO site_rules (host, adblock_enabled) VALUES (?1, ?2)
             ON CONFLICT(host) DO UPDATE SET adblock_enabled = excluded.adblock_enabled",
            params![host, enabled as i64],
        )?;
        Ok(())
    }

    pub fn site_adblock_enabled(&self, host: &str) -> Result<Option<bool>> {
        Ok(self
            .conn
            .query_row(
                "SELECT adblock_enabled FROM site_rules WHERE host = ?1",
                params![host],
                |row| Ok(row.get::<_, i64>(0)? != 0),
            )
            .optional()?)
    }
}

fn map_history(row: &rusqlite::Row<'_>) -> rusqlite::Result<HistoryEntry> {
    Ok(HistoryEntry {
        url: row.get(0)?,
        title: row.get(1)?,
        visit_count: row.get(2)?,
        last_visit: row.get(3)?,
    })
}

/// Turn free-form user input into a safe FTS5 prefix query.
///
/// `"foo bar"` becomes `"foo"* "bar"*`. Returns `None` when the input has no
/// usable characters.
pub fn fts_prefix_query(raw: &str) -> Option<String> {
    let terms: Vec<String> = raw
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .take(8)
        .map(|t| format!("\"{}\"*", t.replace('"', "")))
        .collect();
    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" "))
    }
}

fn escape_like(raw: &str) -> String {
    raw.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

const MIGRATION_V1: &str = r#"
CREATE TABLE history (
  id          INTEGER PRIMARY KEY,
  url         TEXT NOT NULL UNIQUE,
  title       TEXT NOT NULL DEFAULT '',
  visit_count INTEGER NOT NULL DEFAULT 1,
  last_visit  INTEGER NOT NULL
);
CREATE INDEX history_last_visit_idx ON history (last_visit DESC);

CREATE VIRTUAL TABLE history_fts USING fts5(
  url,
  title,
  content='history',
  content_rowid='id',
  tokenize='unicode61'
);

CREATE TRIGGER history_ai AFTER INSERT ON history BEGIN
  INSERT INTO history_fts (rowid, url, title) VALUES (new.id, new.url, new.title);
END;
CREATE TRIGGER history_ad AFTER DELETE ON history BEGIN
  INSERT INTO history_fts (history_fts, rowid, url, title) VALUES ('delete', old.id, old.url, old.title);
END;
CREATE TRIGGER history_au AFTER UPDATE ON history BEGIN
  INSERT INTO history_fts (history_fts, rowid, url, title) VALUES ('delete', old.id, old.url, old.title);
  INSERT INTO history_fts (rowid, url, title) VALUES (new.id, new.url, new.title);
END;

CREATE TABLE tabs (
  id          INTEGER PRIMARY KEY,
  url         TEXT NOT NULL,
  title       TEXT NOT NULL DEFAULT '',
  favicon     TEXT,
  scroll_y    REAL NOT NULL DEFAULT 0,
  position    INTEGER NOT NULL,
  last_active INTEGER NOT NULL
);

CREATE TABLE sessions (
  id            INTEGER PRIMARY KEY CHECK (id = 1),
  created_at    INTEGER NOT NULL,
  active_tab    INTEGER,
  window_layout TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE bookmarks (
  id         INTEGER PRIMARY KEY,
  url        TEXT NOT NULL,
  title      TEXT NOT NULL DEFAULT '',
  created_at INTEGER NOT NULL,
  position   INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE TABLE site_rules (
  host            TEXT PRIMARY KEY,
  adblock_enabled INTEGER NOT NULL DEFAULT 1
);
"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Store {
        Store::open_in_memory().unwrap()
    }

    #[test]
    fn records_and_bumps_visits() {
        let store = store();
        store.record_visit("https://a.test", "A", 100).unwrap();
        store.record_visit("https://a.test", "A better", 200).unwrap();
        let entry = store.history_for("https://a.test").unwrap().unwrap();
        assert_eq!(entry.visit_count, 2);
        assert_eq!(entry.last_visit, 200);
        assert_eq!(entry.title, "A better");
    }

    #[test]
    fn empty_title_does_not_clobber_existing_title() {
        let store = store();
        store.record_visit("https://a.test", "Real title", 1).unwrap();
        store.record_visit("https://a.test", "", 2).unwrap();
        assert_eq!(
            store.history_for("https://a.test").unwrap().unwrap().title,
            "Real title"
        );
    }

    #[test]
    fn fts_search_finds_prefix_and_ignores_operators() {
        let store = store();
        store
            .record_visit("https://rust-lang.org", "Rust Programming", 1)
            .unwrap();
        store
            .record_visit("https://example.com", "Example", 2)
            .unwrap();

        let hits = store.search_history("rus", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].url, "https://rust-lang.org");

        assert!(store.search_history("rust OR", 10).is_ok());
        assert!(store.search_history("   ", 10).unwrap().is_empty());
        assert!(store.search_history("!!!", 10).unwrap().is_empty());
    }

    #[test]
    fn recent_history_is_ordered_and_limited() {
        let store = store();
        for (i, url) in ["https://a", "https://b", "https://c"].iter().enumerate() {
            store.record_visit(url, url, i as i64).unwrap();
        }
        let recent = store.recent_history(2).unwrap();
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].url, "https://c");
    }

    #[test]
    fn deletes_history_and_fts_row() {
        let store = store();
        store.record_visit("https://a", "Alpha", 1).unwrap();
        assert_eq!(store.delete_history_url("https://a").unwrap(), 1);
        assert!(store.search_history("alpha", 10).unwrap().is_empty());
        store.clear_history().unwrap();
        assert!(store.recent_history(10).unwrap().is_empty());
    }

    #[test]
    fn saves_and_loads_tabs_in_position_order() {
        let mut store = store();
        let tabs = vec![
            StoredTab {
                id: 2,
                url: "https://two".into(),
                title: "Two".into(),
                favicon: None,
                scroll_y: 10.5,
                position: 1,
                last_active: 20,
            },
            StoredTab {
                id: 1,
                url: "https://one".into(),
                title: "One".into(),
                favicon: Some("data:image/png;base64,AA==".into()),
                scroll_y: 0.0,
                position: 0,
                last_active: 10,
            },
        ];
        store.save_tabs(&tabs).unwrap();
        let loaded = store.load_tabs().unwrap();
        assert_eq!(loaded, vec![tabs[1].clone(), tabs[0].clone()]);

        store.save_tabs(&tabs[..1]).unwrap();
        assert_eq!(store.load_tabs().unwrap().len(), 1);
    }

    #[test]
    fn session_round_trip() {
        let store = store();
        assert_eq!(store.load_session().unwrap(), None);
        let session = Session {
            active_tab: Some(3),
            window_layout: r#"{"width":1280}"#.into(),
            created_at: 42,
        };
        store.save_session(&session).unwrap();
        assert_eq!(store.load_session().unwrap(), Some(session));
    }

    #[test]
    fn bookmarks_crud_and_search() {
        let store = store();
        let a = store.add_bookmark("https://a.test", "Alpha", 1).unwrap();
        store.add_bookmark("https://b.test", "Beta", 2).unwrap();
        assert_eq!(store.bookmarks().unwrap().len(), 2);
        assert_eq!(store.search_bookmarks("alp", 5).unwrap()[0].id, a);
        assert_eq!(store.search_bookmarks("100%_x", 5).unwrap().len(), 0);
        assert_eq!(store.remove_bookmark(a).unwrap(), 1);
        assert_eq!(store.bookmarks().unwrap().len(), 1);
    }

    #[test]
    fn settings_round_trip_json_and_raw() {
        let store = store();
        store.set_setting("theme", "dark").unwrap();
        assert_eq!(store.get_setting("theme").unwrap().as_deref(), Some("dark"));
        store.set_setting_json("tab_limit", &5u8).unwrap();
        assert_eq!(store.get_setting_or::<u8>("tab_limit", 3).unwrap(), 5);
        assert_eq!(store.get_setting_or::<u8>("missing", 3).unwrap(), 3);
    }

    #[test]
    fn site_rules_round_trip() {
        let store = store();
        assert_eq!(store.site_adblock_enabled("a.test").unwrap(), None);
        store.set_site_adblock("a.test", false).unwrap();
        assert_eq!(store.site_adblock_enabled("a.test").unwrap(), Some(false));
        store.set_site_adblock("a.test", true).unwrap();
        assert_eq!(store.site_adblock_enabled("a.test").unwrap(), Some(true));
    }

    #[test]
    fn fts_query_sanitizes_operators() {
        assert_eq!(fts_prefix_query("rust lang").unwrap(), "\"rust\"* \"lang\"*");
        assert_eq!(fts_prefix_query("a\"b").unwrap(), "\"a\"* \"b\"*");
        assert_eq!(
            fts_prefix_query("x\" OR title:").unwrap(),
            "\"x\"* \"OR\"* \"title\"*"
        );
        assert_eq!(fts_prefix_query("   "), None);
    }
}
