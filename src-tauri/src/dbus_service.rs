use tauri::{AppHandle, Emitter, Manager};
use zbus::{connection, interface};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct LcdDbusService {
    app: Arc<Mutex<AppHandle>>,
}

#[interface(name = "com.linux_claude_desktop.App")]
impl LcdDbusService {
    /// Show the main window
    async fn show(&self) {
        let app = self.app.lock().await;
        if let Some(win) = app.get_webview_window("main") {
            let _ = win.show();
            let _ = win.set_focus();
        }
    }

    /// Hide the main window
    async fn hide(&self) {
        let app = self.app.lock().await;
        if let Some(win) = app.get_webview_window("main") {
            let _ = win.hide();
        }
    }

    /// Toggle the quick-ask overlay
    async fn quick_ask(&self) {
        let app = self.app.lock().await;
        let _ = crate::api::toggle_quickask(app.clone()).await;
    }

    /// Send a question via deep link mechanism (opens new chat and sends)
    async fn ask(&self, question: &str) {
        let app = self.app.lock().await;
        let _ = app.emit("dbus-ask", question.to_string());
        if let Some(win) = app.get_webview_window("main") {
            let _ = win.show();
            let _ = win.set_focus();
        }
    }
}

pub async fn start_dbus_service(app: AppHandle) {
    let service = LcdDbusService {
        app: Arc::new(Mutex::new(app)),
    };

    match connection::Builder::session()
        .expect("Failed to create DBus session builder")
        .name("com.linux_claude_desktop.App")
        .expect("Failed to set DBus name")
        .serve_at("/com/linux_claude_desktop/App", service)
        .expect("Failed to serve DBus interface")
        .build()
        .await
    {
        Ok(conn) => {
            // Keep the connection alive
            std::mem::forget(conn);
        }
        Err(e) => {
            eprintln!("Failed to start DBus service: {}", e);
        }
    }
}
