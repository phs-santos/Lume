use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Banco compartilhado: comandos de rede levam uma cópia do Arc para outra thread.
pub type SharedDb = Arc<Mutex<Option<Connection>>>;

pub struct AppState {
    pub dir: PathBuf,
    pub db: SharedDb,
}

impl AppState {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir, db: Arc::new(Mutex::new(None)) }
    }
    pub fn vault_path(&self) -> PathBuf {
        self.dir.join("vault.json")
    }
    pub fn db_path(&self) -> PathBuf {
        self.dir.join("livro.db")
    }
}
