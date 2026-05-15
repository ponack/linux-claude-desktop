use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use tauri::Emitter;

// ── Config types ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncConfig {
    pub enabled: bool,
    pub backend_type: String,          // "git" | "webdav" | "s3"
    pub auto_sync_interval_mins: u32,  // 0 = manual only
    // Git backend
    pub repo_path: String,
    pub commit_name: String,
    pub commit_email: String,
    // WebDAV backend
    pub webdav_url: String,
    pub webdav_username: String,
    pub webdav_password: String,
    // S3-compatible backend
    pub s3_endpoint: String,
    pub s3_bucket: String,
    pub s3_region: String,
    pub s3_access_key: String,
    pub s3_secret_key: String,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            backend_type: "git".to_string(),
            auto_sync_interval_mins: 0,
            repo_path: String::new(),
            commit_name: "LCD Sync".to_string(),
            commit_email: "sync@linux-claude-desktop.local".to_string(),
            webdav_url: String::new(),
            webdav_username: String::new(),
            webdav_password: String::new(),
            s3_endpoint: String::new(),
            s3_bucket: String::new(),
            s3_region: "us-east-1".to_string(),
            s3_access_key: String::new(),
            s3_secret_key: String::new(),
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct SyncResult {
    pub pushed: usize,
    pub pulled: usize,
    pub skipped: usize,
    pub conflicts: usize,
    pub errors: Vec<String>,
    pub timestamp: String,
}

fn emit_sync_status(app: &tauri::AppHandle, status: &str, result: Option<&SyncResult>) {
    let _ = app.emit("sync-status", serde_json::json!({
        "status": status,
        "pushed": result.map(|r| r.pushed).unwrap_or(0),
        "pulled": result.map(|r| r.pulled).unwrap_or(0),
        "conflicts": result.map(|r| r.conflicts).unwrap_or(0),
        "errors": result.map(|r| r.errors.len()).unwrap_or(0),
        "timestamp": result.map(|r| r.timestamp.as_str()).unwrap_or(""),
    }));
}

fn emit_sync_done(app: &tauri::AppHandle, result: &Result<SyncResult, String>) {
    match result {
        Ok(r) => emit_sync_status(app, if r.errors.is_empty() { "done" } else { "done_with_errors" }, Some(r)),
        Err(e) => { let _ = app.emit("sync-status", serde_json::json!({ "status": "error", "error": e })); }
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_sync_config(state: tauri::State<crate::AppState>) -> SyncConfig {
    let db = state.db.lock().unwrap();
    SyncConfig {
        enabled: db.get_sync_value("enabled").map(|v| v == "1").unwrap_or(false),
        backend_type: db.get_sync_value("backend_type").unwrap_or_else(|| "git".to_string()),
        auto_sync_interval_mins: db.get_sync_value("auto_sync_interval_mins")
            .and_then(|v| v.parse().ok()).unwrap_or(0),
        repo_path: db.get_sync_value("repo_path").unwrap_or_default(),
        commit_name: db.get_sync_value("commit_name")
            .unwrap_or_else(|| "LCD Sync".to_string()),
        commit_email: db.get_sync_value("commit_email")
            .unwrap_or_else(|| "sync@linux-claude-desktop.local".to_string()),
        webdav_url: db.get_sync_value("webdav_url").unwrap_or_default(),
        webdav_username: db.get_sync_value("webdav_username").unwrap_or_default(),
        webdav_password: db.get_sync_value("webdav_password").unwrap_or_default(),
        s3_endpoint: db.get_sync_value("s3_endpoint").unwrap_or_default(),
        s3_bucket: db.get_sync_value("s3_bucket").unwrap_or_default(),
        s3_region: db.get_sync_value("s3_region")
            .unwrap_or_else(|| "us-east-1".to_string()),
        s3_access_key: db.get_sync_value("s3_access_key").unwrap_or_default(),
        s3_secret_key: db.get_sync_value("s3_secret_key").unwrap_or_default(),
    }
}

#[tauri::command]
pub fn set_sync_config(state: tauri::State<crate::AppState>, config: SyncConfig) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    let kv = [
        ("enabled", if config.enabled { "1" } else { "0" }),
        ("backend_type", &config.backend_type),
        ("auto_sync_interval_mins", &config.auto_sync_interval_mins.to_string()),
        ("repo_path", &config.repo_path),
        ("commit_name", &config.commit_name),
        ("commit_email", &config.commit_email),
        ("webdav_url", &config.webdav_url),
        ("webdav_username", &config.webdav_username),
        ("webdav_password", &config.webdav_password),
        ("s3_endpoint", &config.s3_endpoint),
        ("s3_bucket", &config.s3_bucket),
        ("s3_region", &config.s3_region),
        ("s3_access_key", &config.s3_access_key),
        ("s3_secret_key", &config.s3_secret_key),
    ];
    for (k, v) in &kv {
        db.set_sync_value(k, v).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn init_sync_repo(path: String) -> Result<String, String> {
    let p = Path::new(&path);
    if !p.exists() {
        std::fs::create_dir_all(p).map_err(|e| format!("Cannot create directory: {e}"))?;
    }
    let is_repo = Command::new("git")
        .args(["-C", &path, "rev-parse", "--git-dir"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !is_repo {
        let out = Command::new("git").args(["-C", &path, "init"])
            .output().map_err(|e| format!("git not found: {e}"))?;
        if !out.status.success() {
            return Err(String::from_utf8_lossy(&out.stderr).to_string());
        }
    }
    Ok(path)
}

#[tauri::command]
pub async fn test_sync_connection(config: SyncConfig) -> Result<String, String> {
    match config.backend_type.as_str() {
        "webdav" => webdav_test(&config).await,
        "s3" => s3_test(&config).await,
        _ => Err("Use init_sync_repo to verify a git backend".to_string()),
    }
}

#[tauri::command]
pub fn sync_push(app: tauri::AppHandle, state: tauri::State<crate::AppState>) -> Result<SyncResult, String> {
    let config = get_sync_config(state.clone());
    if config.repo_path.is_empty() && config.backend_type == "git" {
        return Err("Sync repo path not configured".to_string());
    }
    emit_sync_status(&app, "syncing", None);
    let result = do_push_sync(&state, &config);
    emit_sync_done(&app, &result);
    result
}

#[tauri::command]
pub fn sync_pull(app: tauri::AppHandle, state: tauri::State<crate::AppState>) -> Result<SyncResult, String> {
    let config = get_sync_config(state.clone());
    emit_sync_status(&app, "syncing", None);
    let result = do_pull_sync(&state, &config);
    emit_sync_done(&app, &result);
    if let Ok(ref r) = result {
        if r.conflicts > 0 {
            let _ = app.emit("sync-conflicts", serde_json::json!({ "count": r.conflicts }));
        }
    }
    result
}

#[tauri::command]
pub fn sync_now(app: tauri::AppHandle, state: tauri::State<crate::AppState>) -> Result<SyncResult, String> {
    let config = get_sync_config(state.clone());
    emit_sync_status(&app, "syncing", None);
    let mut pull_result = do_pull_sync(&state, &config)?;
    let push_result = do_push_sync(&state, &config)?;
    pull_result.pushed = push_result.pushed;
    pull_result.skipped = push_result.skipped;
    pull_result.errors.extend(push_result.errors);
    emit_sync_done(&app, &Ok(pull_result.clone()));
    if pull_result.conflicts > 0 {
        let _ = app.emit("sync-conflicts", serde_json::json!({ "count": pull_result.conflicts }));
    }
    Ok(pull_result)
}

// ── Dispatch ──────────────────────────────────────────────────────────────────

fn do_push_sync(state: &tauri::State<crate::AppState>, config: &SyncConfig) -> Result<SyncResult, String> {
    match config.backend_type.as_str() {
        "webdav" => tokio::runtime::Handle::current()
            .block_on(webdav_push_all(state, config)),
        "s3" => tokio::runtime::Handle::current()
            .block_on(s3_push_all(state, config)),
        _ => git_push(state, config),
    }
}

fn do_pull_sync(state: &tauri::State<crate::AppState>, config: &SyncConfig) -> Result<SyncResult, String> {
    match config.backend_type.as_str() {
        "webdav" => tokio::runtime::Handle::current()
            .block_on(webdav_pull_all(state, config)),
        "s3" => tokio::runtime::Handle::current()
            .block_on(s3_pull_all(state, config)),
        _ => git_pull(state, config),
    }
}

// ── Git backend ───────────────────────────────────────────────────────────────

fn run_git_in(args: &[&str], cwd: &str) -> Result<String, String> {
    let output = Command::new("git").args(args).current_dir(cwd)
        .output().map_err(|e| format!("git not found: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

fn git_push(state: &tauri::State<crate::AppState>, config: &SyncConfig) -> Result<SyncResult, String> {
    let repo = &config.repo_path;
    let conv_dir = format!("{}/conversations", repo);
    std::fs::create_dir_all(&conv_dir)
        .map_err(|e| format!("Cannot create conversations dir: {e}"))?;

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
        if sync_states.get(&conv.id).map(|s| s == &conv.updated_at).unwrap_or(false) {
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
        if let Err(e) = std::fs::write(format!("{conv_dir}/{}.json", conv.id), &content) {
            errors.push(format!("{}: {e}", conv.id)); continue;
        }
        let _ = state.db.lock().unwrap().set_sync_state(&conv.id, &conv.updated_at, &timestamp);
        pushed += 1;
    }

    if pushed == 0 {
        return Ok(SyncResult { pushed, pulled: 0, skipped, conflicts: 0, errors, timestamp });
    }

    if let Err(e) = run_git_in(&["-C", repo, "add", "conversations/"], repo) {
        errors.push(format!("git add: {e}"));
        return Ok(SyncResult { pushed, pulled: 0, skipped, conflicts: 0, errors, timestamp });
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
            let has_remote = run_git_in(&["-C", repo, "remote"], repo)
                .map(|out| !out.trim().is_empty()).unwrap_or(false);
            if has_remote {
                if let Err(e) = run_git_in(&["-C", repo, "push"], repo) {
                    errors.push(format!("git push: {e}"));
                }
            }
        }
    }

    Ok(SyncResult { pushed, pulled: 0, skipped, conflicts: 0, errors, timestamp })
}

fn git_pull(state: &tauri::State<crate::AppState>, config: &SyncConfig) -> Result<SyncResult, String> {
    let repo = &config.repo_path;
    let mut pulled = 0usize;
    let mut conflicts = 0usize;
    let mut errors: Vec<String> = Vec::new();
    let timestamp = chrono::Utc::now().to_rfc3339();

    let has_remote = run_git_in(&["-C", repo, "remote"], repo)
        .map(|out| !out.trim().is_empty()).unwrap_or(false);
    if has_remote {
        if let Err(e) = run_git_in(&["-C", repo, "pull"], repo) {
            errors.push(format!("git pull: {e}"));
        }
    }

    let conv_dir = format!("{}/conversations", repo);
    let entries = match std::fs::read_dir(&conv_dir) {
        Ok(e) => e,
        Err(_) => return Ok(SyncResult { pushed: 0, pulled, skipped: 0, conflicts: 0, errors, timestamp }),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") { continue; }
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => { errors.push(format!("{:?}: {e}", path.file_name())); continue; }
        };
        match upsert_from_json(state, &content, &timestamp) {
            Ok(true) => conflicts += 1,
            Ok(false) => pulled += 1,
            Err(e) => errors.push(e),
        }
    }

    Ok(SyncResult { pushed: 0, pulled, skipped: 0, conflicts, errors, timestamp })
}

// ── WebDAV backend ────────────────────────────────────────────────────────────

fn webdav_client(_config: &SyncConfig) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_default()
}

async fn webdav_test(config: &SyncConfig) -> Result<String, String> {
    if config.webdav_url.is_empty() {
        return Err("WebDAV URL not configured".to_string());
    }
    let client = webdav_client(config);
    let resp = client
        .request(reqwest::Method::from_bytes(b"OPTIONS").unwrap(), &config.webdav_url)
        .basic_auth(&config.webdav_username, Some(&config.webdav_password))
        .send().await.map_err(|e| format!("Connection failed: {e}"))?;
    if resp.status().is_success() || resp.status().as_u16() == 207 {
        Ok("Connected successfully".to_string())
    } else {
        Err(format!("Server returned {}", resp.status()))
    }
}

async fn webdav_ensure_dir(client: &reqwest::Client, config: &SyncConfig) -> Result<(), String> {
    let url = format!("{}/conversations", config.webdav_url.trim_end_matches('/'));
    let resp = client
        .request(reqwest::Method::from_bytes(b"MKCOL").unwrap(), &url)
        .basic_auth(&config.webdav_username, Some(&config.webdav_password))
        .send().await.map_err(|e| e.to_string())?;
    // 201 Created, 405 Already exists — both acceptable
    if resp.status().is_success() || resp.status().as_u16() == 405 {
        Ok(())
    } else {
        Err(format!("MKCOL failed: {}", resp.status()))
    }
}

async fn webdav_put(client: &reqwest::Client, config: &SyncConfig, id: &str, content: String) -> Result<(), String> {
    let url = format!("{}/conversations/{}.json", config.webdav_url.trim_end_matches('/'), id);
    let resp = client
        .put(&url)
        .basic_auth(&config.webdav_username, Some(&config.webdav_password))
        .header("Content-Type", "application/json")
        .body(content)
        .send().await.map_err(|e| e.to_string())?;
    if resp.status().is_success() || resp.status().as_u16() == 201 {
        Ok(())
    } else {
        Err(format!("PUT failed: {}", resp.status()))
    }
}

async fn webdav_list(client: &reqwest::Client, config: &SyncConfig) -> Result<Vec<String>, String> {
    let url = format!("{}/conversations/", config.webdav_url.trim_end_matches('/'));
    let body = r#"<?xml version="1.0"?><propfind xmlns="DAV:"><prop><resourcetype/><getcontentlength/></prop></propfind>"#;
    let resp = client
        .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &url)
        .basic_auth(&config.webdav_username, Some(&config.webdav_password))
        .header("Depth", "1")
        .header("Content-Type", "application/xml")
        .body(body)
        .send().await.map_err(|e| e.to_string())?;

    if !resp.status().is_success() && resp.status().as_u16() != 207 {
        return Err(format!("PROPFIND failed: {}", resp.status()));
    }

    let xml = resp.text().await.map_err(|e| e.to_string())?;
    // Extract hrefs ending in .json from the PROPFIND multistatus response
    let hrefs: Vec<String> = xml.split("<D:href>").skip(1)
        .chain(xml.split("<href>").skip(1))
        .filter_map(|s| {
            let href = s.split("</").next()?.trim().to_string();
            if href.ends_with(".json") { Some(href) } else { None }
        })
        .collect();
    Ok(hrefs)
}

async fn webdav_get(client: &reqwest::Client, config: &SyncConfig, href: &str) -> Result<String, String> {
    let url = if href.starts_with("http://") || href.starts_with("https://") {
        href.to_string()
    } else {
        // href is a path — prepend base host
        let base = reqwest::Url::parse(&config.webdav_url).map_err(|e| e.to_string())?;
        format!("{}://{}{}", base.scheme(), base.host_str().unwrap_or(""), href)
    };
    let resp = client.get(&url)
        .basic_auth(&config.webdav_username, Some(&config.webdav_password))
        .send().await.map_err(|e| e.to_string())?;
    resp.text().await.map_err(|e| e.to_string())
}

async fn webdav_push_all(state: &tauri::State<'_, crate::AppState>, config: &SyncConfig) -> Result<SyncResult, String> {
    let client = webdav_client(config);
    webdav_ensure_dir(&client, config).await?;

    let mut pushed = 0usize;
    let mut skipped = 0usize;
    let mut errors: Vec<String> = Vec::new();
    let timestamp = chrono::Utc::now().to_rfc3339();

    let (conversations, sync_states): (Vec<crate::db::Conversation>, std::collections::HashMap<String, String>) = {
        let db = state.db.lock().unwrap();
        (db.list_conversations().map_err(|e: rusqlite::Error| e.to_string())?,
         db.get_all_sync_states())
    };

    for conv in &conversations {
        if sync_states.get(&conv.id).map(|s: &String| s == &conv.updated_at).unwrap_or(false) {
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
        match webdav_put(&client, config, &conv.id, content).await {
            Ok(()) => {
                let _ = state.db.lock().unwrap().set_sync_state(&conv.id, &conv.updated_at, &timestamp);
                pushed += 1;
            }
            Err(e) => errors.push(format!("{}: {e}", conv.id)),
        }
    }

    Ok(SyncResult { pushed, pulled: 0, skipped, conflicts: 0, errors, timestamp })
}

async fn webdav_pull_all(state: &tauri::State<'_, crate::AppState>, config: &SyncConfig) -> Result<SyncResult, String> {
    let client = webdav_client(config);
    let mut pulled = 0usize;
    let mut conflicts = 0usize;
    let mut errors: Vec<String> = Vec::new();
    let timestamp = chrono::Utc::now().to_rfc3339();

    let hrefs = match webdav_list(&client, config).await {
        Ok(h) => h,
        Err(e) => {
            errors.push(format!("List failed: {e}"));
            return Ok(SyncResult { pushed: 0, pulled, skipped: 0, conflicts: 0, errors, timestamp });
        }
    };

    for href in &hrefs {
        match webdav_get(&client, config, href).await {
            Ok(content) => {
                match upsert_from_json(state, &content, &timestamp) {
                    Ok(true) => conflicts += 1,
                    Ok(false) => pulled += 1,
                    Err(e) => errors.push(e),
                }
            }
            Err(e) => errors.push(format!("{href}: {e}")),
        }
    }

    Ok(SyncResult { pushed: 0, pulled, skipped: 0, conflicts, errors, timestamp })
}

// ── S3-compatible backend (SigV4) ─────────────────────────────────────────────

fn s3_base_url(config: &SyncConfig) -> String {
    if config.s3_endpoint.is_empty() {
        format!("https://{}.s3.{}.amazonaws.com", config.s3_bucket, config.s3_region)
    } else {
        let ep = config.s3_endpoint.trim_end_matches('/');
        format!("{}/{}", ep, config.s3_bucket)
    }
}

async fn s3_test(config: &SyncConfig) -> Result<String, String> {
    if config.s3_bucket.is_empty() || config.s3_access_key.is_empty() {
        return Err("S3 bucket and access key required".to_string());
    }
    let client = reqwest::Client::new();
    let url = format!("{}/?list-type=2&prefix=conversations/&max-keys=1", s3_base_url(config));
    let headers = s3_sign_request("GET", &url, &[], config);
    let mut req = client.get(&url);
    for (k, v) in &headers { req = req.header(k.as_str(), v.as_str()); }
    let resp = req.send().await.map_err(|e| format!("Connection failed: {e}"))?;
    if resp.status().is_success() {
        Ok("Connected successfully".to_string())
    } else {
        Err(format!("Server returned {}: {}", resp.status(),
            resp.text().await.unwrap_or_default().chars().take(200).collect::<String>()))
    }
}

async fn s3_put(client: &reqwest::Client, config: &SyncConfig, key: &str, body: Vec<u8>) -> Result<(), String> {
    let url = format!("{}/{}", s3_base_url(config), key);
    let headers = s3_sign_request("PUT", &url, &body, config);
    let mut req = client.put(&url).body(body);
    for (k, v) in &headers { req = req.header(k.as_str(), v.as_str()); }
    let resp = req.send().await.map_err(|e| e.to_string())?;
    if resp.status().is_success() { Ok(()) }
    else { Err(format!("PUT failed: {}", resp.status())) }
}

async fn s3_list(client: &reqwest::Client, config: &SyncConfig) -> Result<Vec<String>, String> {
    let url = format!("{}/?list-type=2&prefix=conversations/", s3_base_url(config));
    let headers = s3_sign_request("GET", &url, &[], config);
    let mut req = client.get(&url);
    for (k, v) in &headers { req = req.header(k.as_str(), v.as_str()); }
    let resp = req.send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("LIST failed: {}", resp.status()));
    }
    let xml = resp.text().await.map_err(|e| e.to_string())?;
    let keys: Vec<String> = xml.split("<Key>").skip(1)
        .filter_map(|s| {
            let key = s.split("</Key>").next()?.trim().to_string();
            if key.starts_with("conversations/") && key.ends_with(".json") {
                Some(key)
            } else {
                None
            }
        })
        .collect();
    Ok(keys)
}

async fn s3_get(client: &reqwest::Client, config: &SyncConfig, key: &str) -> Result<String, String> {
    let url = format!("{}/{}", s3_base_url(config), key);
    let headers = s3_sign_request("GET", &url, &[], config);
    let mut req = client.get(&url);
    for (k, v) in &headers { req = req.header(k.as_str(), v.as_str()); }
    let resp = req.send().await.map_err(|e| e.to_string())?;
    resp.text().await.map_err(|e| e.to_string())
}

async fn s3_push_all(state: &tauri::State<'_, crate::AppState>, config: &SyncConfig) -> Result<SyncResult, String> {
    let client = reqwest::Client::new();
    let mut pushed = 0usize;
    let mut skipped = 0usize;
    let mut errors: Vec<String> = Vec::new();
    let timestamp = chrono::Utc::now().to_rfc3339();

    let (conversations, sync_states): (Vec<crate::db::Conversation>, std::collections::HashMap<String, String>) = {
        let db = state.db.lock().unwrap();
        (db.list_conversations().map_err(|e: rusqlite::Error| e.to_string())?,
         db.get_all_sync_states())
    };

    for conv in &conversations {
        if sync_states.get(&conv.id).map(|s: &String| s == &conv.updated_at).unwrap_or(false) {
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
        let key = format!("conversations/{}.json", conv.id);
        match s3_put(&client, config, &key, content.into_bytes()).await {
            Ok(()) => {
                let _ = state.db.lock().unwrap().set_sync_state(&conv.id, &conv.updated_at, &timestamp);
                pushed += 1;
            }
            Err(e) => errors.push(format!("{}: {e}", conv.id)),
        }
    }

    Ok(SyncResult { pushed, pulled: 0, skipped, conflicts: 0, errors, timestamp })
}

async fn s3_pull_all(state: &tauri::State<'_, crate::AppState>, config: &SyncConfig) -> Result<SyncResult, String> {
    let client = reqwest::Client::new();
    let mut pulled = 0usize;
    let mut conflicts = 0usize;
    let mut errors: Vec<String> = Vec::new();
    let timestamp = chrono::Utc::now().to_rfc3339();

    let keys = match s3_list(&client, config).await {
        Ok(k) => k,
        Err(e) => {
            errors.push(format!("List failed: {e}"));
            return Ok(SyncResult { pushed: 0, pulled, skipped: 0, conflicts: 0, errors, timestamp });
        }
    };

    for key in &keys {
        match s3_get(&client, config, key).await {
            Ok(content) => {
                match upsert_from_json(state, &content, &timestamp) {
                    Ok(true) => conflicts += 1,
                    Ok(false) => pulled += 1,
                    Err(e) => errors.push(e),
                }
            }
            Err(e) => errors.push(format!("{key}: {e}")),
        }
    }

    Ok(SyncResult { pushed: 0, pulled, skipped: 0, conflicts, errors, timestamp })
}

// ── SigV4 signing ─────────────────────────────────────────────────────────────

fn s3_sign_request(method: &str, url: &str, payload: &[u8], config: &SyncConfig) -> Vec<(String, String)> {
    use hmac::{Hmac, Mac};
    use sha2::{Digest, Sha256};

    let now = chrono::Utc::now();
    let date = now.format("%Y%m%d").to_string();
    let datetime = now.format("%Y%m%dT%H%M%SZ").to_string();

    let parsed = reqwest::Url::parse(url).unwrap();
    let host = parsed.host_str().unwrap_or("");
    let uri = parsed.path();
    let query = parsed.query().unwrap_or("");

    let payload_hash = hex::encode(Sha256::digest(payload));

    let canonical_headers = format!(
        "host:{}\nx-amz-content-sha256:{}\nx-amz-date:{}\n",
        host, payload_hash, datetime
    );
    let signed_headers = "host;x-amz-content-sha256;x-amz-date";

    let canonical_request = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        method, uri, query, canonical_headers, signed_headers, payload_hash
    );

    let scope = format!("{}/{}/s3/aws4_request", date, config.s3_region);
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{}\n{}\n{}",
        datetime, scope,
        hex::encode(Sha256::digest(canonical_request.as_bytes()))
    );

    let hmac_key = |key: &[u8], data: &[u8]| -> Vec<u8> {
        let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC key");
        mac.update(data);
        mac.finalize().into_bytes().to_vec()
    };

    let k_date = hmac_key(format!("AWS4{}", config.s3_secret_key).as_bytes(), date.as_bytes());
    let k_region = hmac_key(&k_date, config.s3_region.as_bytes());
    let k_service = hmac_key(&k_region, b"s3");
    let k_signing = hmac_key(&k_service, b"aws4_request");
    let signature = hex::encode(hmac_key(&k_signing, string_to_sign.as_bytes()));

    let auth = format!(
        "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
        config.s3_access_key, scope, signed_headers, signature
    );

    vec![
        ("x-amz-date".to_string(), datetime),
        ("x-amz-content-sha256".to_string(), payload_hash),
        ("Authorization".to_string(), auth),
    ]
}

// ── Shared helper ─────────────────────────────────────────────────────────────

/// Returns Ok(true) if a conflict was detected (and stored), Ok(false) if cleanly upserted.
fn upsert_from_json(
    state: &tauri::State<crate::AppState>,
    content: &str,
    timestamp: &str,
) -> Result<bool, String> {
    let exported: crate::db::ExportedConversation = serde_json::from_str(content)
        .map_err(|e| format!("Invalid JSON: {e}"))?;
    if exported.id.is_empty() {
        return Err("Export missing id field".to_string());
    }

    let db = state.db.lock().unwrap();

    // Conflict detection: both local and remote changed since last sync
    let local_updated_at = db.get_local_conversation_updated_at(&exported.id);
    let last_synced = db.get_sync_state(&exported.id);
    let remote_updated_at = if exported.updated_at.is_empty() {
        exported.created_at.clone()
    } else {
        exported.updated_at.clone()
    };

    let is_conflict = match (&local_updated_at, &last_synced) {
        (Some(local_ua), Some(last_sync)) => {
            local_ua != last_sync            // local modified since last sync
            && &remote_updated_at != last_sync  // remote also modified since last sync
            && local_ua != &remote_updated_at   // they differ
        }
        _ => false,
    };

    if is_conflict {
        let local_title = db.get_local_conversation_title(&exported.id).unwrap_or_default();
        db.set_sync_conflict(&exported.id, content, &local_title, &exported.title, timestamp)
            .map_err(|e| e.to_string())?;
        return Ok(true);
    }

    db.upsert_synced_conversation(&exported.id, &exported.title, &exported.created_at, &exported.messages)
        .map_err(|e| e.to_string())?;
    let _ = db.set_sync_state(&exported.id, &remote_updated_at, timestamp);
    Ok(false)
}
