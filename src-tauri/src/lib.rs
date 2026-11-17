pub mod ai;
pub mod commands;
pub mod db;
pub mod import;
pub mod remote;
pub mod state;
pub mod vault;

use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{Emitter, Manager};

/// Fica `true` quando a interface confirmou que salvou tudo e pediu para sair.
static EXIT_READY: AtomicBool = AtomicBool::new(false);

#[tauri::command]
fn exit_app(app: tauri::AppHandle) {
    EXIT_READY.store(true, Ordering::SeqCst);
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            app.manage(state::AppState::new(dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::vault_status,
            commands::vault_create,
            commands::vault_unlock,
            commands::vault_recover,
            commands::vault_lock,
            commands::vault_nickname,
            commands::book_list,
            commands::book_create,
            commands::book_rename,
            commands::book_set_mode,
            commands::book_outline,
            commands::book_import_epub,
            commands::chapter_create,
            commands::chapter_rename,
            commands::chapter_move,
            commands::chapter_delete,
            commands::scene_create,
            commands::scene_get,
            commands::scene_rename,
            commands::scene_save,
            commands::scene_move,
            commands::scene_delete,
            commands::scene_versions,
            commands::scene_restore,
            commands::setting_get,
            commands::setting_set,
            commands::remote_status,
            commands::remote_connect,
            commands::remote_disconnect,
            commands::remote_sync,
            commands::inbox_list,
            commands::inbox_body,
            commands::inbox_archive,
            commands::inbox_restore,
            commands::inbox_delete,
            commands::inbox_to_scene,
            commands::ai_key_set,
            commands::ai_key_status,
            commands::ai_models_get,
            commands::ai_models_set,
            commands::ai_usage,
            commands::ai_enable,
            commands::ai_conversation,
            commands::ai_conversation_new,
            commands::ai_conversations,
            commands::ai_conversation_open,
            commands::ai_send,
            commands::ai_retry,
            commands::ai_scene_touched,
            commands::ai_first_pass,
            commands::ai_first_pass_cancel,
            commands::ai_pending_count,
            commands::bible_list,
            commands::bible_update,
            commands::bible_absorb_note,
            commands::bible_delete,
            commands::proposal_set_status,
            commands::proposals_pending,
            exit_app,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // Cmd+Q / sair pelo menu: antes de encerrar, pede à interface que grave o texto pendente.
            // A interface responde chamando `exit_app` (ou não, se o salvamento falhar).
            if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
                let user_quit = code.is_none();
                if user_quit && !EXIT_READY.load(Ordering::SeqCst) && !app.webview_windows().is_empty() {
                    api.prevent_exit();
                    let _ = app.emit("flush-now", ());
                }
            }
        });
}
