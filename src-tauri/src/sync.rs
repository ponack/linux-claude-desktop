use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

// ── Config types ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncConfig {
    pub enabled: bool,
    pub repo_path: String,
    pub auto_sync_interval_mins: u32, // 0 = manual only
    pub commit_name: String,
    pub commit_email: String,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            repo_path: String::new(),
            auto_sync_interval_mins: 0,
            commit_name: "LCD Sync".to_string(),
            commit_email: "sync@linux-claude-desktop.local".to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SyncResult {
    pub pushed: usize,
    pub pulled: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
    pub timestamp: String,
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_sync_config(state: tauri::State<crate::AppState>) -> SyncConfig {
    let db = state.db.lock().unwrap();
    SyncConfig {
        enabled: db.get_sync_value("enabled").map(|v| v == "1").unwrap_or(false),
        repo_path: db.get_sync_value("repo_path").unwrap_or_default(),
        auto_sync_interval_mins: db.get_sync_value("auto_sync_interval_mins")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0),
        commit_name: db.get_sync_value("commit_name")
            .unwrap_or_else(|| "LCD Sync".to_string()),
        commit_email: db.get_sync_value("commit_email")
            .unwrap_or_else(|| "sync@linux-claude-desktop.local".to_string()),
    }
}

#[tauri::command]
pub fn set_sync_config(state: tauri::State<crate::AppState>, config: SyncConfig) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    db.set_sync_value("enabled", if config.enabled { "1" } else { "0" }).map_err(|e| e.to_string())?;
    db.set_sync_value("repo_path", &config.repo_path).map_err(|e| e.to_string())?;
    db.set_sync_value("auto_sync_interval_mins", &config.auto_sync_interval_mins.to_string()).map_err(|e| e.to_string())?;
    db.set_sync_value("commit_name", &config.commit_name).map_err(|e| e.to_string())?;
    db.set_sync_value("commit_email", &config.commit_email).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn init_sync_repo(path: String) -> Result<String, String> {
    let p = Path::new(&path);
    if !p.exists() {
        std::fs::create_dir_all(p).map_err(|e| format!("Cannot create directory: {e}"))?;
    }
    // Check if already a git repo
    let is_repo = Command::new("git")
        .args(["-C", &path, "rev-parse", "--git-dir"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !is_repo {
        let out = Command::new("git")
            .args(["-C", &path, "init"])
            .output()
            .map_err(|e| format!("git not found: {e}"))?;
        if !out.status.success() {
            return Err(String::from_utf8_lossy(&out.stderr).to_string());
        }
    }
    Ok(path)
}

#[tauri::command]
pub fn sync_push(state: tauri::State<crate::AppState>) -> Result<SyncResult, String> {
    let config = get_sync_config(state.clone());
    if config.repo_path.is_empty() {
        return Err("Sync repo path not configured".to_string());
    }
    do_push(&state, &config)
}

#[tauri::command]
pub fn sync_pull(state: tauri::State<crate::AppState>) -> Result<SyncResult, String> {
    let config = get_sync_config(state.clone());
    if config.repo_path.is_empty() {
        return Err("Sync repo path not configured".to_string());
    }
    do_pull(&state, &config)
}

#[tauri::command]
pub fn sync_now(state: tauri::State<crate::AppState>) -> Result<SyncResult, String> {
    let config = get_sync_config(state.clone());
    if config.repo_path.is_empty() {
        return Err("Sync repo path not configured".to_string());
    }
    // Pull first, then push (so we incorporate remote changes before committing)
    let mut pull_result = do_pull(&state, &config)?;
    let push_result = do_push(&state, &config)?;
    pull_result.pushed = push_result.pushed;
    pull_result.errors.extend(push_result.errors);
    Ok(pull_result)
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn run_git_in(args: &[&str], cwd: &str) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("git not found: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

fn do_push(state: &tauri::State<crate::AppState>, config: &SyncConfig) -> Result<SyncResult, String> {
    let repo = &config.repo_path;
    let conv_dir = format!("{}/conversations", repo);
    std::fs::create_dir_all(&conv_dir).map_err(|e| format!("Cannot create conversations dir: {e}"))?;

    let mut pushed = 0usize;
    let mut skipped = 0usize;
    let mut errors: Vec<String> = Vec::new();
    let timestamp = chrono::Utc::now().to_rfc3339();

    let (conversations, sync_states) = {
        let db = state.db.lock().unwrap();
        let convs = db.list_conversations().map_err(|e| e.to_string())?;
        let states = db.get_all_sync_states();
        (convs, states)
    };

    for conv in &conversations {
        let last_synced = sync_states.get(&conv.id).cloned();
        if last_synced.as_deref() == Some(&conv.updated_at) {
            skipped += 1;
            continue;
        }

        let content = {
            let db = state.db.lock().unwrap();
            match crate::db::do_export_conversation_pub(&db, &conv.id, "json") {
                Ok(c) => c,
                Err(e) => { errors.push(format!("{}: {}", conv.id, e)); continue; }
            }
        };

        let filename = format!("{}/conversations/{}.json", repo, conv.id);
        if let Err(e) = std::fs::write(&filename, &content) {
            errors.push(format!("{}: {e}", conv.id));
            continue;
        }

        {
            let db = state.db.lock().unwrap();
            let _ = db.set_sync_state(&conv.id, &conv.updated_at, &timestamp);
        }
        pushed += 1;
    }

    if pushed == 0 {
        return Ok(SyncResult { pushed, pulled: 0, skipped, errors, timestamp });
    }

    // Stage, commit, push
    if let Err(e) = run_git_in(&["-C", repo, "add", "conversations/"], repo) {
        errors.push(format!("git add: {e}"));
        return Ok(SyncResult { pushed, pulled: 0, skipped, errors, timestamp });
    }

    let msg = format!("LCD sync {}", chrono::Utc::now().format("%Y-%m-%d %H:%M UTC"));
    let commit_result = run_git_in(&[
        "-C", repo,
        "-c", &format!("user.name={}", config.commit_name),
        "-c", &format!("user.email={}", config.commit_email),
        "commit", "-m", &msg,
    ], repo);

    match commit_result {
        Err(e) if e.contains("nothing to commit") => {}
        Err(e) => { errors.push(format!("git commit: {e}")); }
        Ok(_) => {
            // Push if a remote exists
            let has_remote = run_git_in(&["-C", repo, "remote"], repo)
                .map(|out| !out.trim().is_empty())
                .unwrap_or(false);
            if has_remote {
                if let Err(e) = run_git_in(&["-C", repo, "push"], repo) {
                    errors.push(format!("git push: {e}"));
                }
            }
        }
    }

    Ok(SyncResult { pushed, pulled: 0, skipped, errors, timestamp })
}

fn do_pull(state: &tauri::State<crate::AppState>, config: &SyncConfig) -> Result<SyncResult, String> {
    let repo = &config.repo_path;
    let mut pulled = 0usize;
    let mut errors: Vec<String> = Vec::new();
    let timestamp = chrono::Utc::now().to_rfc3339();

    // Pull from remote if one exists
    let has_remote = run_git_in(&["-C", repo, "remote"], repo)
        .map(|out| !out.trim().is_empty())
        .unwrap_or(false);
    if has_remote {
        if let Err(e) = run_git_in(&["-C", repo, "pull"], repo) {
            errors.push(format!("git pull: {e}"));
        }
    }

    // Scan conversations/ directory
    let conv_dir = format!("{}/conversations", repo);
    let entries = match std::fs::read_dir(&conv_dir) {
        Ok(e) => e,
        Err(_) => return Ok(SyncResult { pushed: 0, pulled, skipped: 0, errors, timestamp }),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => { errors.push(format!("{:?}: {e}", path.file_name())); continue; }
        };
        let exported: crate::db::ExportedConversation = match serde_json::from_str(&content) {
            Ok(e) => e,
            Err(e) => { errors.push(format!("{:?}: invalid JSON: {e}", path.file_name())); continue; }
        };
        if exported.id.is_empty() {
            errors.push(format!("{:?}: missing id field", path.file_name()));
            continue;
        }

        let should_import = {
            let db = state.db.lock().unwrap();
            let last_synced = db.get_sync_state(&exported.id);
            // Import if we have no record of syncing this conversation, or if it was updated remotely
            last_synced.is_none() || last_synced.as_deref() != Some(&exported.created_at)
        };

        if !should_import {
            continue;
        }

        let db = state.db.lock().unwrap();
        match db.upsert_synced_conversation(
            &exported.id,
            &exported.title,
            &exported.created_at,
            &exported.messages,
        ) {
            Ok(_) => {
                let _ = db.set_sync_state(&exported.id, &exported.created_at, &timestamp);
                pulled += 1;
            }
            Err(e) => errors.push(format!("{}: upsert failed: {e}", exported.id)),
        }
    }

    Ok(SyncResult { pushed: 0, pulled, skipped: 0, errors, timestamp })
}
