mod api;
mod db;
mod dbus_service;
mod mcp;
mod providers;

use chrono;
use db::Database;
use std::sync::Mutex;
use tauri::{
    Emitter,
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
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_deep_link::init())
        .manage(AppState {
            db: Mutex::new(db),
        })
        .setup(|app| {
            let show = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Linux Claude Desktop")
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

            // Start DBus service for external scripting
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                dbus_service::start_dbus_service(app_handle).await;
            });

            // Start scheduled prompts background task
            let app_handle2 = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                    let state = app_handle2.state::<AppState>();
                    let due = {
                        let db = state.db.lock().unwrap();
                        db.get_due_scheduled_prompts().unwrap_or_default()
                    };
                    for sp in &due {
                        // Emit event to frontend to execute the prompt
                        let _ = app_handle2.emit("scheduled-prompt", serde_json::json!({
                            "id": sp.id,
                            "name": sp.name,
                            "prompt": sp.prompt,
                        }));
                        // Update last_run
                        let db = state.db.lock().unwrap();
                        let _ = db.update_scheduled_prompt_last_run(&sp.id);
                    }
                }
            });

            // Start file watcher background task (checks every 30s)
            let app_handle3 = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                    let state = app_handle3.state::<AppState>();
                    let watches = {
                        let db = state.db.lock().unwrap();
                        db.list_file_watches().unwrap_or_default()
                    };
                    for watch in &watches {
                        let path = std::path::Path::new(&watch.file_path);
                        if !path.exists() {
                            continue;
                        }
                        let current_modified = std::fs::metadata(path).ok()
                            .and_then(|m| m.modified().ok())
                            .map(|t| {
                                let dt: chrono::DateTime<chrono::Utc> = t.into();
                                dt.to_rfc3339()
                            });
                        let changed = match (&watch.last_modified, &current_modified) {
                            (Some(old), Some(new)) => old != new,
                            (None, Some(_)) => true,
                            _ => false,
                        };
                        if changed {
                            if let Ok(content) = std::fs::read_to_string(path) {
                                let content = if content.len() > 50_000 { content[..50_000].to_string() } else { content };
                                let db = state.db.lock().unwrap();
                                let _ = db.update_knowledge_content(&watch.knowledge_id, &content);
                                if let Some(ref modified) = current_modified {
                                    let _ = db.update_file_watch_modified(&watch.id, modified);
                                }
                            }
                        }
                    }
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    // Hide main window to tray instead of quitting
                    let _ = window.hide();
                    api.prevent_close();
                }
                // Other windows (quickask) close normally
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
            api::get_app_info,
            api::run_custom_command,
            api::capture_screenshot,
            api::popout_conversation,
            api::toggle_quickask,
            providers::fetch_ollama_models,
            providers::test_custom_endpoint,
            db::get_conversations,
            db::get_messages,
            db::get_messages_paginated,
            db::get_message_count,
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
            db::get_scheduled_prompts,
            db::create_scheduled_prompt,
            db::update_scheduled_prompt,
            db::delete_scheduled_prompt,
            db::fork_conversation,
            db::get_conversation_usage,
            db::get_total_usage,
            db::backup_database,
            db::restore_database,
            db::get_database_path,
            db::get_database_size,
            db::create_artifact,
            db::get_artifacts,
            db::get_artifact_content,
            db::get_artifact_versions,
            db::save_artifact_version,
            db::update_artifact_title,
            db::delete_artifact,
            db::save_artifact_to_file,
            db::open_artifact_external,
            db::get_custom_endpoint_id,
            db::set_custom_endpoint_id,
            db::get_custom_endpoints,
            db::create_custom_endpoint,
            db::update_custom_endpoint,
            db::delete_custom_endpoint,
            db::get_model_pricing,
            db::set_model_pricing,
            db::delete_model_pricing,
            db::get_routing_rules,
            db::create_routing_rule,
            db::update_routing_rule,
            db::delete_routing_rule,
            db::get_comparison_sessions,
            db::get_comparison_responses,
            db::rate_comparison_response,
            db::delete_comparison_session,
            db::get_conversation_cost,
            db::get_cost_summary,
            db::get_memory_entries,
            db::save_memory_entry,
            db::delete_memory_entry,
            db::get_knowledge_entries,
            db::create_knowledge_entry,
            db::update_knowledge_entry,
            db::delete_knowledge_entry,
            db::toggle_knowledge_entry,
            db::import_file_to_knowledge,
            db::get_file_watches,
            db::delete_file_watch,
            api::send_comparison,
            api::import_url,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
