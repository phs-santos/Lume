pub mod commands;
pub mod db;
pub mod state;
pub mod vault;

use tauri::Manager;

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
            commands::book_list,
            commands::book_create,
            commands::book_rename,
            commands::book_set_mode,
            commands::book_outline,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
