use crate::AppState;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{Emitter, Manager};

static STOP_FLAG: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Serialize, Deserialize)]
struct ApiMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize, Clone)]
struct StreamEvent {
    event: String, // "delta", "done", "error"
    content: String,
    message_id: String,
}

#[tauri::command]
pub fn stop_generation() {
    STOP_FLAG.store(true, Ordering::SeqCst);
}

#[tauri::command]
pub async fn send_message(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    conversation_id: String,
    content: String,
) -> Result<String, String> {
    STOP_FLAG.store(false, Ordering::SeqCst);

    let api_key = {
        let db = state.db.lock().unwrap();
        db.get_setting("api_key")
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "API key not set. Please add your key in Settings.".to_string())?
    };

    let model = {
        let db = state.db.lock().unwrap();
        db.get_setting("model")
            .map_err(|e| e.to_string())?
            .unwrap_or_else(|| "claude-sonnet-4-6".to_string())
    };

    let system_prompt = {
        let db = state.db.lock().unwrap();
        db.get_setting("system_prompt")
            .map_err(|e| e.to_string())?
    };

    // Save user message
    let user_msg_id = uuid::Uuid::new_v4().to_string();
    {
        let db = state.db.lock().unwrap();
        db.insert_message(&user_msg_id, &conversation_id, "user", &content)
            .map_err(|e| e.to_string())?;
    }

    // Load conversation history
    let messages: Vec<ApiMessage> = {
        let db = state.db.lock().unwrap();
        db.list_messages(&conversation_id)
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|m| ApiMessage {
                role: m.role,
                content: m.content,
            })
            .collect()
    };

    // Create placeholder for assistant message
    let assistant_msg_id = uuid::Uuid::new_v4().to_string();
    {
        let db = state.db.lock().unwrap();
        db.insert_message(&assistant_msg_id, &conversation_id, "assistant", "")
            .map_err(|e| e.to_string())?;
    }

    let assistant_msg_id_clone = assistant_msg_id.clone();
    let app_clone = app.clone();
    let state_inner = Arc::new((
        api_key,
        model,
        messages,
        conversation_id.clone(),
        system_prompt,
    ));

    // Spawn streaming task
    let msg_id = assistant_msg_id.clone();
    tauri::async_runtime::spawn(async move {
        let (api_key, model, messages, _conversation_id, system_prompt) = state_inner.as_ref();

        let mut body = serde_json::json!({
            "model": model,
            "max_tokens": 8192,
            "stream": true,
            "messages": messages,
        });

        if let Some(sp) = system_prompt {
            if !sp.trim().is_empty() {
                body["system"] = serde_json::json!(sp);
            }
        }

        let client = reqwest::Client::new();
        let response = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key.as_str())
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .body(body.to_string())
            .send()
            .await;

        match response {
            Ok(resp) => {
                if !resp.status().is_success() {
                    let status = resp.status();
                    let error_body = resp.text().await.unwrap_or_default();
                    let _ = app_clone.emit("stream-event", StreamEvent {
                        event: "error".into(),
                        content: format!("API error {}: {}", status, error_body),
                        message_id: msg_id.clone(),
                    });
                    return;
                }

                let mut stream = resp.bytes_stream();
                let mut full_content = String::new();
                let mut buffer = String::new();

                while let Some(chunk) = stream.next().await {
                    if STOP_FLAG.load(Ordering::SeqCst) {
                        let _ = app_clone.emit("stream-event", StreamEvent {
                            event: "done".into(),
                            content: String::new(),
                            message_id: msg_id.clone(),
                        });
                        break;
                    }

                    match chunk {
                        Ok(bytes) => {
                            buffer.push_str(&String::from_utf8_lossy(&bytes));

                            // Process SSE lines
                            while let Some(pos) = buffer.find('\n') {
                                let line = buffer[..pos].trim().to_string();
                                buffer = buffer[pos + 1..].to_string();

                                if line.starts_with("data: ") {
                                    let data = &line[6..];
                                    if data == "[DONE]" {
                                        continue;
                                    }
                                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                                        // Extract text delta
                                        if let Some(delta) = parsed.get("delta") {
                                            if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                                                full_content.push_str(text);
                                                let _ = app_clone.emit("stream-event", StreamEvent {
                                                    event: "delta".into(),
                                                    content: text.to_string(),
                                                    message_id: msg_id.clone(),
                                                });
                                            }
                                        }
                                        // Check for message_stop
                                        if parsed.get("type").and_then(|t| t.as_str()) == Some("message_stop") {
                                            let _ = app_clone.emit("stream-event", StreamEvent {
                                                event: "done".into(),
                                                content: String::new(),
                                                message_id: msg_id.clone(),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            let _ = app_clone.emit("stream-event", StreamEvent {
                                event: "error".into(),
                                content: format!("Stream error: {}", e),
                                message_id: msg_id.clone(),
                            });
                            break;
                        }
                    }
                }

                // Save final content to DB
                if !full_content.is_empty() {
                    let db_state = app_clone.state::<AppState>();
                    let db = db_state.db.lock().unwrap();
                    let _ = db.update_message_content(&msg_id, &full_content);
                }
            }
            Err(e) => {
                let _ = app_clone.emit("stream-event", StreamEvent {
                    event: "error".into(),
                    content: format!("Request failed: {}", e),
                    message_id: msg_id.clone(),
                });
            }
        }
    });

    Ok(assistant_msg_id_clone)
}

#[tauri::command]
pub async fn generate_title(
    state: tauri::State<'_, AppState>,
    conversation_id: String,
    user_message: String,
) -> Result<String, String> {
    let api_key = {
        let db = state.db.lock().unwrap();
        db.get_setting("api_key")
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "API key not set".to_string())?
    };

    let body = serde_json::json!({
        "model": "claude-haiku-4-5-20251001",
        "max_tokens": 30,
        "messages": [{
            "role": "user",
            "content": format!(
                "Generate a short title (max 6 words, no quotes) for a conversation that starts with: {}",
                if user_message.len() > 200 { &user_message[..200] } else { &user_message }
            )
        }]
    });

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .body(body.to_string())
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let title = json["content"][0]["text"]
        .as_str()
        .unwrap_or("New Conversation")
        .trim()
        .trim_matches('"')
        .to_string();

    // Update the conversation title in DB
    {
        let db = state.db.lock().unwrap();
        db.rename_conversation_by_id(&conversation_id, &title)
            .map_err(|e| e.to_string())?;
    }

    Ok(title)
}
