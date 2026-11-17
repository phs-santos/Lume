//! Lógica dos comandos sobre `AppState` + wrappers `#[tauri::command]` finos.
//! Erros chegam ao frontend como string com código.

use crate::state::AppState;

pub type CmdResult<T> = Result<T, String>;

use crate::{db, vault};
use tauri::State;

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

pub fn status(s: &AppState) -> &'static str {
    if s.db.lock().unwrap().is_some() {
        "unlocked"
    } else if s.vault_path().exists() {
        "locked"
    } else {
        "none"
    }
}

fn open_db(s: &AppState, key: &vault::MasterKey) -> CmdResult<()> {
    let c = db::open(&s.db_path(), key).map_err(err)?;
    *s.db.lock().unwrap() = Some(c);
    Ok(())
}

pub fn create_vault(s: &AppState, nickname: &str, password: &str) -> CmdResult<String> {
    if s.vault_path().exists() {
        return Err("VAULT_EXISTS".into());
    }
    let (v, key, phrase) = vault::create(nickname, password).map_err(err)?;
    vault::save(&s.vault_path(), &v).map_err(err)?;
    open_db(s, &key)?;
    Ok(phrase)
}

fn load_vault(s: &AppState) -> CmdResult<vault::VaultFile> {
    vault::load(&s.vault_path()).map_err(err)?.ok_or_else(|| "NO_VAULT".to_string())
}

pub fn unlock_vault(s: &AppState, password: &str) -> CmdResult<()> {
    let v = load_vault(s)?;
    let key = vault::unlock(&v, password).map_err(err)?;
    open_db(s, &key)
}

pub fn recover_vault(s: &AppState, phrase: &str, new_password: &str) -> CmdResult<()> {
    let v = load_vault(s)?;
    let (nv, key) = vault::recover(&v, phrase, new_password).map_err(err)?;
    open_db(s, &key)?;
    vault::save(&s.vault_path(), &nv).map_err(err)
}

pub fn lock_vault(s: &AppState) {
    *s.db.lock().unwrap() = None;
}

#[tauri::command]
pub fn vault_status(state: State<AppState>) -> &'static str {
    status(&state)
}
#[tauri::command]
pub fn vault_create(state: State<AppState>, nickname: String, password: String) -> CmdResult<String> {
    create_vault(&state, &nickname, &password)
}
#[tauri::command]
pub fn vault_unlock(state: State<AppState>, password: String) -> CmdResult<()> {
    unlock_vault(&state, &password)
}
#[tauri::command]
pub fn vault_recover(state: State<AppState>, phrase: String, new_password: String) -> CmdResult<()> {
    recover_vault(&state, &phrase, &new_password)
}
#[tauri::command]
pub fn vault_lock(state: State<AppState>) {
    lock_vault(&state)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> (tempfile::TempDir, AppState) {
        let d = tempfile::tempdir().unwrap();
        let s = AppState::new(d.path().to_path_buf());
        (d, s)
    }

    #[test]
    fn lifecycle_none_unlocked_locked() {
        let (_d, s) = state();
        assert_eq!(status(&s), "none");
        create_vault(&s, "ana", "senha-boa").unwrap();
        assert_eq!(status(&s), "unlocked");
        lock_vault(&s);
        assert_eq!(status(&s), "locked");
        unlock_vault(&s, "senha-boa").unwrap();
        assert_eq!(status(&s), "unlocked");
    }

    #[test]
    fn create_twice_is_vault_exists() {
        let (_d, s) = state();
        create_vault(&s, "ana", "x").unwrap();
        assert_eq!(create_vault(&s, "ana", "y").unwrap_err(), "VAULT_EXISTS");
    }

    #[test]
    fn unlock_without_vault_is_no_vault() {
        let (_d, s) = state();
        assert_eq!(unlock_vault(&s, "x").unwrap_err(), "NO_VAULT");
    }

    #[test]
    fn wrong_password_leaves_files_untouched() {
        let (_d, s) = state();
        create_vault(&s, "ana", "certa").unwrap();
        lock_vault(&s);
        let v = std::fs::read(s.vault_path()).unwrap();
        let db = std::fs::read(s.db_path()).unwrap();
        assert_eq!(unlock_vault(&s, "errada").unwrap_err(), "WRONG_PASSWORD");
        assert_eq!(status(&s), "locked");
        assert_eq!(std::fs::read(s.vault_path()).unwrap(), v);
        assert_eq!(std::fs::read(s.db_path()).unwrap(), db);
    }

    #[test]
    fn recover_unlocks_and_replaces_password() {
        let (_d, s) = state();
        let phrase = create_vault(&s, "ana", "velha").unwrap();
        lock_vault(&s);
        assert_eq!(recover_vault(&s, "abandon abandon", "nova").unwrap_err(), "INVALID_PHRASE");
        recover_vault(&s, &phrase, "nova").unwrap();
        assert_eq!(status(&s), "unlocked");
        lock_vault(&s);
        assert_eq!(unlock_vault(&s, "velha").unwrap_err(), "WRONG_PASSWORD");
        unlock_vault(&s, "nova").unwrap();
    }
}
