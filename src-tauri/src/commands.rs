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
    } else if s.db_path().exists() {
        // Há um livro cifrado, mas o arquivo do cofre sumiu: nunca oferecer "criar" por cima.
        "missing_vault"
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
    if s.db_path().exists() {
        return Err("DB_EXISTS".into());
    }
    std::fs::create_dir_all(&s.dir).map_err(err)?;
    let (v, key, phrase) = vault::create(nickname, password).map_err(err)?;
    // Cria o banco antes de gravar o cofre; se o cofre não puder ser gravado, desfaz o banco.
    open_db(s, &key)?;
    if let Err(e) = vault::save(&s.vault_path(), &v) {
        lock_vault(s);
        let _ = std::fs::remove_file(s.db_path());
        return Err(err(e));
    }
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

/// Apelido do cofre (fica em texto puro no vault.json; não é segredo). `None` sem cofre.
pub fn vault_nickname_of(s: &AppState) -> CmdResult<Option<String>> {
    Ok(vault::load(&s.vault_path()).map_err(err)?.map(|v| v.nickname))
}

#[tauri::command]
pub fn vault_nickname(state: State<AppState>) -> CmdResult<Option<String>> {
    vault_nickname_of(&state)
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

// ---------- livro ----------
use crate::db::books::{self, Book, Chapter, Outline};
use crate::db::scenes::{self, Scene, Version};
use crate::db::settings;

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn with_db<T>(s: &AppState, f: impl FnOnce(&rusqlite::Connection) -> Result<T, db::DbError>) -> CmdResult<T> {
    let g = s.db.lock().unwrap();
    let c = g.as_ref().ok_or_else(|| "LOCKED".to_string())?;
    f(c).map_err(err)
}

macro_rules! db_cmd {
    ($name:ident ( $($arg:ident : $ty:ty),* ) -> $ret:ty => |$c:ident| $body:expr) => {
        #[tauri::command]
        pub fn $name(state: State<AppState>, $($arg: $ty),*) -> CmdResult<$ret> {
            with_db(&state, |$c| $body)
        }
    };
}

db_cmd!(book_list() -> Vec<Book> => |c| books::book_list(c));
db_cmd!(book_create(title: String, ai_mode: String) -> Book => |c| books::book_create(c, &title, &ai_mode, now_ms()));
db_cmd!(book_rename(id: String, title: String) -> () => |c| books::book_rename(c, &id, &title));
db_cmd!(book_set_mode(id: String, ai_mode: String) -> () => |c| books::book_set_mode(c, &id, &ai_mode));
db_cmd!(book_outline(book_id: String) -> Outline => |c| books::book_outline(c, &book_id));
db_cmd!(chapter_create(book_id: String, title: String, index: Option<i64>) -> Chapter => |c| books::chapter_create(c, &book_id, &title, index));
db_cmd!(chapter_rename(id: String, title: String) -> () => |c| books::chapter_rename(c, &id, &title));
db_cmd!(chapter_move(id: String, to_index: i64) -> () => |c| books::chapter_move(c, &id, to_index));
db_cmd!(chapter_delete(id: String) -> () => |c| books::chapter_delete(c, &id));
db_cmd!(scene_create(chapter_id: String, title: String, index: Option<i64>) -> Scene => |c| scenes::scene_create(c, &chapter_id, &title, index, now_ms()));
db_cmd!(scene_get(id: String) -> Scene => |c| scenes::scene_get(c, &id));
db_cmd!(scene_rename(id: String, title: String) -> () => |c| scenes::scene_rename(c, &id, &title));
db_cmd!(scene_save(id: String, content_json: String, word_count: i64) -> () => |c| scenes::scene_save(c, &id, &content_json, word_count, now_ms()));
db_cmd!(scene_move(id: String, to_chapter_id: String, to_index: i64) -> () => |c| scenes::scene_move(c, &id, &to_chapter_id, to_index));
db_cmd!(scene_delete(id: String) -> () => |c| scenes::scene_delete(c, &id));
db_cmd!(scene_versions(scene_id: String) -> Vec<Version> => |c| scenes::scene_versions(c, &scene_id));
db_cmd!(scene_restore(version_id: String) -> Scene => |c| scenes::scene_restore(c, &version_id, now_ms()));
db_cmd!(setting_get(key: String) -> Option<String> => |c| settings::setting_get(c, &key));
db_cmd!(setting_set(key: String, value: String) -> () => |c| settings::setting_set(c, &key, &value));

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> (tempfile::TempDir, AppState) {
        let d = tempfile::tempdir().unwrap();
        let s = AppState::new(d.path().to_path_buf());
        (d, s)
    }

    #[test]
    fn db_commands_require_unlock() {
        let (_d, s) = state();
        create_vault(&s, "ana", "x").unwrap();
        lock_vault(&s);
        assert_eq!(with_db(&s, books::book_list).unwrap_err(), "LOCKED");
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
    fn create_over_existing_book_is_db_exists_and_writes_no_vault() {
        let (_d, s) = state();
        std::fs::write(s.db_path(), b"livro antigo cifrado").unwrap();
        assert_eq!(create_vault(&s, "ana", "x").unwrap_err(), "DB_EXISTS");
        assert!(!s.vault_path().exists());
        assert_eq!(std::fs::read(s.db_path()).unwrap(), b"livro antigo cifrado");
    }

    #[test]
    fn status_reports_missing_vault_when_book_exists() {
        let (_d, s) = state();
        std::fs::write(s.db_path(), b"x").unwrap();
        assert_eq!(status(&s), "missing_vault");
    }

    #[test]
    fn nickname_is_readable_while_locked() {
        let (_d, s) = state();
        assert_eq!(vault_nickname_of(&s).unwrap(), None);
        create_vault(&s, "henri", "x").unwrap();
        lock_vault(&s);
        assert_eq!(vault_nickname_of(&s).unwrap().as_deref(), Some("henri"));
    }

    #[test]
    fn create_in_fresh_install_creates_data_dir() {
        let d = tempfile::tempdir().unwrap();
        let s = AppState::new(d.path().join("ainda-nao-existe"));
        create_vault(&s, "ana", "x").unwrap();
        assert_eq!(status(&s), "unlocked");
        assert!(s.vault_path().exists());
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
