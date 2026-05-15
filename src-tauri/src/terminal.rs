use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use serde::Serialize;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Mutex;
use tauri::Emitter;

// ── Session state ─────────────────────────────────────────────────────────────

enum TermCmd {
    Input(Vec<u8>),
    Resize(u16, u16),
    Close,
}

struct SessionHandle {
    tx: std::sync::mpsc::SyncSender<TermCmd>,
}

pub struct TerminalState {
    sessions: Mutex<HashMap<String, SessionHandle>>,
}

impl TerminalState {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }
}

// ── Commands ──────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct TermAvailability {
    pub available: bool,
    pub message: String,
}

#[tauri::command]
pub fn check_terminal_available() -> TermAvailability {
    TermAvailability {
        available: true,
        message: "Ready".into(),
    }
}

#[tauri::command]
pub async fn spawn_terminal(
    app: tauri::AppHandle,
    term_state: tauri::State<'_, TerminalState>,
    session_id: String,
    cwd: Option<String>,
    shell: Option<String>,
) -> Result<(), String> {
    let shell_cmd = shell.unwrap_or_else(|| {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
    });
    let cwd_path = cwd.unwrap_or_else(|| {
        dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string())
    });

    let (tx, rx) = std::sync::mpsc::sync_channel::<TermCmd>(64);

    // Only Send types cross the thread boundary.
    // The PTY itself is created *inside* the thread to avoid !Send on Box<dyn MasterPty>.
    let app_clone = app.clone();
    let sid = session_id.clone();

    std::thread::spawn(move || {
        // Create PTY inside the thread — never crosses a thread boundary.
        let pty_system = native_pty_system();
        let pair = match pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        }) {
            Ok(p) => p,
            Err(e) => {
                let _ = app_clone.emit(
                    "terminal-exit",
                    serde_json::json!({ "session_id": &sid, "code": -1, "message": e.to_string() }),
                );
                return;
            }
        };

        let mut cmd = CommandBuilder::new(&shell_cmd);
        cmd.cwd(&cwd_path);
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        for var in &["HOME", "USER", "PATH", "LANG", "LOGNAME"] {
            if let Ok(v) = std::env::var(var) {
                cmd.env(var, v);
            }
        }

        let _child = match pair.slave.spawn_command(cmd) {
            Ok(c) => c,
            Err(e) => {
                let _ = app_clone.emit(
                    "terminal-exit",
                    serde_json::json!({ "session_id": &sid, "code": -1, "message": format!("spawn failed: {e}") }),
                );
                return;
            }
        };
        drop(pair.slave);

        // reader and writer are Box<dyn Read/Write + Send> — explicitly Send.
        let mut reader = match pair.master.try_clone_reader() {
            Ok(r) => r,
            Err(e) => {
                let _ = app_clone.emit(
                    "terminal-exit",
                    serde_json::json!({ "session_id": &sid, "code": -1, "message": e.to_string() }),
                );
                return;
            }
        };
        let mut writer = match pair.master.take_writer() {
            Ok(w) => w,
            Err(e) => {
                let _ = app_clone.emit(
                    "terminal-exit",
                    serde_json::json!({ "session_id": &sid, "code": -1, "message": e.to_string() }),
                );
                return;
            }
        };

        // Reader sub-thread — only captures Send types.
        let app_reader = app_clone.clone();
        let sid_reader = sid.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        let data = String::from_utf8_lossy(&buf[..n]).into_owned();
                        let _ = app_reader.emit(
                            "terminal-output",
                            serde_json::json!({ "session_id": &sid_reader, "data": data }),
                        );
                    }
                }
            }
            let _ = app_reader.emit(
                "terminal-exit",
                serde_json::json!({ "session_id": &sid_reader, "code": 0 }),
            );
        });

        // Command loop — master stays here for resize, writer for input.
        for cmd in rx.iter() {
            match cmd {
                TermCmd::Input(data) => {
                    let _ = writer.write_all(&data);
                }
                TermCmd::Resize(cols, rows) => {
                    let _ = pair.master.resize(PtySize {
                        rows,
                        cols,
                        pixel_width: 0,
                        pixel_height: 0,
                    });
                }
                TermCmd::Close => break,
            }
        }
    });

    term_state
        .sessions
        .lock()
        .unwrap()
        .insert(session_id, SessionHandle { tx });
    Ok(())
}

#[tauri::command]
pub async fn send_terminal_input(
    term_state: tauri::State<'_, TerminalState>,
    session_id: String,
    data: String,
) -> Result<(), String> {
    let sessions = term_state.sessions.lock().unwrap();
    let s = sessions.get(&session_id).ok_or("Session not found")?;
    s.tx.send(TermCmd::Input(data.into_bytes()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn resize_terminal(
    term_state: tauri::State<'_, TerminalState>,
    session_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let sessions = term_state.sessions.lock().unwrap();
    let s = sessions.get(&session_id).ok_or("Session not found")?;
    s.tx.send(TermCmd::Resize(cols, rows))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn close_terminal(
    term_state: tauri::State<'_, TerminalState>,
    session_id: String,
) -> Result<(), String> {
    if let Some(s) = term_state.sessions.lock().unwrap().remove(&session_id) {
        let _ = s.tx.send(TermCmd::Close);
    }
    Ok(())
}
