//! Banco SQLCipher local. A chave é a chave-mestra do cofre.

use crate::vault::MasterKey;
use rusqlite::Connection;
use std::path::Path;

pub mod ai;
pub mod books;
pub mod images;
pub mod inbox;
pub mod scenes;
pub mod settings;

/// Transação aninhável (SAVEPOINT). Sem `commit()`, desfaz ao sair do escopo.
pub(crate) struct Atomic<'a> {
    c: &'a Connection,
    done: bool,
}

impl<'a> Atomic<'a> {
    pub fn begin(c: &'a Connection) -> Result<Self, DbError> {
        c.execute_batch("SAVEPOINT atomic")?;
        Ok(Self { c, done: false })
    }
    pub fn commit(mut self) -> Result<(), DbError> {
        self.c.execute_batch("RELEASE atomic")?;
        self.done = true;
        Ok(())
    }
}

impl Drop for Atomic<'_> {
    fn drop(&mut self) {
        if !self.done {
            let _ = self.c.execute_batch("ROLLBACK TO atomic; RELEASE atomic");
        }
    }
}

/// Reescreve `position` = 0..n-1 para os ids na ordem dada.
pub(crate) fn write_positions(c: &Connection, table: &str, ids: &[String]) -> Result<(), DbError> {
    let mut st = c.prepare(&format!("UPDATE {table} SET position=?1 WHERE id=?2"))?;
    for (i, id) in ids.iter().enumerate() {
        st.execute(rusqlite::params![i as i64, id])?;
    }
    Ok(())
}

/// Ids de `table` cujo `parent_col` = `parent`, em ordem de posição.
pub(crate) fn ordered_ids(c: &Connection, table: &str, parent_col: &str, parent: &str) -> Result<Vec<String>, DbError> {
    let mut st = c.prepare(&format!("SELECT id FROM {table} WHERE {parent_col}=?1 ORDER BY position"))?;
    let ids = st.query_map([parent], |r| r.get(0))?.collect::<Result<Vec<String>, _>>()?;
    Ok(ids)
}

/// Insere `id` em `ids` no índice dado, limitado ao intervalo válido (None = fim).
pub(crate) fn insert_clamped(ids: &mut Vec<String>, id: String, index: Option<i64>) {
    let i = index.map(|i| i.clamp(0, ids.len() as i64) as usize).unwrap_or(ids.len());
    ids.insert(i, id);
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("WRONG_KEY")]
    WrongKey,
    #[error("NOT_FOUND")]
    NotFound,
    #[error("CHAPTER_NOT_EMPTY")]
    ChapterNotEmpty,
    #[error("INVALID: {0}")]
    Invalid(String),
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
"#, r#"
CREATE TABLE inbox(
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL CHECK(kind IN('audio','text')),
  mime TEXT NOT NULL,
  body BLOB NOT NULL,
  created_at INTEGER NOT NULL,
  received_at INTEGER NOT NULL,
  status TEXT NOT NULL CHECK(status IN('nova','arquivada'))
);
"#, r#"
CREATE TABLE ai_scene_digest(
  scene_id TEXT PRIMARY KEY REFERENCES scenes(id) ON DELETE CASCADE,
  content_hash TEXT NOT NULL,
  summary TEXT NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE TABLE bible_entries(
  id TEXT PRIMARY KEY,
  book_id TEXT NOT NULL REFERENCES books(id),
  kind TEXT NOT NULL CHECK(kind IN('personagem','lugar','fato','linha_do_tempo')),
  name TEXT NOT NULL,
  aliases_json TEXT NOT NULL DEFAULT '[]',
  body TEXT NOT NULL,
  ai_notes_json TEXT NOT NULL DEFAULT '[]',
  story_order INTEGER,
  source_scene_id TEXT REFERENCES scenes(id) ON DELETE SET NULL,
  edited_by_user INTEGER NOT NULL DEFAULT 0,
  updated_at INTEGER NOT NULL
);
CREATE TABLE ai_conversations(
  id TEXT PRIMARY KEY,
  book_id TEXT NOT NULL REFERENCES books(id),
  title TEXT NOT NULL,
  rolling_summary TEXT NOT NULL DEFAULT '',
  folded_count INTEGER NOT NULL DEFAULT 0,
  closed INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE TABLE ai_messages(
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL REFERENCES ai_conversations(id) ON DELETE CASCADE,
  role TEXT NOT NULL CHECK(role IN('user','assistant','note')),
  content TEXT NOT NULL,
  tool_name TEXT,
  created_at INTEGER NOT NULL,
  seq INTEGER NOT NULL
);
CREATE TABLE ai_proposals(
  id TEXT PRIMARY KEY,
  book_id TEXT NOT NULL REFERENCES books(id),
  conversation_id TEXT REFERENCES ai_conversations(id) ON DELETE SET NULL,
  scene_id TEXT REFERENCES scenes(id) ON DELETE SET NULL,
  text TEXT NOT NULL,
  status TEXT NOT NULL CHECK(status IN('pendente','inserida','virou_cena','descartada')),
  created_at INTEGER NOT NULL
);
CREATE TABLE ai_usage(
  month TEXT NOT NULL,
  model TEXT NOT NULL,
  input_tokens INTEGER NOT NULL DEFAULT 0,
  output_tokens INTEGER NOT NULL DEFAULT 0,
  cost REAL,
  PRIMARY KEY(month, model)
);
"#, r#"
ALTER TABLE inbox ADD COLUMN transcript TEXT;
ALTER TABLE inbox ADD COLUMN transcript_status TEXT CHECK(transcript_status IN('pendente','pronta','erro','sem_chave'));
ALTER TABLE inbox ADD COLUMN transcript_error TEXT;
UPDATE inbox SET transcript_status='pendente' WHERE kind='audio';
"#, r#"
CREATE TABLE images(
  id TEXT PRIMARY KEY,
  book_id TEXT NOT NULL REFERENCES books(id),
  mime TEXT NOT NULL,
  bytes BLOB NOT NULL,
  width INTEGER,
  height INTEGER,
  created_at INTEGER NOT NULL
);
CREATE INDEX images_book ON images(book_id);
ALTER TABLE books ADD COLUMN author TEXT;
ALTER TABLE books ADD COLUMN synopsis TEXT;
ALTER TABLE books ADD COLUMN dedication TEXT;
ALTER TABLE books ADD COLUMN cover_image_id TEXT;
ALTER TABLE books ADD COLUMN back_image_id TEXT;
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

/// Conexão em arquivo temporário para testes de outros módulos.
#[cfg(test)]
pub fn test_conn() -> (tempfile::TempDir, Connection) {
    let d = tempfile::tempdir().unwrap();
    let c = open(&d.path().join("t.db"), &MasterKey([1; 32])).unwrap();
    (d, c)
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
    fn migration_v2_keeps_existing_books() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("livro.db");
        {
            // banco no formato v1, como o de quem já usava o Lume
            let c = Connection::open(&p).unwrap();
            let hex: String = key(7).0.iter().map(|b| format!("{b:02x}")).collect();
            c.execute_batch(&format!("PRAGMA key = \"x'{hex}'\"; {} PRAGMA user_version = 1;", MIGRATIONS[0])).unwrap();
            insert_book(&c, "Livro antigo");
        }
        let c = open(&p, &key(7)).unwrap();
        let t: String = c.query_row("SELECT title FROM books", [], |r| r.get(0)).unwrap();
        assert_eq!(t, "Livro antigo");
        let v: i64 = c.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(v, MIGRATIONS.len() as i64);
        c.execute("SELECT count(*) FROM inbox", []).ok();
    }

    #[test]
    fn migration_v4_marks_existing_audio_as_pending() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("livro.db");
        {
            let c = Connection::open(&p).unwrap();
            let hex: String = key(7).0.iter().map(|b| format!("{b:02x}")).collect();
            c.execute_batch(&format!("PRAGMA key = \"x'{hex}'\"; {} {} {} PRAGMA user_version = 3;", MIGRATIONS[0], MIGRATIONS[1], MIGRATIONS[2])).unwrap();
            c.execute("INSERT INTO inbox(id,kind,mime,body,created_at,received_at,status) VALUES('a','audio','audio/mp4',x'01',1,1,'nova')", []).unwrap();
            c.execute("INSERT INTO inbox(id,kind,mime,body,created_at,received_at,status) VALUES('t','text','text/plain',x'01',1,1,'nova')", []).unwrap();
        }
        let c = open(&p, &key(7)).unwrap();
        let st = |id: &str| c.query_row("SELECT transcript_status FROM inbox WHERE id=?1", [id], |r| r.get::<_, Option<String>>(0)).unwrap();
        assert_eq!(st("a").as_deref(), Some("pendente"));
        assert_eq!(st("t"), None);
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
