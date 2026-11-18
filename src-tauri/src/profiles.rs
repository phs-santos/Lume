//! Vários usuários no mesmo computador. Cada um tem a própria pasta com cofre e livros.
//! A raiz da pasta de dados é o usuário "principal" (quem já usava o Lume); os outros ficam em `perfis/<id>/`.

use crate::state::AppState;
use serde::Serialize;

pub const MAIN: &str = "principal";

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Profile {
    pub id: String,
    pub nickname: String,
    pub active: bool,
}

use std::path::{Path, PathBuf};

const LAST: &str = "ultimo-perfil";

fn profiles_root(s: &AppState) -> PathBuf {
    s.dir.join("perfis")
}

fn nickname_at(dir: &Path) -> Option<String> {
    crate::vault::load(&dir.join("vault.json")).ok().flatten().map(|v| v.nickname)
}

/// Pasta de um usuário pelo id. Só aceita ids que já existem (nada de `../`).
fn dir_of(s: &AppState, id: &str) -> Option<PathBuf> {
    if id == MAIN {
        return Some(s.dir.clone());
    }
    let ok = !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    let d = profiles_root(s).join(id);
    (ok && d.is_dir()).then_some(d)
}

fn id_of(s: &AppState, dir: &Path) -> String {
    if dir == s.dir { MAIN.into() } else { dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default() }
}

fn ensure_locked(s: &AppState) -> Result<(), String> {
    if s.db.lock().unwrap().is_some() { Err("UNLOCKED".into()) } else { Ok(()) }
}

/// Usuários com cofre criado, o principal primeiro e os outros por ordem de criação.
pub fn list(s: &AppState) -> Vec<Profile> {
    let active = s.profile_dir();
    let mut out = Vec::new();
    if let Some(n) = nickname_at(&s.dir) {
        out.push(Profile { id: MAIN.into(), nickname: n, active: active == s.dir });
    }
    let mut others: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(profiles_root(s))
        .map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.is_dir()).collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
        .map(|p| (std::fs::metadata(&p).and_then(|m| m.created().or_else(|_| m.modified())).unwrap_or(std::time::UNIX_EPOCH), p))
        .collect();
    others.sort();
    for (_, p) in others {
        if let Some(n) = nickname_at(&p) {
            out.push(Profile { id: id_of(s, &p), nickname: n, active: active == p });
        }
    }
    out
}

/// Troca o usuário ativo (com o cofre trancado). Um usuário novo que nem criou cofre é descartado.
pub fn select(s: &AppState, id: &str) -> Result<(), String> {
    ensure_locked(s)?;
    let target = dir_of(s, id).ok_or_else(|| "NO_PROFILE".to_string())?;
    let previous = s.profile_dir();
    *s.active.lock().unwrap() = target.clone();
    if previous != target && previous != s.dir && !previous.join("vault.json").exists() && !previous.join("livro.db").exists() {
        let _ = std::fs::remove_dir_all(&previous);
    }
    let _ = std::fs::write(s.dir.join(LAST), id);
    Ok(())
}

/// Cria a pasta de um usuário novo e a deixa ativa; a tela segue para "criar cofre".
pub fn new_profile(s: &AppState) -> Result<String, String> {
    ensure_locked(s)?;
    let id = uuid::Uuid::new_v4().to_string();
    std::fs::create_dir_all(profiles_root(s).join(&id)).map_err(|e| e.to_string())?;
    select(s, &id)?;
    Ok(id)
}

/// Ao abrir o app: volta para o último usuário que entrou neste computador.
pub fn restore_last(s: &AppState) {
    if let Ok(id) = std::fs::read_to_string(s.dir.join(LAST)) {
        let id = id.trim();
        if dir_of(s, id).is_some_and(|d| d.join("vault.json").exists()) {
            *s.active.lock().unwrap() = dir_of(s, id).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{create_vault, lock_vault, status, unlock_vault, vault_nickname_of};

    fn state() -> (tempfile::TempDir, AppState) {
        let d = tempfile::tempdir().unwrap();
        let s = AppState::new(d.path().to_path_buf());
        (d, s)
    }

    #[test]
    fn existing_vault_at_root_is_the_main_profile() {
        let (_d, s) = state();
        create_vault(&s, "henri", "senha-1").unwrap();
        lock_vault(&s);
        assert_eq!(list(&s), [Profile { id: MAIN.into(), nickname: "henri".into(), active: true }]);
        assert!(s.vault_path().starts_with(&s.dir) && s.vault_path().parent().unwrap() == s.dir);
    }

    #[test]
    fn new_profile_has_its_own_vault_and_books() {
        let (_d, s) = state();
        create_vault(&s, "henri", "senha-1").unwrap();
        lock_vault(&s);
        let id = new_profile(&s).unwrap();
        assert_eq!(status(&s), "none");
        create_vault(&s, "ana", "senha-2").unwrap();
        lock_vault(&s);
        assert_ne!(s.vault_path().parent().unwrap(), s.dir);
        let names: Vec<_> = list(&s).into_iter().map(|p| (p.nickname, p.active)).collect();
        assert_eq!(names, [("henri".into(), false), ("ana".into(), true)]);
        // cada senha só abre o próprio cofre
        assert_eq!(unlock_vault(&s, "senha-1").unwrap_err(), "WRONG_PASSWORD");
        unlock_vault(&s, "senha-2").unwrap();
        lock_vault(&s);
        select(&s, MAIN).unwrap();
        assert_eq!(vault_nickname_of(&s).unwrap().as_deref(), Some("henri"));
        unlock_vault(&s, "senha-1").unwrap();
        lock_vault(&s);
        select(&s, &id).unwrap();
        assert_eq!(vault_nickname_of(&s).unwrap().as_deref(), Some("ana"));
    }

    #[test]
    fn cannot_switch_while_a_vault_is_open() {
        let (_d, s) = state();
        create_vault(&s, "henri", "x").unwrap();
        assert_eq!(new_profile(&s).unwrap_err(), "UNLOCKED");
        assert_eq!(select(&s, MAIN).unwrap_err(), "UNLOCKED");
    }

    #[test]
    fn unknown_profile_is_refused() {
        let (_d, s) = state();
        assert_eq!(select(&s, "../../etc").unwrap_err(), "NO_PROFILE");
        assert_eq!(select(&s, "nao-existe").unwrap_err(), "NO_PROFILE");
    }

    #[test]
    fn abandoned_empty_profile_is_removed_when_switching_back() {
        let (_d, s) = state();
        create_vault(&s, "henri", "x").unwrap();
        lock_vault(&s);
        new_profile(&s).unwrap();
        let empty = s.vault_path().parent().unwrap().to_path_buf();
        select(&s, MAIN).unwrap();
        assert!(!empty.exists());
        assert_eq!(list(&s).len(), 1);
    }

    #[test]
    fn last_profile_is_remembered_across_restarts() {
        let (d, s) = state();
        create_vault(&s, "henri", "x").unwrap();
        lock_vault(&s);
        new_profile(&s).unwrap();
        create_vault(&s, "ana", "y").unwrap();
        lock_vault(&s);
        let again = AppState::new(d.path().to_path_buf());
        restore_last(&again);
        assert_eq!(vault_nickname_of(&again).unwrap().as_deref(), Some("ana"));
    }
}
