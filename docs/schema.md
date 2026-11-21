# Peek — Database Schema (v2)

Peek uses SQLite in WAL mode (`journal_mode = WAL`, `foreign_keys = ON`) with one database per profile located at:
- macOS: `~/Library/Application Support/dev.Peek.Peek/profiles/<profile>/peek.db`
- Windows: `%APPDATA%\Peek\Peek\profiles\<profile>\peek.db`
- Linux: `~/.local/share/Peek/profiles/<profile>/peek.db`

---

## 1. Tables

### 1.1 `history`
Stores visited URLs, titles, visit frequency, and timestamp of the last visit.
```sql
CREATE TABLE history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    url TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL DEFAULT '',
    visit_count INTEGER NOT NULL DEFAULT 1,
    last_visit INTEGER NOT NULL
);
CREATE INDEX idx_history_last_visit ON history(last_visit DESC);
CREATE INDEX idx_history_visit_count ON history(visit_count DESC);
```

### 1.2 `history_fts` (Full-Text Search)
FTS5 external content table indexing `title` and `url` from `history` for instant omnibar suggestions.
```sql
CREATE VIRTUAL TABLE history_fts USING fts5(
    title,
    url,
    content='history',
    content_rowid='id'
);

-- Automated triggers keep FTS5 synchronized on INSERT, UPDATE, and DELETE:
CREATE TRIGGER history_ai AFTER INSERT ON history BEGIN
    INSERT INTO history_fts(rowid, title, url) VALUES (new.id, new.title, new.url);
END;

CREATE TRIGGER history_ad AFTER DELETE ON history BEGIN
    INSERT INTO history_fts(history_fts, rowid, title, url) VALUES('delete', old.id, old.title, old.url);
END;

CREATE TRIGGER history_au AFTER UPDATE ON history BEGIN
    INSERT INTO history_fts(history_fts, rowid, title, url) VALUES('delete', old.id, old.title, old.url);
    INSERT INTO history_fts(rowid, title, url) VALUES (new.id, new.title, new.url);
END;
```

### 1.3 `tabs`
Stores tabs for session persistence across application restarts and LRU discard restoration.
```sql
CREATE TABLE tabs (
    id INTEGER PRIMARY KEY,
    url TEXT NOT NULL,
    title TEXT NOT NULL DEFAULT '',
    favicon TEXT,
    scroll_y REAL NOT NULL DEFAULT 0.0,
    position INTEGER NOT NULL,
    last_active INTEGER NOT NULL
);
CREATE INDEX idx_tabs_position ON tabs(position ASC);
```

### 1.4 `sessions`
Stores window layout state and active tab ID.
```sql
CREATE TABLE sessions (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    created_at INTEGER NOT NULL,
    active_tab INTEGER,
    window_layout TEXT NOT NULL DEFAULT ''
);
```

### 1.5 `bookmarks`
Saved URLs with user-defined or page titles.
```sql
CREATE TABLE bookmarks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    url TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL DEFAULT '',
    created_at INTEGER NOT NULL,
    position INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_bookmarks_position ON bookmarks(position ASC);
```

### 1.6 `settings`
Key-value store storing typed JSON blobs for runtime settings.
```sql
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

### 1.7 `site_rules`
Per-domain rules (e.g. ad blocking exceptions).
```sql
CREATE TABLE site_rules (
    host TEXT PRIMARY KEY,
    adblock_enabled INTEGER NOT NULL DEFAULT 1
);
```

### 1.8 `cookies` (v2)
Stores persistent HTTP and client-side cookies per profile.
```sql
CREATE TABLE cookies (
    domain      TEXT NOT NULL,
    name        TEXT NOT NULL,
    path        TEXT NOT NULL DEFAULT '/',
    value       TEXT NOT NULL,
    expires_at  INTEGER,
    secure      INTEGER NOT NULL DEFAULT 0,
    http_only   INTEGER NOT NULL DEFAULT 0,
    same_site   TEXT,
    created_at  INTEGER NOT NULL,
    PRIMARY KEY (domain, name, path)
);
CREATE INDEX cookies_domain_idx ON cookies(domain);
```

---

## 2. Integrity & Concurrency
- `user_version` PRAGMA tracks migrations (`SCHEMA_VERSION = 2`).
- FTS queries are sanitized using quoted token prefix matching (e.g. `"foo"* "bar"*`) to prevent syntax injection errors from user input.
- Database writes run inside atomic transactions.
