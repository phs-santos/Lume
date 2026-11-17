use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct AppState {
    pub dir: PathBuf,
    pub db: Mutex<Option<Connection>>,
}

impl AppState {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir, db: Mutex::new(None) }
    }
    pub fn vault_path(&self) -> PathBuf {
        self.dir.join("vault.json")
    }
    pub fn db_path(&self) -> PathBuf {
        self.dir.join("livro.db")
    }
}
