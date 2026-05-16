use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::Manager;
use tower_http::cors::CorsLayer;

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiServerConfig {
    pub enabled: bool,
    pub port: u16,
    pub lan_access: bool,
    pub token: String,
}

impl Default for ApiServerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 7432,
            lan_access: false,
            token: uuid::Uuid::new_v4().to_string(),
        }
    }
}

pub fn load_config(state: &crate::AppState) -> ApiServerConfig {
    let db = state.db.lock().unwrap();
    let token = db.get_api_server_value("token").unwrap_or_else(|| {
        let t = uuid::Uuid::new_v4().to_string();
        let _ = db.set_api_server_value("token", &t);
        t
    });
    ApiServerConfig {
        enabled: db.get_api_server_value("enabled").map(|v| v == "1").unwrap_or(false),
        port: db.get_api_server_value("port").and_then(|v| v.parse().ok()).unwrap_or(7432),
        lan_access: db.get_api_server_value("lan_access").map(|v| v == "1").unwrap_or(false),
        token,
    }
}

#[tauri::command]
pub fn get_api_server_config(state: tauri::State<crate::AppState>) -> ApiServerConfig {
    load_config(&state)
}

#[tauri::command]
pub fn set_api_server_config(
    state: tauri::State<crate::AppState>,
    config: ApiServerConfig,
) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    let kv = [
        ("enabled", if config.enabled { "1" } else { "0" }),
        ("port", &config.port.to_string()),
        ("lan_access", if config.lan_access { "1" } else { "0" }),
    ];
    for (k, v) in &kv {
        db.set_api_server_value(k, v).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn rotate_api_server_token(state: tauri::State<crate::AppState>) -> Result<String, String> {
    let new_token = uuid::Uuid::new_v4().to_string();
    let db = state.db.lock().unwrap();
    db.set_api_server_value("token", &new_token).map_err(|e| e.to_string())?;
    Ok(new_token)
}

#[derive(Serialize)]
pub struct PairingQrResult {
    pub svg: String,
    pub token: String,
    pub url: String,
}

fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'
            | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

#[tauri::command]
pub fn generate_pairing_qr(
    state: tauri::State<crate::AppState>,
    local_ip: String,
) -> Result<PairingQrResult, String> {
    let new_token = uuid::Uuid::new_v4().to_string();
    let db = state.db.lock().unwrap();
    db.set_api_server_value("token", &new_token).map_err(|e| e.to_string())?;
    let port = db
        .get_api_server_value("port")
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(7432);
    drop(db);

    let api_url = format!("http://{}:{}", local_ip.trim(), port);
    let pairing_url = format!(
        "https://ponack.github.io/linux-claude-desktop/?lcd_url={}&lcd_token={}",
        percent_encode(&api_url),
        percent_encode(&new_token),
    );

    let code = qrcode::QrCode::new(pairing_url.as_bytes()).map_err(|e| e.to_string())?;
    let svg = code
        .render::<qrcode::render::svg::Color>()
        .min_dimensions(220, 220)
        .build();

    Ok(PairingQrResult { svg, token: new_token, url: pairing_url })
}

// ── HTTP server ───────────────────────────────────────────────────────────────

#[derive(Clone)]
struct Ctx {
    app: tauri::AppHandle,
}

type Cx = State<Arc<Ctx>>;

fn get_cfg(ctx: &Ctx) -> ApiServerConfig {
    let s = ctx.app.state::<crate::AppState>();
    load_config(&s)
}

fn auth_err() -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"})))
}

fn svc_err(msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error": msg})))
}

fn check_auth(
    headers: &axum::http::HeaderMap,
    ctx: &Ctx,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    let cfg = get_cfg(ctx);
    if !cfg.enabled {
        return Err(svc_err("API server is disabled"));
    }
    let ok = headers.get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|t| t == cfg.token)
        .unwrap_or(false);
    if ok { Ok(()) } else { Err(auth_err()) }
}

// GET /api/conversations
async fn list_conversations_handler(
    State(ctx): Cx,
    headers: axum::http::HeaderMap,
) -> Result<Json<Vec<crate::db::Conversation>>, (StatusCode, Json<serde_json::Value>)> {
    check_auth(&headers, &ctx)?;
    let s = ctx.app.state::<crate::AppState>();
    let db = s.db.lock().unwrap();
    let convs = db.list_conversations()
        .map_err(|e: rusqlite::Error| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?;
    Ok(Json(convs))
}

// GET /api/conversations/:id
#[derive(Serialize)]
struct ConversationDetail {
    id: String,
    title: String,
    created_at: String,
    updated_at: String,
    messages: Vec<crate::db::Message>,
}

async fn get_conversation_handler(
    State(ctx): Cx,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ConversationDetail>, (StatusCode, Json<serde_json::Value>)> {
    check_auth(&headers, &ctx)?;
    let s = ctx.app.state::<crate::AppState>();
    let db = s.db.lock().unwrap();
    let conv = db.list_conversations()
        .map_err(|e: rusqlite::Error| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?
        .into_iter()
        .find(|c| c.id == id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Not found"}))))?;
    let messages = db.list_messages(&id)
        .map_err(|e: rusqlite::Error| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?;
    Ok(Json(ConversationDetail {
        id: conv.id,
        title: conv.title,
        created_at: conv.created_at,
        updated_at: conv.updated_at,
        messages,
    }))
}

// POST /api/conversations
#[derive(Deserialize)]
struct CreateConvBody {
    title: Option<String>,
}

#[derive(Serialize)]
struct CreatedConv {
    id: String,
    title: String,
}

async fn create_conversation_handler(
    State(ctx): Cx,
    headers: axum::http::HeaderMap,
    Json(body): Json<CreateConvBody>,
) -> Result<Json<CreatedConv>, (StatusCode, Json<serde_json::Value>)> {
    check_auth(&headers, &ctx)?;
    let id = uuid::Uuid::new_v4().to_string();
    let title = body.title.unwrap_or_else(|| "New Conversation".to_string());
    let s = ctx.app.state::<crate::AppState>();
    let db = s.db.lock().unwrap();
    db.insert_conversation(&id, &title)
        .map_err(|e: rusqlite::Error| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?;
    Ok(Json(CreatedConv { id, title }))
}

// POST /api/conversations/:id/messages
#[derive(Deserialize)]
struct SendMsgBody {
    content: String,
}

async fn send_message_handler(
    State(ctx): Cx,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<SendMsgBody>,
) -> Result<Json<crate::db::Message>, (StatusCode, Json<serde_json::Value>)> {
    check_auth(&headers, &ctx)?;

    let s = ctx.app.state::<crate::AppState>();

    // Verify conversation exists
    {
        let db = s.db.lock().unwrap();
        let exists = db.list_conversations()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?
            .iter().any(|c| c.id == id);
        if !exists {
            return Err((StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Conversation not found"}))));
        }
    }

    // Resolve provider
    let provider = crate::api::resolve_provider(&s, None)
        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": e}))))?;

    // Store user message
    let user_msg_id = uuid::Uuid::new_v4().to_string();
    {
        let db = s.db.lock().unwrap();
        db.insert_message(&user_msg_id, &id, "user", &body.content)
            .map_err(|e: rusqlite::Error| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?;
    }

    // Build message history for API call
    let api_messages: Vec<serde_json::Value> = {
        let db = s.db.lock().unwrap();
        db.list_messages(&id)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?
            .into_iter()
            .map(|m| serde_json::json!({"role": m.role, "content": m.content}))
            .collect()
    };

    // Get system prompt
    let system_prompt: Option<String> = {
        let db = s.db.lock().unwrap();
        db.get_setting("system_prompt").ok().flatten()
    };

    // Call AI (non-streaming)
    let assistant_content = call_ai_sync(&provider, &api_messages, system_prompt.as_deref())
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": e}))))?;

    // Store assistant message
    let assistant_msg_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    {
        let db = s.db.lock().unwrap();
        db.insert_message(&assistant_msg_id, &id, "assistant", &assistant_content)
            .map_err(|e: rusqlite::Error| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?;
    }

    Ok(Json(crate::db::Message {
        id: assistant_msg_id,
        conversation_id: id,
        role: "assistant".to_string(),
        content: assistant_content,
        created_at: now,
    }))
}

async fn call_ai_sync(
    provider: &crate::providers::ResolvedProvider,
    messages: &[serde_json::Value],
    system_prompt: Option<&str>,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;

    if provider.api_format == "anthropic" {
        let mut body = serde_json::json!({
            "model": provider.model,
            "max_tokens": 8192,
            "stream": false,
            "messages": messages,
        });
        if let Some(sp) = system_prompt {
            if !sp.trim().is_empty() {
                body["system"] = serde_json::json!(sp);
            }
        }
        let resp = client
            .post(format!("{}/v1/messages", provider.base_url))
            .header("x-api-key", &provider.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send().await.map_err(|e| format!("Request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("API error {status}: {}", text.chars().take(300).collect::<String>()));
        }

        let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        data["content"][0]["text"].as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "Unexpected API response shape".to_string())
    } else {
        // OpenAI-compatible (OpenAI, Ollama, custom)
        let mut body = serde_json::json!({
            "model": provider.model,
            "stream": false,
            "messages": messages,
        });
        if let Some(sp) = system_prompt {
            if !sp.trim().is_empty() {
                // Prepend system message
                let mut msgs = vec![serde_json::json!({"role":"system","content":sp})];
                if let Some(arr) = body["messages"].as_array() {
                    msgs.extend_from_slice(arr);
                }
                body["messages"] = serde_json::json!(msgs);
            }
        }
        let mut req = client
            .post(format!("{}/v1/chat/completions", provider.base_url))
            .header("content-type", "application/json");
        if !provider.api_key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", provider.api_key));
        }
        let resp = req.json(&body).send().await.map_err(|e| format!("Request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("API error {status}: {}", text.chars().take(300).collect::<String>()));
        }

        let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        data["choices"][0]["message"]["content"].as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "Unexpected API response shape".to_string())
    }
}

// ── Server entrypoint ─────────────────────────────────────────────────────────

pub async fn start(app: tauri::AppHandle, port: u16, lan: bool) {
    let addr = if lan {
        format!("0.0.0.0:{port}")
    } else {
        format!("127.0.0.1:{port}")
    };

    let ctx = Arc::new(Ctx { app });

    let router = Router::new()
        .route("/api/conversations", get(list_conversations_handler).post(create_conversation_handler))
        .route("/api/conversations/:id", get(get_conversation_handler))
        .route("/api/conversations/:id/messages", post(send_message_handler))
        .layer(CorsLayer::permissive())
        .with_state(ctx);

    match tokio::net::TcpListener::bind(&addr).await {
        Ok(listener) => {
            let _ = axum::serve(listener, router).await;
        }
        Err(e) => {
            eprintln!("LCD API server failed to bind {addr}: {e}");
        }
    }
}
