mod api;
mod db;

use db::Database;
use std::sync::Mutex;

pub struct AppState {
    pub db: Mutex<Database>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db = Database::new().expect("Failed to initialize database");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            db: Mutex::new(db),
        })
        .invoke_handler(tauri::generate_handler![
            api::send_message,
            api::stop_generation,
            api::generate_title,
            db::get_conversations,
            db::get_messages,
            db::create_conversation,
            db::delete_conversation,
            db::rename_conversation,
            db::get_api_key,
            db::set_api_key,
            db::get_model,
            db::set_model,
            db::get_system_prompt,
            db::set_system_prompt,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
