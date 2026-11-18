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
/// Preferências que a webview nunca lê nem escreve pelo comando genérico (a chave de IA).
const PRIVATE_SETTINGS: &[&str] = &["ai.inworld_key"];

pub fn guarded_setting_get(s: &AppState, key: &str) -> CmdResult<Option<String>> {
    if PRIVATE_SETTINGS.contains(&key) {
        return Err("FORBIDDEN".into());
    }
    with_db(s, |c| settings::setting_get(c, key))
}

pub fn guarded_setting_set(s: &AppState, key: &str, value: &str) -> CmdResult<()> {
    if PRIVATE_SETTINGS.contains(&key) {
        return Err("FORBIDDEN".into());
    }
    with_db(s, |c| settings::setting_set(c, key, value))
}

#[tauri::command]
pub fn setting_get(state: State<AppState>, key: String) -> CmdResult<Option<String>> {
    guarded_setting_get(&state, &key)
}

#[tauri::command]
pub fn setting_set(state: State<AppState>, key: String, value: String) -> CmdResult<()> {
    guarded_setting_set(&state, &key, &value)
}

// ---------- celular e caixa de entrada ----------
use crate::db::inbox::{self, InboxItem};
use crate::remote::{self, client::HttpApi, sync, SharedLocal};

#[derive(serde::Serialize)]
pub struct RemoteStatus {
    pub connected: bool,
    pub nickname: Option<String>,
    pub url: Option<String>,
}

#[tauri::command]
pub fn remote_status(state: State<AppState>) -> CmdResult<RemoteStatus> {
    let g = state.db.lock().unwrap();
    let c = g.as_ref().ok_or_else(|| "LOCKED".to_string())?;
    let cfg = remote::load_config(c).map_err(err)?;
    Ok(RemoteStatus { connected: cfg.is_some(), nickname: cfg.as_ref().map(|x| x.nickname.clone()), url: cfg.map(|x| x.url) })
}

/// Entra na conta do servidor (ou cadastra com convite) e publica a chave pública deste computador.
#[tauri::command]
pub async fn remote_connect(
    state: State<'_, AppState>,
    url: String,
    nickname: String,
    password: String,
    invite: Option<String>,
) -> CmdResult<()> {
    let kp = {
        let g = state.db.lock().unwrap();
        let c = g.as_ref().ok_or_else(|| "LOCKED".to_string())?;
        remote::load_keypair(c).map_err(err)?.unwrap_or_else(remote::crypto::generate_keypair)
    };
    let (cfg, kp) = tauri::async_runtime::spawn_blocking(move || remote::connect(&url, &nickname, &password, invite.as_deref(), kp))
        .await
        .map_err(err)?
        .map_err(err)?;
    let g = state.db.lock().unwrap();
    let c = g.as_ref().ok_or_else(|| "LOCKED".to_string())?;
    remote::save_connection(c, &cfg, &kp).map_err(err)
}

#[tauri::command]
pub fn remote_disconnect(state: State<AppState>) -> CmdResult<()> {
    let g = state.db.lock().unwrap();
    let c = g.as_ref().ok_or_else(|| "LOCKED".to_string())?;
    remote::forget_session(c).map_err(err)
}

/// Importa um .epub do disco como livro novo.
#[tauri::command]
pub async fn book_import_epub(state: State<'_, AppState>, path: String, ai_mode: String) -> CmdResult<Book> {
    let parsed = tauri::async_runtime::spawn_blocking(move || {
        let bytes = std::fs::read(&path).map_err(|e| format!("Não consegui ler o arquivo: {e}"))?;
        crate::import::epub::parse_epub(&bytes).map_err(err)
    })
    .await
    .map_err(err)??;
    with_db(&state, |c| crate::import::import_book(c, &parsed, &ai_mode, now_ms()))
}

/// Transcreve os áudios pendentes da caixa de ideias (um por vez, em segundo plano).
#[tauri::command]
pub async fn inbox_transcribe_pending(state: State<'_, AppState>) -> CmdResult<usize> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let key = {
            let g = db.lock().unwrap();
            let c = g.as_ref().ok_or_else(|| "LOCKED".to_string())?;
            settings::setting_get(c, "ai.inworld_key").map_err(err)?.filter(|k| !k.trim().is_empty())
        };
        let stt = key.map(|key| crate::ai::stt::InworldStt { key });
        crate::ai::transcribe::transcribe_pending(&db, stt.as_ref().map(|t| t as &dyn crate::ai::stt::Transcriber), now_ms())
    })
    .await
    .map_err(err)?
}

db_cmd!(inbox_retry_transcript(id: String) -> () => |c| crate::db::inbox::inbox_retry_transcript(c, &id));

/// Só uma sincronização por vez (o painel e o ciclo de fundo podem pedir juntos).
static SYNCING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Busca as ideias novas no servidor. Devolve quantas chegaram.
#[tauri::command]
pub async fn remote_sync(state: State<'_, AppState>) -> CmdResult<usize> {
    use std::sync::atomic::Ordering;
    if SYNCING.swap(true, Ordering::SeqCst) {
        return Ok(0);
    }
    let db = state.db.clone();
    let r = tauri::async_runtime::spawn_blocking(move || {
        let (cfg, kp) = {
            let g = db.lock().unwrap();
            let c = g.as_ref().ok_or_else(|| "LOCKED".to_string())?;
            let cfg = remote::load_config(c).map_err(err)?.ok_or_else(|| "NOT_CONNECTED".to_string())?;
            let kp = remote::load_keypair(c).map_err(err)?.ok_or_else(|| "NOT_CONNECTED".to_string())?;
            (cfg, kp)
        };
        let api = HttpApi { url: cfg.url, token: cfg.token };
        sync::sync(&SharedLocal(db.clone()), &api, &kp, now_ms()).map_err(|e| {
            let g = db.lock().unwrap();
            match g.as_ref() {
                Some(c) => remote::sync_error_code(c, &e),
                None => e.to_string(),
            }
        })
    })
    .await
    .map_err(err);
    SYNCING.store(false, Ordering::SeqCst);
    r?
}

/// Áudio vai como bytes crus (não como lista JSON de números), para não travar a tela.
#[tauri::command]
pub fn inbox_body(state: State<AppState>, id: String) -> CmdResult<tauri::ipc::Response> {
    with_db(&state, |c| inbox::inbox_body(c, &id)).map(tauri::ipc::Response::new)
}

db_cmd!(inbox_list() -> Vec<InboxItem> => |c| inbox::inbox_list(c));
db_cmd!(inbox_archive(id: String) -> () => |c| inbox::inbox_set_status(c, &id, "arquivada"));
db_cmd!(inbox_restore(id: String) -> () => |c| inbox::inbox_set_status(c, &id, "nova"));
db_cmd!(inbox_delete(id: String) -> () => |c| inbox::inbox_delete(c, &id));
db_cmd!(inbox_to_scene(id: String, chapter_id: String, index: Option<i64>) -> Scene => |c| inbox::inbox_to_scene(c, &id, &chapter_id, index, now_ms()));

// ---------- assistente de IA ----------
use crate::ai::{background, chat, provider::Inworld};
use crate::db::ai::{self as aidb, BibleEntry, Conversation, Message, Proposal, UsageRow};

pub fn ai_key_save(s: &AppState, key: &str) -> CmdResult<()> {
    let key = key.trim();
    if key.is_empty() {
        return Err("Chave vazia".into());
    }
    with_db(s, |c| {
        settings::setting_set(c, "ai.inworld_key", key)?;
        // áudios que esperavam a chave, ou que falharam por chave/crédito, voltam para a fila
        use crate::ai::provider::ProviderError;
        crate::db::inbox::inbox_wake_no_key(c)?;
        crate::db::inbox::inbox_wake_errors(c, &[ProviderError::Auth.to_string(), ProviderError::Quota.to_string()]).map(|_| ())
    })
}

pub fn ai_key_present(s: &AppState) -> CmdResult<bool> {
    with_db(s, |c| Ok(settings::setting_get(c, "ai.inworld_key")?.is_some_and(|k| !k.is_empty())))
}

#[derive(Debug, serde::Serialize)]
pub struct AiModels {
    pub chat: String,
    pub background: String,
}

pub fn ai_models(s: &AppState) -> CmdResult<AiModels> {
    with_db(s, |c| {
        let cfg = chat::AiConfig::load(c)?;
        Ok(AiModels { chat: cfg.chat_model, background: cfg.background_model })
    })
}

/// Lê a chave dentro do Rust. Sem chave: mensagem pronta para a interface.
fn inworld(db: &crate::state::SharedDb) -> Result<Inworld, String> {
    let g = db.lock().unwrap();
    let c = g.as_ref().ok_or_else(|| "LOCKED".to_string())?;
    let key = settings::setting_get(c, "ai.inworld_key").map_err(err)?.filter(|k| !k.is_empty());
    key.map(|key| Inworld { key }).ok_or_else(|| "Configure a chave da Inworld em Ajustes".to_string())
}

#[tauri::command]
pub fn ai_key_set(state: State<AppState>, key: String) -> CmdResult<()> {
    ai_key_save(&state, &key)
}

#[tauri::command]
pub fn ai_key_status(state: State<AppState>) -> CmdResult<bool> {
    ai_key_present(&state)
}

#[tauri::command]
pub fn ai_models_get(state: State<AppState>) -> CmdResult<AiModels> {
    ai_models(&state)
}

#[tauri::command]
pub fn ai_models_set(state: State<AppState>, chat: String, background: String) -> CmdResult<()> {
    with_db(&state, |c| {
        settings::setting_set(c, "ai.chat_model", chat.trim())?;
        settings::setting_set(c, "ai.background_model", background.trim())
    })
}

db_cmd!(ai_usage(month: String) -> Vec<UsageRow> => |c| aidb::usage_month(c, &month));
db_cmd!(ai_enable(book_id: String) -> () => |c| settings::setting_set(c, &format!("ai.enabled.{book_id}"), "1"));

#[derive(serde::Serialize)]
pub struct ConversationView {
    pub conversation: Conversation,
    pub messages: Vec<Message>,
}

fn conversation_view(c: &rusqlite::Connection, cv: Conversation) -> Result<ConversationView, db::DbError> {
    let messages = aidb::messages_all(c, &cv.id)?;
    Ok(ConversationView { conversation: cv, messages })
}

db_cmd!(ai_conversation(book_id: String) -> ConversationView => |c| { let cv = aidb::conversation_current(c, &book_id, now_ms())?; conversation_view(c, cv) });
db_cmd!(ai_conversation_new(book_id: String) -> ConversationView => |c| { let cv = aidb::conversation_new(c, &book_id, now_ms())?; conversation_view(c, cv) });
db_cmd!(ai_conversations(book_id: String) -> Vec<Conversation> => |c| aidb::conversation_list(c, &book_id));
db_cmd!(ai_conversation_open(id: String) -> ConversationView => |c| { let cv = aidb::conversation_open(c, &id)?; conversation_view(c, cv) });
db_cmd!(bible_list(book_id: String) -> Vec<BibleEntry> => |c| aidb::bible_list(c, &book_id));
db_cmd!(bible_update(id: String, name: String, aliases: Vec<String>, body: String, story_order: Option<i64>) -> BibleEntry => |c| aidb::bible_update_user(c, &id, &name, &aliases, &body, story_order, now_ms()));
db_cmd!(bible_absorb_note(id: String, index: usize) -> BibleEntry => |c| aidb::bible_absorb_note(c, &id, index, now_ms()));
db_cmd!(bible_delete(id: String) -> () => |c| aidb::bible_delete(c, &id));
db_cmd!(proposal_set_status(id: String, status: String) -> () => |c| aidb::proposal_set_status(c, &id, &status));
db_cmd!(proposals_pending(book_id: String) -> Vec<Proposal> => |c| aidb::proposals_pending(c, &book_id));

/// Envia uma mensagem e transmite a resposta pelo canal.
#[tauri::command]
pub async fn ai_send(state: State<'_, AppState>, book_id: String, scene_id: Option<String>, text: String, on_event: tauri::ipc::Channel<chat::ChatEvent>) -> CmdResult<()> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut emit = |e: chat::ChatEvent| {
            let _ = on_event.send(e);
        };
        let model = match inworld(&db) {
            Ok(m) => m,
            Err(message) if text.trim().is_empty() => return Err(message),
            Err(message) => {
                // grava a pergunta mesmo assim, para não se perder
                let _ = with_db_shared(&db, |c| {
                    let cv = aidb::conversation_current(c, &book_id, now_ms())?;
                    aidb::message_add(c, &cv.id, "user", text.trim(), None, now_ms()).map(|_| ())
                });
                emit(chat::ChatEvent::Error { message: message.clone() });
                return Err(message);
            }
        };
        chat::send(&db, &model, &book_id, scene_id.as_deref(), &text, &mut emit, &now_ms)
    })
    .await
    .map_err(err)?
}

#[tauri::command]
pub async fn ai_retry(state: State<'_, AppState>, book_id: String, scene_id: Option<String>, on_event: tauri::ipc::Channel<chat::ChatEvent>) -> CmdResult<()> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut emit = |e: chat::ChatEvent| {
            let _ = on_event.send(e);
        };
        let model = inworld(&db).inspect_err(|m| emit(chat::ChatEvent::Error { message: m.clone() }))?;
        chat::retry(&db, &model, &book_id, scene_id.as_deref(), &mut emit, &now_ms)
    })
    .await
    .map_err(err)?
}

fn with_db_shared<T>(db: &crate::state::SharedDb, f: impl FnOnce(&rusqlite::Connection) -> Result<T, db::DbError>) -> CmdResult<T> {
    let g = db.lock().unwrap();
    let c = g.as_ref().ok_or_else(|| "LOCKED".to_string())?;
    f(c).map_err(err)
}

/// Livro com o Assistente ligado e chave configurada?
fn ai_ready_for_scene(db: &crate::state::SharedDb, scene_id: &str) -> Option<Inworld> {
    let book: String = with_db_shared(db, |c| {
        let ch = scenes::scene_get(c, scene_id)?.chapter_id;
        Ok(c.query_row("SELECT book_id FROM chapters WHERE id=?1", [ch], |r| r.get(0))?)
    })
    .ok()?;
    let enabled = with_db_shared(db, |c| settings::setting_get(c, &format!("ai.enabled.{book}"))).ok()??;
    (enabled == "1").then_some(())?;
    inworld(db).ok()
}

static DIGESTING: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// A cena mudou (a pessoa saiu dela ou parou de digitar): atualiza resumo e Bíblia em segundo plano.
#[tauri::command]
pub fn ai_scene_touched(state: State<AppState>, scene_id: String) {
    let db = state.db.clone();
    {
        let mut d = DIGESTING.lock().unwrap_or_else(|e| e.into_inner());
        if d.contains(&scene_id) {
            return;
        }
        d.push(scene_id.clone());
    }
    std::thread::spawn(move || {
        // tira a cena da lista mesmo se algo der pânico no meio
        struct Done(String);
        impl Drop for Done {
            fn drop(&mut self) {
                DIGESTING.lock().unwrap_or_else(|e| e.into_inner()).retain(|x| x != &self.0);
            }
        }
        let _done = Done(scene_id.clone());
        if let Some(model) = ai_ready_for_scene(&db, &scene_id) {
            let _ = background::digest_scene(&db, &model, &scene_id, &now_ms);
        }
    });
}

static FIRST_PASS_RUNNING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static FIRST_PASS_CANCEL: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[derive(Clone, serde::Serialize)]
pub struct Progress {
    pub done: usize,
    pub total: usize,
}

/// Lê o livro inteiro (só cenas pendentes), uma por vez. Só uma passada por vez.
#[tauri::command]
pub async fn ai_first_pass(state: State<'_, AppState>, book_id: String, on_progress: tauri::ipc::Channel<Progress>) -> CmdResult<()> {
    use std::sync::atomic::Ordering;
    if FIRST_PASS_RUNNING.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    FIRST_PASS_CANCEL.store(false, Ordering::SeqCst);
    let db = state.db.clone();
    let r = tauri::async_runtime::spawn_blocking(move || {
        let model = inworld(&db)?;
        background::run_queue(&db, &model, &book_id, &FIRST_PASS_CANCEL, &mut |done, total| {
            let _ = on_progress.send(Progress { done, total });
        }, &now_ms)
    })
    .await
    .map_err(err);
    FIRST_PASS_RUNNING.store(false, Ordering::SeqCst);
    r?
}

#[tauri::command]
pub fn ai_first_pass_cancel() {
    FIRST_PASS_CANCEL.store(true, std::sync::atomic::Ordering::SeqCst);
}

db_cmd!(ai_pending_count(book_id: String) -> usize => |c| background::pending_scenes(c, &book_id).map(|v| v.len()));

#[cfg(test)]
mod tests {
    #[test]
    fn saving_ai_key_wakes_audio_waiting_for_key() {
        let (_d, s) = state();
        create_vault(&s, "ana", "x").unwrap();
        with_db(&s, |c| {
            crate::db::inbox::inbox_insert(c, "a", "audio", "audio/mp4", &[1], 1, 1)?;
            crate::db::inbox::inbox_mark_no_key(c).map(|_| ())
        })
        .unwrap();
        ai_key_save(&s, "chave-teste").unwrap();
        let st = with_db(&s, |c| crate::db::inbox::inbox_list(c)).unwrap()[0].transcript_status.clone();
        assert_eq!(st.as_deref(), Some("pendente"));
    }

    #[test]
    fn saving_ai_key_retries_audio_that_failed_for_key_or_credit() {
        use crate::ai::provider::ProviderError;
        use crate::db::inbox;
        let (_d, s) = state();
        create_vault(&s, "ana", "x").unwrap();
        with_db(&s, |c| {
            for (id, err) in [("auth", ProviderError::Auth.to_string()), ("quota", ProviderError::Quota.to_string()), ("outro", "500 erro do servidor".into())] {
                inbox::inbox_insert(c, id, "audio", "audio/mp4", &[1], 1, 1)?;
                inbox::inbox_set_transcript(c, id, "erro", None, Some(&err))?;
            }
            Ok(())
        })
        .unwrap();
        ai_key_save(&s, "chave-nova").unwrap();
        let st: std::collections::HashMap<_, _> = with_db(&s, inbox::inbox_list).unwrap().into_iter().map(|i| (i.id, i.transcript_status.unwrap())).collect();
        assert_eq!(st["auth"], "pendente");
        assert_eq!(st["quota"], "pendente");
        assert_eq!(st["outro"], "erro");
    }

    use super::*;

    #[test]
    fn ai_key_never_returns_to_webview() {
        let (_d, s) = state();
        create_vault(&s, "ana", "x").unwrap();
        assert!(!ai_key_present(&s).unwrap());
        ai_key_save(&s, "  SEGREDO-123  ").unwrap();
        assert!(ai_key_present(&s).unwrap());
        assert_eq!(guarded_setting_get(&s, "ai.inworld_key").unwrap_err(), "FORBIDDEN");
        assert_eq!(guarded_setting_set(&s, "ai.inworld_key", "x").unwrap_err(), "FORBIDDEN");
        let models = ai_models(&s).unwrap();
        assert!(!format!("{models:?}").contains("SEGREDO"));
        // a chave fica salva sem espaços em volta
        assert_eq!(with_db(&s, |c| settings::setting_get(c, "ai.inworld_key")).unwrap().as_deref(), Some("SEGREDO-123"));
        assert_eq!(guarded_setting_get(&s, "theme").unwrap(), None);
    }

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
