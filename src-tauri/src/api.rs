use crate::AppState;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{Emitter, Manager};

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const GITHUB_RELEASES_URL: &str = "https://api.github.com/repos/ponack/ubuntu-claude-desktop/releases/latest";

static STOP_FLAG: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ApiMessage {
    role: String,
    content: serde_json::Value,
}

#[derive(Debug, Serialize, Clone)]
struct StreamEvent {
    event: String, // "delta", "done", "error"
    content: String,
    message_id: String,
}

#[derive(Debug, Deserialize)]
pub struct Attachment {
    path: String,
    media_type: String,
}

fn encode_file_to_base64(path: &str) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;
    Ok(base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes))
}

fn build_content_blocks(text: &str, attachments: &[Attachment]) -> serde_json::Value {
    if attachments.is_empty() {
        return serde_json::json!(text);
    }

    let mut blocks: Vec<serde_json::Value> = Vec::new();

    for att in attachments {
        if let Ok(data) = encode_file_to_base64(&att.path) {
            blocks.push(serde_json::json!({
                "type": "image",
                "source": {
                    "type": "base64",
                    "media_type": &att.media_type,
                    "data": data
                }
            }));
        }
    }

    if !text.is_empty() {
        blocks.push(serde_json::json!({
            "type": "text",
            "text": text
        }));
    }

    serde_json::json!(blocks)
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
    attachments: Option<Vec<Attachment>>,
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
        let base_prompt = db.get_setting("system_prompt").map_err(|e| e.to_string())?;

        // Inject project context if conversation is in a project
        let project_context = db.get_conversation_project_id(&conversation_id)
            .ok()
            .flatten()
            .and_then(|pid| db.get_project_context(&pid).ok().flatten());

        match (base_prompt, project_context) {
            (Some(bp), Some(pc)) => Some(format!("{}\n\n---\nProject Context:\n{}", bp, pc)),
            (Some(bp), None) => Some(bp),
            (None, Some(pc)) => Some(format!("Project Context:\n{}", pc)),
            (None, None) => None,
        }
    };

    let atts = attachments.unwrap_or_default();

    // Build display text for DB (include attachment filenames)
    let display_content = if atts.is_empty() {
        content.clone()
    } else {
        let filenames: Vec<String> = atts.iter().map(|a| {
            std::path::Path::new(&a.path)
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| "file".to_string())
        }).collect();
        format!("[Attached: {}]\n{}", filenames.join(", "), content)
    };

    // Save user message
    let user_msg_id = uuid::Uuid::new_v4().to_string();
    {
        let db = state.db.lock().unwrap();
        db.insert_message(&user_msg_id, &conversation_id, "user", &display_content)
            .map_err(|e| e.to_string())?;
    }

    // Load conversation history
    let mut messages: Vec<ApiMessage> = {
        let db = state.db.lock().unwrap();
        db.list_messages(&conversation_id)
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|m| ApiMessage {
                role: m.role,
                content: serde_json::json!(m.content),
            })
            .collect()
    };

    // Replace the last user message content with multimodal blocks if there are attachments
    if !atts.is_empty() {
        if let Some(last) = messages.last_mut() {
            if last.role == "user" {
                last.content = build_content_blocks(&content, &atts);
            }
        }
    }

    // Create placeholder for assistant message
    let assistant_msg_id = uuid::Uuid::new_v4().to_string();
    {
        let db = state.db.lock().unwrap();
        db.insert_message(&assistant_msg_id, &conversation_id, "assistant", "")
            .map_err(|e| e.to_string())?;
    }

    let assistant_msg_id_clone = assistant_msg_id.clone();
    let app_clone = app.clone();
    // Load MCP server configs
    let mcp_configs: Vec<crate::mcp::McpServerConfig> = {
        let db = state.db.lock().unwrap();
        db.get_setting("mcp_servers")
            .ok()
            .flatten()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    };

    let state_inner = Arc::new((
        api_key,
        model,
        messages,
        conversation_id.clone(),
        system_prompt,
        mcp_configs,
    ));

    // Spawn streaming task
    let msg_id = assistant_msg_id.clone();
    tauri::async_runtime::spawn(async move {
        let (api_key, model, messages, _conversation_id, system_prompt, mcp_configs) = state_inner.as_ref();

        // Connect to MCP servers and collect tools
        let (mcp_tools, mut mcp_connections) = if !mcp_configs.is_empty() {
            crate::mcp::collect_tools(mcp_configs)
        } else {
            (Vec::new(), Vec::new())
        };
        let api_tools = crate::mcp::tools_to_api_format(&mcp_tools);

        let client = reqwest::Client::new();
        let mut current_messages = messages.clone();
        let mut full_content = String::new();

        // Tool use loop: Claude may request tool calls, we execute and continue
        loop {
            let mut body = serde_json::json!({
                "model": model,
                "max_tokens": 8192,
                "stream": true,
                "messages": current_messages,
            });

            if let Some(sp) = system_prompt {
                if !sp.trim().is_empty() {
                    body["system"] = serde_json::json!(sp);
                }
            }

            if !api_tools.is_empty() {
                body["tools"] = serde_json::json!(api_tools);
            }

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
                        break;
                    }

                    let mut stream = resp.bytes_stream();
                    let mut buffer = String::new();
                    let mut stop_reason = String::new();
                    let mut tool_use_blocks: Vec<serde_json::Value> = Vec::new();
                    let mut current_tool_id = String::new();
                    let mut current_tool_name = String::new();
                    let mut current_tool_input_json = String::new();

                    while let Some(chunk) = stream.next().await {
                        if STOP_FLAG.load(Ordering::SeqCst) {
                            let _ = app_clone.emit("stream-event", StreamEvent {
                                event: "done".into(),
                                content: String::new(),
                                message_id: msg_id.clone(),
                            });
                            // Clean up MCP connections
                            for conn in mcp_connections {
                                conn.disconnect();
                            }
                            return;
                        }

                        match chunk {
                            Ok(bytes) => {
                                buffer.push_str(&String::from_utf8_lossy(&bytes));

                                while let Some(pos) = buffer.find('\n') {
                                    let line = buffer[..pos].trim().to_string();
                                    buffer = buffer[pos + 1..].to_string();

                                    if line.starts_with("data: ") {
                                        let data = &line[6..];
                                        if data == "[DONE]" { continue; }

                                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                                            let event_type = parsed.get("type").and_then(|t| t.as_str()).unwrap_or("");

                                            match event_type {
                                                "content_block_start" => {
                                                    if let Some(cb) = parsed.get("content_block") {
                                                        if cb.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                                                            current_tool_id = cb.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                                            current_tool_name = cb.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                                            current_tool_input_json.clear();

                                                            // Show tool call in UI
                                                            let tool_msg = format!("\n\n🔧 *Calling tool: {}*\n", current_tool_name);
                                                            full_content.push_str(&tool_msg);
                                                            let _ = app_clone.emit("stream-event", StreamEvent {
                                                                event: "delta".into(),
                                                                content: tool_msg,
                                                                message_id: msg_id.clone(),
                                                            });
                                                        }
                                                    }
                                                }
                                                "content_block_delta" => {
                                                    if let Some(delta) = parsed.get("delta") {
                                                        if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                                                            full_content.push_str(text);
                                                            let _ = app_clone.emit("stream-event", StreamEvent {
                                                                event: "delta".into(),
                                                                content: text.to_string(),
                                                                message_id: msg_id.clone(),
                                                            });
                                                        }
                                                        // Accumulate tool input JSON
                                                        if let Some(partial) = delta.get("partial_json").and_then(|t| t.as_str()) {
                                                            current_tool_input_json.push_str(partial);
                                                        }
                                                    }
                                                }
                                                "content_block_stop" => {
                                                    if !current_tool_id.is_empty() {
                                                        let input: serde_json::Value = serde_json::from_str(&current_tool_input_json)
                                                            .unwrap_or(serde_json::json!({}));
                                                        tool_use_blocks.push(serde_json::json!({
                                                            "type": "tool_use",
                                                            "id": current_tool_id,
                                                            "name": current_tool_name,
                                                            "input": input
                                                        }));
                                                        current_tool_id.clear();
                                                        current_tool_name.clear();
                                                        current_tool_input_json.clear();
                                                    }
                                                }
                                                "message_delta" => {
                                                    if let Some(d) = parsed.get("delta") {
                                                        if let Some(sr) = d.get("stop_reason").and_then(|v| v.as_str()) {
                                                            stop_reason = sr.to_string();
                                                        }
                                                    }
                                                }
                                                "message_stop" => {
                                                    // Will be handled after the stream ends
                                                }
                                                _ => {}
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
                                for conn in mcp_connections {
                                    conn.disconnect();
                                }
                                return;
                            }
                        }
                    }

                    // If Claude requested tool use, execute tools and continue
                    if stop_reason == "tool_use" && !tool_use_blocks.is_empty() {
                        // Build assistant message with tool_use blocks
                        let mut assistant_content: Vec<serde_json::Value> = Vec::new();
                        if !full_content.is_empty() {
                            // Extract text before tool calls
                            let text_before_tools = full_content.split("\n\n🔧").next().unwrap_or("").trim();
                            if !text_before_tools.is_empty() {
                                assistant_content.push(serde_json::json!({"type": "text", "text": text_before_tools}));
                            }
                        }
                        assistant_content.extend(tool_use_blocks.clone());

                        current_messages.push(ApiMessage {
                            role: "assistant".into(),
                            content: serde_json::json!(assistant_content),
                        });

                        // Execute each tool call
                        let mut tool_results: Vec<serde_json::Value> = Vec::new();
                        for tool_block in &tool_use_blocks {
                            let tool_name = tool_block.get("name").and_then(|v| v.as_str()).unwrap_or("");
                            let tool_id = tool_block.get("id").and_then(|v| v.as_str()).unwrap_or("");
                            let tool_input = tool_block.get("input").cloned().unwrap_or(serde_json::json!({}));

                            // Find which MCP connection has this tool
                            let mut result_text = String::from("Tool not found");
                            let tool_info = mcp_tools.iter().find(|t| t.name == tool_name);

                            if let Some(info) = tool_info {
                                for conn in mcp_connections.iter_mut() {
                                    if conn.server_name == info.server_name {
                                        match conn.call_tool(tool_name, tool_input.clone()) {
                                            Ok(result) => {
                                                // Extract text content from MCP result
                                                if let Some(content_arr) = result.get("content").and_then(|c| c.as_array()) {
                                                    let texts: Vec<&str> = content_arr.iter()
                                                        .filter_map(|c| c.get("text").and_then(|t| t.as_str()))
                                                        .collect();
                                                    result_text = texts.join("\n");
                                                } else {
                                                    result_text = serde_json::to_string_pretty(&result).unwrap_or_default();
                                                }
                                            }
                                            Err(e) => {
                                                result_text = format!("Error: {}", e);
                                            }
                                        }
                                        break;
                                    }
                                }
                            }

                            // Show tool result in UI
                            let result_msg = format!("\n\n📋 *Result from {}:*\n```\n{}\n```\n", tool_name, &result_text[..result_text.len().min(500)]);
                            full_content.push_str(&result_msg);
                            let _ = app_clone.emit("stream-event", StreamEvent {
                                event: "delta".into(),
                                content: result_msg,
                                message_id: msg_id.clone(),
                            });

                            tool_results.push(serde_json::json!({
                                "type": "tool_result",
                                "tool_use_id": tool_id,
                                "content": result_text
                            }));
                        }

                        // Add tool results as user message
                        current_messages.push(ApiMessage {
                            role: "user".into(),
                            content: serde_json::json!(tool_results),
                        });

                        // Continue the loop — Claude will respond to tool results
                        continue;
                    }

                    // Normal end (stop_reason == "end_turn" or similar)
                    let _ = app_clone.emit("stream-event", StreamEvent {
                        event: "done".into(),
                        content: String::new(),
                        message_id: msg_id.clone(),
                    });

                    // Save final content to DB
                    if !full_content.is_empty() {
                        let db_state = app_clone.state::<AppState>();
                        let db = db_state.db.lock().unwrap();
                        let _ = db.update_message_content(&msg_id, &full_content);
                    }

                    break;
                }
                Err(e) => {
                    let _ = app_clone.emit("stream-event", StreamEvent {
                        event: "error".into(),
                        content: format!("Request failed: {}", e),
                        message_id: msg_id.clone(),
                    });
                    break;
                }
            }
        }

        // Clean up MCP connections
        for conn in mcp_connections {
            conn.disconnect();
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

    {
        let db = state.db.lock().unwrap();
        db.rename_conversation_by_id(&conversation_id, &title)
            .map_err(|e| e.to_string())?;
    }

    Ok(title)
}

#[derive(Debug, Serialize)]
pub struct UpdateInfo {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub download_url: String,
    pub release_notes: String,
}

#[tauri::command]
pub async fn check_for_updates() -> Result<UpdateInfo, String> {
    let client = reqwest::Client::new();
    let resp = client
        .get(GITHUB_RELEASES_URL)
        .header("User-Agent", "ubuntu-claude-desktop")
        .send()
        .await
        .map_err(|e| format!("Failed to check for updates: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("GitHub API returned {}", resp.status()));
    }

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    let latest_tag = json["tag_name"]
        .as_str()
        .unwrap_or("")
        .trim_start_matches('v')
        .to_string();

    let download_url = json["html_url"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let release_notes = json["body"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let has_update = !latest_tag.is_empty() && latest_tag != CURRENT_VERSION;

    Ok(UpdateInfo {
        has_update,
        current_version: CURRENT_VERSION.to_string(),
        latest_version: if latest_tag.is_empty() { CURRENT_VERSION.to_string() } else { latest_tag },
        download_url,
        release_notes,
    })
}
