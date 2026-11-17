//! Banco SQLCipher local. A chave é a chave-mestra do cofre.

use crate::vault::MasterKey;
use rusqlite::Connection;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("WRONG_KEY")]
    WrongKey,
    #[error("{0}")]
    Sql(#[from] rusqlite::Error),
}

const MIGRATIONS: &[&str] = &[r#"
CREATE TABLE books(
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  ai_mode TEXT NOT NULL CHECK(ai_mode IN('conselheira','coautora')),
  created_at INTEGER NOT NULL
);
CREATE TABLE chapters(
  id TEXT PRIMARY KEY,
  book_id TEXT NOT NULL REFERENCES books(id),
  title TEXT NOT NULL,
  position INTEGER NOT NULL
);
CREATE TABLE scenes(
  id TEXT PRIMARY KEY,
  chapter_id TEXT NOT NULL REFERENCES chapters(id),
  title TEXT NOT NULL,
  content_json TEXT NOT NULL,
  word_count INTEGER NOT NULL DEFAULT 0,
  position INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE TABLE scene_versions(
  id TEXT PRIMARY KEY,
  scene_id TEXT NOT NULL REFERENCES scenes(id),
  content_json TEXT NOT NULL,
  word_count INTEGER NOT NULL,
  created_at INTEGER NOT NULL
);
CREATE TABLE settings(key TEXT PRIMARY KEY, value TEXT NOT NULL);
"#];

pub fn open(path: &Path, key: &MasterKey) -> Result<Connection, DbError> {
    let c = Connection::open(path)?;
    let hex: String = key.0.iter().map(|b| format!("{b:02x}")).collect();
    c.execute_batch(&format!("PRAGMA key = \"x'{hex}'\";"))?;
    // Com a chave errada, a primeira leitura falha com "file is not a database".
    if c.query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get::<_, i64>(0)).is_err() {
        return Err(DbError::WrongKey);
    }
    c.execute_batch("PRAGMA foreign_keys = ON;")?;
    migrate(&c)?;
    Ok(c)
}

fn migrate(c: &Connection) -> Result<(), DbError> {
    let current: usize = c.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))? as usize;
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(current) {
        c.execute_batch(&format!("BEGIN; {sql} PRAGMA user_version = {}; COMMIT;", i + 1))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(b: u8) -> MasterKey {
        MasterKey([b; 32])
    }

    fn insert_book(c: &Connection, title: &str) {
        c.execute(
            "INSERT INTO books(id,title,ai_mode,created_at) VALUES('b1',?1,'conselheira',0)",
            [title],
        )
        .unwrap();
    }

    #[test]
    fn open_with_right_key_reads_back() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("livro.db");
        insert_book(&open(&p, &key(7)).unwrap(), "Meu livro");
        let c = open(&p, &key(7)).unwrap();
        let t: String = c.query_row("SELECT title FROM books", [], |r| r.get(0)).unwrap();
        assert_eq!(t, "Meu livro");
    }

    #[test]
    fn open_with_wrong_key_fails() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("livro.db");
        insert_book(&open(&p, &key(7)).unwrap(), "x");
        assert!(matches!(open(&p, &key(8)), Err(DbError::WrongKey)));
    }

    #[test]
    fn file_is_not_plaintext() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("livro.db");
        {
            let c = open(&p, &key(7)).unwrap();
            insert_book(&c, "Segredo da Marina");
        }
        let bytes = std::fs::read(&p).unwrap();
        let needle = b"Segredo da Marina";
        assert!(!bytes.windows(needle.len()).any(|w| w == needle));
    }
}
