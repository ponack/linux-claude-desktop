mod api;
mod db;
mod mcp;
mod providers;

use db::Database;
use std::sync::Mutex;
use tauri::{
    Manager,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};

pub struct AppState {
    pub db: Mutex<Database>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db = Database::new().expect("Failed to initialize database");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(AppState {
            db: Mutex::new(db),
        })
        .setup(|app| {
            let show = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Ubuntu Claude Desktop")
                .menu(&menu)
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, .. } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Hide to tray instead of quitting
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            api::send_message,
            api::stop_generation,
            api::generate_title,
            api::check_for_updates,
            api::download_update,
            api::install_update,
            api::restart_app,
            api::run_custom_command,
            providers::fetch_ollama_models,
            db::get_conversations,
            db::get_messages,
            db::create_conversation,
            db::delete_conversation,
            db::delete_messages_from,
            db::rename_conversation,
            db::get_api_key,
            db::set_api_key,
            db::get_model,
            db::set_model,
            db::get_theme,
            db::set_theme,
            db::get_system_prompt,
            db::set_system_prompt,
            db::get_mcp_servers,
            db::set_mcp_servers,
            db::export_conversation,
            db::get_projects,
            db::create_project,
            db::update_project,
            db::delete_project,
            db::set_conversation_project,
            db::get_conversation_project,
            db::get_provider,
            db::set_provider,
            db::get_openai_api_key,
            db::set_openai_api_key,
            db::get_openai_base_url,
            db::set_openai_base_url,
            db::get_ollama_base_url,
            db::set_ollama_base_url,
            db::get_custom_css,
            db::set_custom_css,
            db::get_prompts,
            db::create_prompt,
            db::update_prompt,
            db::delete_prompt,
            db::get_custom_commands,
            db::set_custom_commands,
            db::get_update_interval,
            db::set_update_interval,
            db::get_skipped_version,
            db::set_skipped_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
