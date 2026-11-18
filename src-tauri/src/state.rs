use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Banco compartilhado: comandos de rede levam uma cópia do Arc para outra thread.
pub type SharedDb = Arc<Mutex<Option<Connection>>>;

pub struct AppState {
    /// Raiz da pasta de dados (é também a pasta do usuário principal).
    pub dir: PathBuf,
    /// Pasta do usuário ativo (a raiz ou `perfis/<id>`).
    pub active: Mutex<PathBuf>,
    pub db: SharedDb,
}

impl AppState {
    pub fn new(dir: PathBuf) -> Self {
        Self { active: Mutex::new(dir.clone()), dir, db: Arc::new(Mutex::new(None)) }
    }
    pub fn profile_dir(&self) -> PathBuf {
        self.active.lock().unwrap().clone()
    }
    pub fn vault_path(&self) -> PathBuf {
        self.profile_dir().join("vault.json")
    }
    pub fn db_path(&self) -> PathBuf {
        self.profile_dir().join("livro.db")
    }
}
