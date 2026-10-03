//! The SQLite database: notes, screenshots, paired devices, audit log.

use std::path::Path;

use parking_lot::Mutex;
use rusqlite::Connection;

pub struct Db {
    conn: Mutex<Connection>,
}

const MIGRATIONS: &[&str] = &[
    // 1
    r#"
    CREATE TABLE notes (
        id TEXT PRIMARY KEY,
        kind TEXT NOT NULL DEFAULT 'note',
        title TEXT NOT NULL DEFAULT '',
        body TEXT NOT NULL DEFAULT '',
        tags TEXT NOT NULL DEFAULT '[]',
        pinned INTEGER NOT NULL DEFAULT 0,
        color TEXT,
        created INTEGER NOT NULL,
        updated INTEGER NOT NULL,
        exported_path TEXT,
        exported_at INTEGER
    );
    CREATE INDEX notes_updated ON notes(updated DESC);

    CREATE TABLE screenshots (
        id TEXT PRIMARY KEY,
        path TEXT NOT NULL UNIQUE,
        created INTEGER NOT NULL,
        width INTEGER NOT NULL,
        height INTEGER NOT NULL,
        bytes INTEGER NOT NULL DEFAULT 0,
        app_exe TEXT,
        app_title TEXT,
        tags TEXT NOT NULL DEFAULT '[]',
        note TEXT NOT NULL DEFAULT '',
        favorite INTEGER NOT NULL DEFAULT 0
    );
    CREATE INDEX screenshots_created ON screenshots(created DESC);

    CREATE TABLE devices (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        token_hash TEXT NOT NULL UNIQUE,
        user_agent TEXT NOT NULL DEFAULT '',
        created INTEGER NOT NULL,
        last_seen INTEGER NOT NULL,
        last_ip TEXT NOT NULL DEFAULT '',
        revoked INTEGER NOT NULL DEFAULT 0
    );

    CREATE TABLE audit (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        at INTEGER NOT NULL,
        actor TEXT NOT NULL,
        action TEXT NOT NULL,
        detail TEXT NOT NULL DEFAULT '',
        ok INTEGER NOT NULL DEFAULT 1
    );
    CREATE INDEX audit_at ON audit(at DESC);
    "#,
];

impl Db {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        Self::init(conn)
    }

    pub fn in_memory() -> rusqlite::Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> rusqlite::Result<Self> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
            conn.execute_batch(&format!("BEGIN; {sql}; PRAGMA user_version = {}; COMMIT;", i + 1))?;
        }
        Ok(Db { conn: Mutex::new(conn) })
    }

    pub fn with<T>(&self, f: impl FnOnce(&Connection) -> rusqlite::Result<T>) -> rusqlite::Result<T> {
        f(&self.conn.lock())
    }
}

pub fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_apply_once() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("t.db");
        Db::open(&p).unwrap();
        let db = Db::open(&p).unwrap();
        let v: i64 = db.with(|c| c.query_row("PRAGMA user_version", [], |r| r.get(0))).unwrap();
        assert_eq!(v, MIGRATIONS.len() as i64);
    }
}
