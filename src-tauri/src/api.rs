use crate::AppState;
use crate::providers::{ProviderType, ResolvedProvider};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const GITHUB_RELEASES_URL: &str = "https://api.github.com/repos/ponack/linux-claude-desktop/releases/latest";

static STOP_FLAG: AtomicBool = AtomicBool::new(false);
static RECORDING_CHILD: Mutex<Option<std::process::Child>> = Mutex::new(None);

/// Compare semver strings: returns true if `latest` is newer than `current`
fn version_is_newer(latest: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u64> {
        v.split('.').filter_map(|p| p.parse().ok()).collect()
    };
    let l = parse(latest);
    let c = parse(current);
    for i in 0..l.len().max(c.len()) {
        let lv = l.get(i).copied().unwrap_or(0);
        let cv = c.get(i).copied().unwrap_or(0);
        if lv > cv { return true; }
        if lv < cv { return false; }
    }
    false
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ApiMessage {
    role: String,
    content: serde_json::Value,
}

#[derive(Debug, Serialize, Clone)]
struct StreamEvent {
    event: String,
    content: String,
    message_id: String,
}

#[derive(Debug, Serialize, Clone)]
struct UsageEvent {
    message_id: String,
    input_tokens: i64,
    output_tokens: i64,
    model: String,
}

#[derive(Debug, Deserialize)]
pub struct Attachment {
    path: Option<String>,
    media_type: String,
    data: Option<String>,
}

pub fn encode_file_to_base64(path: &str) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;
    Ok(base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes))
}

fn build_content_blocks(text: &str, attachments: &[Attachment]) -> serde_json::Value {
    if attachments.is_empty() {
        return serde_json::json!(text);
    }

    let mut blocks: Vec<serde_json::Value> = Vec::new();

    for att in attachments {
        let b64 = if let Some(ref data) = att.data {
            Some(data.clone())
        } else if let Some(ref path) = att.path {
            encode_file_to_base64(path).ok()
        } else {
            None
        };
        if let Some(data) = b64 {
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

/// Resolve the active provider config from settings, with optional project overrides
fn resolve_provider(state: &AppState, project_id: Option<&str>) -> Result<ResolvedProvider, String> {
    let db = state.db.lock().unwrap();

    // Check for project-level overrides
    let project = project_id
        .and_then(|pid| db.get_project(pid).ok().flatten());

    let provider_str = project.as_ref()
        .and_then(|p| p.provider.clone())
        .or_else(|| db.get_setting("provider").ok().flatten())
        .unwrap_or_else(|| "anthropic".to_string());
    let provider_type = ProviderType::from_str(&provider_str);

    match provider_type {
        ProviderType::Anthropic => {
            let api_key = project.as_ref()
                .and_then(|p| p.api_key.clone())
                .or_else(|| db.get_setting("api_key").ok().flatten())
                .ok_or_else(|| "API key not set. Please add your Anthropic key in Settings.".to_string())?;
            let model = project.as_ref()
                .and_then(|p| p.model.clone())
                .or_else(|| db.get_setting("model").ok().flatten())
                .unwrap_or_else(|| "claude-sonnet-4-6".to_string());
            Ok(ResolvedProvider {
                provider_type: ProviderType::Anthropic,
                api_key,
                base_url: "https://api.anthropic.com".to_string(),
                model,
                api_format: "anthropic".to_string(),
                endpoint_id: None,
            })
        }
        ProviderType::OpenAI => {
            let api_key = project.as_ref()
                .and_then(|p| p.api_key.clone())
                .or_else(|| db.get_setting("openai_api_key").ok().flatten())
                .ok_or_else(|| "OpenAI API key not set. Please add your key in Settings.".to_string())?;
            let base_url = db.get_setting("openai_base_url").map_err(|e| e.to_string())?
                .unwrap_or_else(|| "https://api.openai.com".to_string());
            let model = project.as_ref()
                .and_then(|p| p.model.clone())
                .or_else(|| db.get_setting("model").ok().flatten())
                .unwrap_or_else(|| "gpt-4o".to_string());
            Ok(ResolvedProvider {
                provider_type: ProviderType::OpenAI,
                api_key,
                base_url,
                model,
                api_format: "openai".to_string(),
                endpoint_id: None,
            })
        }
        ProviderType::Ollama => {
            let base_url = db.get_setting("ollama_base_url").map_err(|e| e.to_string())?
                .unwrap_or_else(|| "http://localhost:11434".to_string());
            let model = project.as_ref()
                .and_then(|p| p.model.clone())
                .or_else(|| db.get_setting("model").ok().flatten())
                .unwrap_or_else(|| "llama3.2".to_string());
            Ok(ResolvedProvider {
                provider_type: ProviderType::Ollama,
                api_key: String::new(),
                base_url,
                model,
                api_format: "openai".to_string(),
                endpoint_id: None,
            })
        }
        ProviderType::Custom => {
            // Custom endpoint: look up endpoint_id from settings
            let endpoint_id = db.get_setting("custom_endpoint_id").map_err(|e| e.to_string())?
                .ok_or_else(|| "No custom endpoint selected. Configure one in Settings.".to_string())?;
            let endpoint = db.get_custom_endpoint(&endpoint_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "Custom endpoint not found.".to_string())?;
            let model = project.as_ref()
                .and_then(|p| p.model.clone())
                .or_else(|| db.get_setting("model").ok().flatten())
                .unwrap_or_else(|| endpoint.default_model.clone());
            Ok(ResolvedProvider {
                provider_type: ProviderType::Custom,
                api_key: endpoint.api_key,
                base_url: endpoint.base_url,
                model,
                api_format: endpoint.api_format,
                endpoint_id: Some(endpoint.id),
            })
        }
    }
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

    // Get project ID for this conversation
    let project_id = {
        let db = state.db.lock().unwrap();
        db.get_conversation_project_id(&conversation_id).ok().flatten()
    };

    let provider = resolve_provider(&state, project_id.as_deref())?;

    let system_prompt = {
        let db = state.db.lock().unwrap();
        let base_prompt = db.get_setting("system_prompt").map_err(|e| e.to_string())?;

        // Project-level system prompt override or context
        let (project_sys_prompt, project_context) = match &project_id {
            Some(pid) => {
                let proj = db.get_project(pid).ok().flatten();
                let sys = proj.as_ref().and_then(|p| p.system_prompt.clone()).filter(|s| !s.is_empty());
                let ctx = db.get_project_context(pid).ok().flatten();
                (sys, ctx)
            }
            None => (None, None),
        };

        // If project has its own system prompt, use it instead of the global one
        let effective_prompt = project_sys_prompt.or(base_prompt);

        let base = match (effective_prompt, project_context) {
            (Some(bp), Some(pc)) => Some(format!("{}\n\n---\nProject Context:\n{}", bp, pc)),
            (Some(bp), None) => Some(bp),
            (None, Some(pc)) => Some(format!("Project Context:\n{}", pc)),
            (None, None) => None,
        };

        // Inject memory entries
        let memory_entries = db.list_memory_entries().unwrap_or_default();
        let with_memory = if !memory_entries.is_empty() {
            let mut mem_section = String::from("\n\n---\nUser Memory (persistent facts):");
            for entry in &memory_entries {
                mem_section.push_str(&format!("\n- {}: {}", entry.key, entry.value));
            }
            match base {
                Some(b) => Some(format!("{}{}", b, mem_section)),
                None => Some(mem_section.trim_start().to_string()),
            }
        } else {
            base
        };

        // Inject enabled knowledge base entries
        let knowledge_entries = db.get_enabled_knowledge_for_context(project_id.as_deref()).unwrap_or_default();
        if !knowledge_entries.is_empty() {
            let mut kb_section = String::from("\n\n---\nKnowledge Base:");
            for entry in &knowledge_entries {
                kb_section.push_str(&format!("\n## {}\n{}", entry.title, entry.content));
            }
            match with_memory {
                Some(b) => Some(format!("{}{}", b, kb_section)),
                None => Some(kb_section.trim_start().to_string()),
            }
        } else {
            with_memory
        }
    };

    let atts = attachments.unwrap_or_default();

    let display_content = if atts.is_empty() {
        content.clone()
    } else {
        let filenames: Vec<String> = atts.iter().map(|a| {
            if let Some(ref path) = a.path {
                std::path::Path::new(path)
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_else(|| "image".to_string())
            } else {
                "pasted-image".to_string()
            }
        }).collect();
        format!("[Attached: {}]\n{}", filenames.join(", "), content)
    };

    let user_msg_id = uuid::Uuid::new_v4().to_string();
    {
        let db = state.db.lock().unwrap();
        db.insert_message(&user_msg_id, &conversation_id, "user", &display_content)
            .map_err(|e| e.to_string())?;
    }

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

    if !atts.is_empty() {
        if let Some(last) = messages.last_mut() {
            if last.role == "user" {
                last.content = build_content_blocks(&content, &atts);
            }
        }
    }

    let assistant_msg_id = uuid::Uuid::new_v4().to_string();
    {
        let db = state.db.lock().unwrap();
        db.insert_message(&assistant_msg_id, &conversation_id, "assistant", "")
            .map_err(|e| e.to_string())?;
    }

    let assistant_msg_id_clone = assistant_msg_id.clone();
    let app_clone = app.clone();

    let mcp_configs: Vec<crate::mcp::McpServerConfig> = if provider.api_format == "anthropic" {
        let db = state.db.lock().unwrap();
        db.get_setting("mcp_servers")
            .ok()
            .flatten()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let state_inner = Arc::new((
        provider,
        messages,
        conversation_id.clone(),
        system_prompt,
        mcp_configs,
    ));

    let msg_id = assistant_msg_id.clone();
    tauri::async_runtime::spawn(async move {
        let (provider, messages, conversation_id, system_prompt, mcp_configs) = state_inner.as_ref();

        if provider.api_format == "anthropic" {
            stream_anthropic(&app_clone, &msg_id, conversation_id, provider, messages, system_prompt, mcp_configs).await;
        } else {
            stream_openai_compatible(&app_clone, &msg_id, conversation_id, provider, messages, system_prompt).await;
        }
    });

    Ok(assistant_msg_id_clone)
}

async fn stream_anthropic(
    app: &tauri::AppHandle,
    msg_id: &str,
    conversation_id: &str,
    provider: &ResolvedProvider,
    messages: &[ApiMessage],
    system_prompt: &Option<String>,
    mcp_configs: &[crate::mcp::McpServerConfig],
) {
    let (mcp_tools, mut mcp_connections) = if !mcp_configs.is_empty() {
        crate::mcp::collect_tools(mcp_configs)
    } else {
        (Vec::new(), Vec::new())
    };
    let api_tools = crate::mcp::tools_to_api_format(&mcp_tools);

    let client = reqwest::Client::new();
    let mut current_messages = messages.to_vec();
    let mut full_content = String::new();
    let mut total_input_tokens: i64 = 0;
    let mut total_output_tokens: i64 = 0;

    loop {
        let mut body = serde_json::json!({
            "model": provider.model,
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
            .header("x-api-key", provider.api_key.as_str())
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
                    let _ = app.emit("stream-event", StreamEvent {
                        event: "error".into(),
                        content: format!("API error {}: {}", status, error_body),
                        message_id: msg_id.to_string(),
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
                        if !full_content.is_empty() {
                            let db_state = app.state::<AppState>();
                            let db = db_state.db.lock().unwrap();
                            let _ = db.update_message_content(msg_id, &full_content);
                        }
                        let _ = app.emit("stream-event", StreamEvent {
                            event: "done".into(),
                            content: String::new(),
                            message_id: msg_id.to_string(),
                        });
                        for conn in mcp_connections { conn.disconnect(); }
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
                                            "message_start" => {
                                                if let Some(usage) = parsed.get("message").and_then(|m| m.get("usage")) {
                                                    if let Some(it) = usage.get("input_tokens").and_then(|v| v.as_i64()) {
                                                        total_input_tokens += it;
                                                    }
                                                }
                                            }
                                            "content_block_start" => {
                                                if let Some(cb) = parsed.get("content_block") {
                                                    if cb.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                                                        current_tool_id = cb.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                                        current_tool_name = cb.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                                        current_tool_input_json.clear();

                                                        let tool_msg = format!("\n\n**Calling tool: {}**\n", current_tool_name);
                                                        full_content.push_str(&tool_msg);
                                                        let _ = app.emit("stream-event", StreamEvent {
                                                            event: "delta".into(),
                                                            content: tool_msg,
                                                            message_id: msg_id.to_string(),
                                                        });
                                                    }
                                                }
                                            }
                                            "content_block_delta" => {
                                                if let Some(delta) = parsed.get("delta") {
                                                    if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                                                        full_content.push_str(text);
                                                        let _ = app.emit("stream-event", StreamEvent {
                                                            event: "delta".into(),
                                                            content: text.to_string(),
                                                            message_id: msg_id.to_string(),
                                                        });
                                                    }
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
                                                if let Some(usage) = parsed.get("usage") {
                                                    if let Some(ot) = usage.get("output_tokens").and_then(|v| v.as_i64()) {
                                                        total_output_tokens = ot;
                                                    }
                                                }
                                            }
                                            "message_stop" => {}
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            // Save partial content before reporting error
                            if !full_content.is_empty() {
                                let db_state = app.state::<AppState>();
                                let db = db_state.db.lock().unwrap();
                                let _ = db.update_message_content(msg_id, &full_content);
                            }
                            let _ = app.emit("stream-event", StreamEvent {
                                event: "error".into(),
                                content: format!("Stream error: {}", e),
                                message_id: msg_id.to_string(),
                            });
                            for conn in mcp_connections { conn.disconnect(); }
                            return;
                        }
                    }
                }

                if stop_reason == "tool_use" && !tool_use_blocks.is_empty() {
                    let mut assistant_content: Vec<serde_json::Value> = Vec::new();
                    if !full_content.is_empty() {
                        let text_before_tools = full_content.split("\n\n**Calling tool:").next().unwrap_or("").trim();
                        if !text_before_tools.is_empty() {
                            assistant_content.push(serde_json::json!({"type": "text", "text": text_before_tools}));
                        }
                    }
                    assistant_content.extend(tool_use_blocks.clone());

                    current_messages.push(ApiMessage {
                        role: "assistant".into(),
                        content: serde_json::json!(assistant_content),
                    });

                    let mut tool_results: Vec<serde_json::Value> = Vec::new();
                    for tool_block in &tool_use_blocks {
                        let tool_name = tool_block.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        let tool_id = tool_block.get("id").and_then(|v| v.as_str()).unwrap_or("");
                        let tool_input = tool_block.get("input").cloned().unwrap_or(serde_json::json!({}));

                        let mut result_text = String::from("Tool not found");
                        let tool_info = mcp_tools.iter().find(|t| t.name == tool_name);

                        if let Some(info) = tool_info {
                            for conn in mcp_connections.iter_mut() {
                                if conn.server_name == info.server_name {
                                    match conn.call_tool(tool_name, tool_input.clone()) {
                                        Ok(result) => {
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

                        let result_msg = format!("\n\n**Result from {}:**\n```\n{}\n```\n", tool_name, &result_text[..result_text.len().min(500)]);
                        full_content.push_str(&result_msg);
                        let _ = app.emit("stream-event", StreamEvent {
                            event: "delta".into(),
                            content: result_msg,
                            message_id: msg_id.to_string(),
                        });

                        tool_results.push(serde_json::json!({
                            "type": "tool_result",
                            "tool_use_id": tool_id,
                            "content": result_text
                        }));
                    }

                    current_messages.push(ApiMessage {
                        role: "user".into(),
                        content: serde_json::json!(tool_results),
                    });

                    continue;
                }

                // Save token usage
                if total_input_tokens > 0 || total_output_tokens > 0 {
                    let db_state = app.state::<AppState>();
                    let db = db_state.db.lock().unwrap();
                    let _ = db.insert_token_usage(conversation_id, msg_id, total_input_tokens, total_output_tokens, &provider.model);
                    let _ = app.emit("token-usage", UsageEvent {
                        message_id: msg_id.to_string(),
                        input_tokens: total_input_tokens,
                        output_tokens: total_output_tokens,
                        model: provider.model.clone(),
                    });
                }

                let _ = app.emit("stream-event", StreamEvent {
                    event: "done".into(),
                    content: String::new(),
                    message_id: msg_id.to_string(),
                });

                if !full_content.is_empty() {
                    let db_state = app.state::<AppState>();
                    let db = db_state.db.lock().unwrap();
                    let _ = db.update_message_content(msg_id, &full_content);
                }

                break;
            }
            Err(e) => {
                let _ = app.emit("stream-event", StreamEvent {
                    event: "error".into(),
                    content: format!("Request failed: {}", e),
                    message_id: msg_id.to_string(),
                });
                break;
            }
        }
    }

    for conn in mcp_connections {
        conn.disconnect();
    }
}

async fn stream_openai_compatible(
    app: &tauri::AppHandle,
    msg_id: &str,
    conversation_id: &str,
    provider: &ResolvedProvider,
    messages: &[ApiMessage],
    system_prompt: &Option<String>,
) {
    let client = reqwest::Client::new();

    let mut oai_messages: Vec<serde_json::Value> = Vec::new();

    if let Some(sp) = system_prompt {
        if !sp.trim().is_empty() {
            oai_messages.push(serde_json::json!({"role": "system", "content": sp}));
        }
    }

    for msg in messages {
        let content_str = if let Some(s) = msg.content.as_str() {
            s.to_string()
        } else {
            msg.content.to_string()
        };
        oai_messages.push(serde_json::json!({"role": msg.role, "content": content_str}));
    }

    let body = serde_json::json!({
        "model": provider.model,
        "stream": true,
        "stream_options": {"include_usage": true},
        "messages": oai_messages,
    });

    let url = format!("{}/v1/chat/completions", provider.base_url.trim_end_matches('/'));

    let mut req = client
        .post(&url)
        .header("content-type", "application/json");

    if !provider.api_key.is_empty() {
        req = req.header("authorization", format!("Bearer {}", provider.api_key));
    }

    let response = req.body(body.to_string()).send().await;

    match response {
        Ok(resp) => {
            if !resp.status().is_success() {
                let status = resp.status();
                let error_body = resp.text().await.unwrap_or_default();
                let _ = app.emit("stream-event", StreamEvent {
                    event: "error".into(),
                    content: format!("API error {}: {}", status, error_body),
                    message_id: msg_id.to_string(),
                });
                return;
            }

            let mut stream = resp.bytes_stream();
            let mut buffer = String::new();
            let mut full_content = String::new();
            let mut total_input_tokens: i64 = 0;
            let mut total_output_tokens: i64 = 0;

            while let Some(chunk) = stream.next().await {
                if STOP_FLAG.load(Ordering::SeqCst) {
                    if !full_content.is_empty() {
                        let db_state = app.state::<AppState>();
                        let db = db_state.db.lock().unwrap();
                        let _ = db.update_message_content(msg_id, &full_content);
                    }
                    let _ = app.emit("stream-event", StreamEvent {
                        event: "done".into(),
                        content: String::new(),
                        message_id: msg_id.to_string(),
                    });
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
                                    if let Some(choices) = parsed.get("choices").and_then(|c| c.as_array()) {
                                        if let Some(first) = choices.first() {
                                            if let Some(delta) = first.get("delta") {
                                                if let Some(text) = delta.get("content").and_then(|c| c.as_str()) {
                                                    full_content.push_str(text);
                                                    let _ = app.emit("stream-event", StreamEvent {
                                                        event: "delta".into(),
                                                        content: text.to_string(),
                                                        message_id: msg_id.to_string(),
                                                    });
                                                }
                                            }
                                        }
                                    }
                                    // Capture usage from final chunk
                                    if let Some(usage) = parsed.get("usage") {
                                        if let Some(pt) = usage.get("prompt_tokens").and_then(|v| v.as_i64()) {
                                            total_input_tokens = pt;
                                        }
                                        if let Some(ct) = usage.get("completion_tokens").and_then(|v| v.as_i64()) {
                                            total_output_tokens = ct;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        // Save partial content before reporting error
                        if !full_content.is_empty() {
                            let db_state = app.state::<AppState>();
                            let db = db_state.db.lock().unwrap();
                            let _ = db.update_message_content(msg_id, &full_content);
                        }
                        let _ = app.emit("stream-event", StreamEvent {
                            event: "error".into(),
                            content: format!("Stream error: {}", e),
                            message_id: msg_id.to_string(),
                        });
                        return;
                    }
                }
            }

            // Save token usage
            if total_input_tokens > 0 || total_output_tokens > 0 {
                let db_state = app.state::<AppState>();
                let db = db_state.db.lock().unwrap();
                let _ = db.insert_token_usage(conversation_id, msg_id, total_input_tokens, total_output_tokens, &provider.model);
                let _ = app.emit("token-usage", UsageEvent {
                    message_id: msg_id.to_string(),
                    input_tokens: total_input_tokens,
                    output_tokens: total_output_tokens,
                    model: provider.model.clone(),
                });
            }

            let _ = app.emit("stream-event", StreamEvent {
                event: "done".into(),
                content: String::new(),
                message_id: msg_id.to_string(),
            });

            if !full_content.is_empty() {
                let db_state = app.state::<AppState>();
                let db = db_state.db.lock().unwrap();
                let _ = db.update_message_content(msg_id, &full_content);
            }
        }
        Err(e) => {
            let _ = app.emit("stream-event", StreamEvent {
                event: "error".into(),
                content: format!("Request failed: {}", e),
                message_id: msg_id.to_string(),
            });
        }
    }
}

#[tauri::command]
pub async fn generate_title(
    state: tauri::State<'_, AppState>,
    conversation_id: String,
    user_message: String,
) -> Result<String, String> {
    let project_id = {
        let db = state.db.lock().unwrap();
        db.get_conversation_project_id(&conversation_id).ok().flatten()
    };
    let provider = resolve_provider(&state, project_id.as_deref())?;

    let prompt = format!(
        "Generate a short title (max 6 words, no quotes) for a conversation that starts with: {}",
        if user_message.len() > 200 { &user_message[..200] } else { &user_message }
    );

    let client = reqwest::Client::new();

    let title = if provider.api_format == "anthropic" {
        {
            // Use Haiku for title generation — fast and cheap
            let title_model = if provider.model.contains("haiku") {
                provider.model.clone()
            } else {
                "claude-haiku-4-5-20251001".to_string()
            };
            let body = serde_json::json!({
                "model": title_model,
                "max_tokens": 30,
                "messages": [{"role": "user", "content": prompt}]
            });

            let resp = client
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", &provider.api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .body(body.to_string())
                .send()
                .await
                .map_err(|e| e.to_string())?;

            let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
            json["content"][0]["text"]
                .as_str()
                .unwrap_or("New Conversation")
                .trim()
                .trim_matches('"')
                .to_string()
        }
    } else {
        {
            let body = serde_json::json!({
                "model": provider.model,
                "max_tokens": 30,
                "messages": [
                    {"role": "system", "content": "You generate short conversation titles. Respond with ONLY the title, no quotes, max 6 words."},
                    {"role": "user", "content": prompt}
                ]
            });

            let url = format!("{}/v1/chat/completions", provider.base_url.trim_end_matches('/'));
            let mut req = client.post(&url).header("content-type", "application/json");
            if !provider.api_key.is_empty() {
                req = req.header("authorization", format!("Bearer {}", provider.api_key));
            }

            let resp = req.body(body.to_string()).send().await.map_err(|e| e.to_string())?;
            let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
            json["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("New Conversation")
                .trim()
                .trim_matches('"')
                .to_string()
        }
    };

    {
        let db = state.db.lock().unwrap();
        db.rename_conversation_by_id(&conversation_id, &title)
            .map_err(|e| e.to_string())?;
    }

    Ok(title)
}

/// Execute a custom command and return its output
#[tauri::command]
pub async fn run_custom_command(command: String) -> Result<String, String> {
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(&command)
        .output()
        .map_err(|e| format!("Failed to execute command: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(stdout)
    } else {
        Ok(format!("{}\n{}", stdout, stderr))
    }
}

#[derive(Debug, Serialize)]
pub struct UpdateInfo {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub download_url: String,
    pub release_notes: String,
    pub deb_asset_url: String,
}

#[tauri::command]
pub async fn check_for_updates() -> Result<UpdateInfo, String> {
    let client = reqwest::Client::new();
    let resp = client
        .get(GITHUB_RELEASES_URL)
        .header("User-Agent", "linux-claude-desktop")
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

    // Find the .deb asset URL
    let deb_asset_url = json["assets"]
        .as_array()
        .and_then(|assets| {
            assets.iter().find(|a| {
                a["name"].as_str().map(|n| n.ends_with(".deb")).unwrap_or(false)
            })
        })
        .and_then(|a| a["browser_download_url"].as_str())
        .unwrap_or("")
        .to_string();

    let has_update = !latest_tag.is_empty() && version_is_newer(&latest_tag, CURRENT_VERSION);

    Ok(UpdateInfo {
        has_update,
        current_version: CURRENT_VERSION.to_string(),
        latest_version: if latest_tag.is_empty() { CURRENT_VERSION.to_string() } else { latest_tag },
        download_url,
        release_notes,
        deb_asset_url,
    })
}

/// Download a .deb update to a temp file and return the path
#[tauri::command]
pub async fn download_update(app: tauri::AppHandle, url: String) -> Result<String, String> {
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("User-Agent", "linux-claude-desktop")
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Download failed: HTTP {}", resp.status()));
    }

    let total_size = resp.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;

    let temp_dir = std::env::temp_dir();
    let filename = url.split('/').last().unwrap_or("update.deb");
    let dest_path = temp_dir.join(filename);

    let mut file = std::fs::File::create(&dest_path)
        .map_err(|e| format!("Failed to create temp file: {}", e))?;

    let mut stream = resp.bytes_stream();
    use std::io::Write;

    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|e| format!("Download stream error: {}", e))?;
        file.write_all(&bytes).map_err(|e| format!("Write error: {}", e))?;
        downloaded += bytes.len() as u64;

        if total_size > 0 {
            let progress = (downloaded as f64 / total_size as f64 * 100.0) as u32;
            let _ = app.emit("update-progress", progress);
        }
    }

    Ok(dest_path.to_string_lossy().to_string())
}

/// Install a .deb package using pkexec (polkit elevation)
#[tauri::command]
pub async fn install_update(deb_path: String) -> Result<(), String> {
    // Verify file exists
    if !std::path::Path::new(&deb_path).exists() {
        return Err("Update file not found".to_string());
    }

    // Build a script that installs the new .deb and removes the old package
    // in a single elevated session to avoid a double auth prompt
    let script = format!(
        r#"#!/bin/sh
dpkg -i "{deb}" || exit 1
dpkg -s ubuntu-claude-desktop >/dev/null 2>&1 && dpkg --remove ubuntu-claude-desktop
exit 0
"#,
        deb = deb_path.replace('"', r#"\""#),
    );
    let script_path = format!("/tmp/lcd-update-{}.sh", std::process::id());
    std::fs::write(&script_path, &script)
        .map_err(|e| format!("Failed to write update script: {}", e))?;

    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o700))
        .map_err(|e| format!("Failed to set script permissions: {}", e))?;

    let output = std::process::Command::new("pkexec")
        .arg(&script_path)
        .output()
        .map_err(|e| format!("Failed to launch installer: {}", e))?;

    // Clean up scripts and temp file
    let _ = std::fs::remove_file(&script_path);
    let _ = std::fs::remove_file(&deb_path);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Installation failed: {}", stderr));
    }

    Ok(())
}

/// Find the installed binary by querying dpkg for our package's executables.
/// Returns the first /usr/bin/* match, which is the main app binary.
fn find_installed_binary() -> Option<String> {
    // Try the known binary name first
    let known = "/usr/bin/linux-claude-desktop";
    if std::path::Path::new(known).exists() {
        return Some(known.to_string());
    }

    // Fallback: search dpkg for any package matching our app name pattern
    let output = std::process::Command::new("dpkg")
        .args(["-S", "*/bin/linux-claude-desktop"])
        .output()
        .ok()?;
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Format: "package-name: /usr/bin/binary-name"
        if let Some(path) = stdout.split_whitespace().last() {
            if std::path::Path::new(path).exists() {
                return Some(path.to_string());
            }
        }
    }

    None
}

/// Restart the application by writing a relaunch script and executing it fully detached
#[tauri::command]
pub async fn restart_app(_app: tauri::AppHandle) -> Result<(), String> {
    // After an update, the binary name may have changed (e.g. rebrand).
    // Query dpkg for the actual installed binary, falling back to current_exe.
    let exe_str = find_installed_binary().unwrap_or_else(|| {
        let exe = std::env::current_exe().unwrap_or_default();
        exe.to_string_lossy().replace(" (deleted)", "")
    });
    let pid = std::process::id();

    let script = format!(
        r#"#!/bin/sh
exec > /tmp/lcd-restart.log 2>&1
EXE_PATH=$(echo "{exe_str}" | sed 's/ (deleted)$//')
echo "Waiting for PID {pid} to exit..."
while kill -0 {pid} 2>/dev/null; do sleep 0.2; done
echo "PID {pid} exited, launching $EXE_PATH"
sleep 0.5
exec "$EXE_PATH"
"#,
        pid = pid,
        exe_str = exe_str,
    );
    let script_path = format!("/tmp/lcd-restart-{}.sh", pid);
    std::fs::write(&script_path, &script)
        .map_err(|e| format!("Failed to write restart script: {}", e))?;

    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o700))
        .map_err(|e| format!("Failed to set script permissions: {}", e))?;

    // Launch the script in a new session so it survives parent exit
    // Pass display env vars via the process environment (not inlined in script) to avoid shell injection
    std::process::Command::new("setsid")
        .arg(&script_path)
        .env("DISPLAY", std::env::var("DISPLAY").unwrap_or_default())
        .env("WAYLAND_DISPLAY", std::env::var("WAYLAND_DISPLAY").unwrap_or_default())
        .env("XDG_RUNTIME_DIR", std::env::var("XDG_RUNTIME_DIR").unwrap_or_default())
        .env("DBUS_SESSION_BUS_ADDRESS", std::env::var("DBUS_SESSION_BUS_ADDRESS").unwrap_or_default())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to spawn restart script: {}", e))?;

    // Hard exit the current process
    std::process::exit(0);
}

/// Get app info for the About section
#[tauri::command]
pub fn get_app_info() -> Result<AppInfo, String> {
    let distro = std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|contents| {
            contents.lines()
                .find(|l| l.starts_with("PRETTY_NAME="))
                .map(|l| l.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_string())
        })
        .unwrap_or_else(|| "Linux".to_string());

    Ok(AppInfo {
        version: CURRENT_VERSION.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        os: distro,
    })
}

#[derive(Debug, Serialize)]
pub struct AppInfo {
    pub version: String,
    pub arch: String,
    pub os: String,
}

/// Capture a screenshot region and return its base64 data
#[tauri::command]
pub async fn capture_screenshot() -> Result<ScreenshotResult, String> {
    let tmp_path = format!("/tmp/ucd-screenshot-{}.png", uuid::Uuid::new_v4());
    let tmp_path = tmp_path.as_str();

    // Try gnome-screenshot first (most common on Ubuntu), then flameshot, then import (ImageMagick)
    let tools: Vec<(&str, Vec<&str>)> = vec![
        ("gnome-screenshot", vec!["-a", "-f", tmp_path]),
        ("flameshot", vec!["gui", "--raw", "-p", tmp_path]),
        ("import", vec![tmp_path]),
    ];

    let mut captured = false;
    for (cmd, args) in &tools {
        if let Ok(status) = std::process::Command::new(cmd)
            .args(args)
            .status()
        {
            if status.success() && std::path::Path::new(tmp_path).exists() {
                captured = true;
                break;
            }
        }
    }

    if !captured {
        return Err("No screenshot tool available. Install gnome-screenshot, flameshot, or imagemagick.".to_string());
    }

    let data = encode_file_to_base64(tmp_path)?;
    let _ = std::fs::remove_file(tmp_path);

    Ok(ScreenshotResult {
        data,
        media_type: "image/png".to_string(),
    })
}

#[derive(Debug, Serialize)]
pub struct ScreenshotResult {
    pub data: String,
    pub media_type: String,
}

/// Open a conversation in a separate window
#[tauri::command]
pub async fn popout_conversation(app: tauri::AppHandle, conversation_id: String) -> Result<(), String> {
    let label = format!("conv-{}", &conversation_id[..8.min(conversation_id.len())]);

    if let Some(win) = app.get_webview_window(&label) {
        win.show().map_err(|e| e.to_string())?;
        win.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    let url = format!("index.html?conversation={}", conversation_id);
    let win = tauri::WebviewWindowBuilder::new(
        &app,
        &label,
        tauri::WebviewUrl::App(url.into()),
    )
    .title("Chat")
    .inner_size(800.0, 600.0)
    .resizable(true)
    .decorations(true)
    .center()
    .build()
    .map_err(|e| e.to_string())?;
    win.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn toggle_quickask(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("quickask") {
        if win.is_visible().unwrap_or(false) {
            win.hide().map_err(|e| e.to_string())?;
        } else {
            win.show().map_err(|e| e.to_string())?;
            win.set_focus().map_err(|e| e.to_string())?;
        }
    } else {
        let win = tauri::WebviewWindowBuilder::new(
            &app,
            "quickask",
            tauri::WebviewUrl::App("index.html?quickask".into()),
        )
        .title("Quick Ask")
        .inner_size(600.0, 400.0)
        .resizable(true)
        .always_on_top(true)
        .decorations(true)
        .center()
        .build()
        .map_err(|e| e.to_string())?;
        win.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[derive(Debug, Serialize, Clone)]
struct ComparisonStreamEvent {
    event: String,
    content: String,
    session_id: String,
    response_id: String,
    model: String,
}

#[derive(Debug, Serialize, Clone)]
struct ComparisonUsageEvent {
    session_id: String,
    response_id: String,
    input_tokens: i64,
    output_tokens: i64,
    model: String,
    latency_ms: i64,
    estimated_cost: f64,
}

#[derive(Debug, Deserialize)]
pub struct ComparisonTarget {
    provider: String,   // "anthropic", "openai", "ollama", "custom"
    model: String,
    endpoint_id: Option<String>,
}

/// Resolve a specific provider from a ComparisonTarget (not from settings)
fn resolve_comparison_provider(state: &AppState, target: &ComparisonTarget) -> Result<ResolvedProvider, String> {
    let db = state.db.lock().unwrap();
    let provider_type = ProviderType::from_str(&target.provider);

    match provider_type {
        ProviderType::Anthropic => {
            let api_key = db.get_setting("api_key").map_err(|e| e.to_string())?
                .ok_or_else(|| "Anthropic API key not set".to_string())?;
            Ok(ResolvedProvider {
                provider_type: ProviderType::Anthropic,
                api_key,
                base_url: "https://api.anthropic.com".to_string(),
                model: target.model.clone(),
                api_format: "anthropic".to_string(),
                endpoint_id: None,
            })
        }
        ProviderType::OpenAI => {
            let api_key = db.get_setting("openai_api_key").map_err(|e| e.to_string())?
                .ok_or_else(|| "OpenAI API key not set".to_string())?;
            let base_url = db.get_setting("openai_base_url").map_err(|e| e.to_string())?
                .unwrap_or_else(|| "https://api.openai.com".to_string());
            Ok(ResolvedProvider {
                provider_type: ProviderType::OpenAI,
                api_key,
                base_url,
                model: target.model.clone(),
                api_format: "openai".to_string(),
                endpoint_id: None,
            })
        }
        ProviderType::Ollama => {
            let base_url = db.get_setting("ollama_base_url").map_err(|e| e.to_string())?
                .unwrap_or_else(|| "http://localhost:11434".to_string());
            Ok(ResolvedProvider {
                provider_type: ProviderType::Ollama,
                api_key: String::new(),
                base_url,
                model: target.model.clone(),
                api_format: "openai".to_string(),
                endpoint_id: None,
            })
        }
        ProviderType::Custom => {
            let endpoint_id = target.endpoint_id.as_ref()
                .ok_or_else(|| "Custom endpoint ID required".to_string())?;
            let endpoint = db.get_custom_endpoint(endpoint_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "Custom endpoint not found".to_string())?;
            Ok(ResolvedProvider {
                provider_type: ProviderType::Custom,
                api_key: endpoint.api_key,
                base_url: endpoint.base_url,
                model: if target.model.is_empty() { endpoint.default_model } else { target.model.clone() },
                api_format: endpoint.api_format,
                endpoint_id: Some(endpoint.id),
            })
        }
    }
}

#[tauri::command]
pub async fn send_comparison(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    prompt: String,
    targets: Vec<ComparisonTarget>,
    system_prompt: Option<String>,
) -> Result<String, String> {
    let session_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    // Create comparison session in DB
    {
        let db = state.db.lock().unwrap();
        db.create_comparison_session(&session_id, &prompt, &now)
            .map_err(|e| e.to_string())?;
    }

    // Resolve all providers before spawning tasks
    let mut resolved: Vec<(String, ResolvedProvider)> = Vec::new();
    for target in &targets {
        let response_id = uuid::Uuid::new_v4().to_string();
        let provider = resolve_comparison_provider(&state, target)?;
        // Create comparison response row
        {
            let db = state.db.lock().unwrap();
            db.create_comparison_response(&response_id, &session_id, &target.provider, &provider.model, &now)
                .map_err(|e| e.to_string())?;
        }
        resolved.push((response_id, provider));
    }

    // Spawn parallel streaming tasks
    for (response_id, provider) in resolved {
        let app_clone = app.clone();
        let session_id = session_id.clone();
        let prompt = prompt.clone();
        let system_prompt = system_prompt.clone();

        tauri::async_runtime::spawn(async move {
            let start = std::time::Instant::now();
            let messages = vec![ApiMessage {
                role: "user".into(),
                content: serde_json::json!(prompt),
            }];

            let client = reqwest::Client::new();
            let mut full_content = String::new();
            let mut total_input_tokens: i64 = 0;
            let mut total_output_tokens: i64 = 0;

            let result = if provider.api_format == "anthropic" {
                stream_comparison_anthropic(
                    &client, &app_clone, &session_id, &response_id, &provider,
                    &messages, &system_prompt, &mut full_content,
                    &mut total_input_tokens, &mut total_output_tokens,
                ).await
            } else {
                stream_comparison_openai(
                    &client, &app_clone, &session_id, &response_id, &provider,
                    &messages, &system_prompt, &mut full_content,
                    &mut total_input_tokens, &mut total_output_tokens,
                ).await
            };

            let latency_ms = start.elapsed().as_millis() as i64;

            // Calculate cost
            let estimated_cost = {
                let db_state = app_clone.state::<AppState>();
                let db = db_state.db.lock().unwrap();
                db.estimate_cost(&provider.model, total_input_tokens, total_output_tokens).unwrap_or(0.0)
            };

            // Update comparison response in DB
            {
                let db_state = app_clone.state::<AppState>();
                let db = db_state.db.lock().unwrap();
                let _ = db.update_comparison_response(
                    &response_id, &full_content, total_input_tokens,
                    total_output_tokens, latency_ms, estimated_cost,
                );
            }

            let _ = app_clone.emit("comparison-usage", ComparisonUsageEvent {
                session_id: session_id.clone(),
                response_id: response_id.clone(),
                input_tokens: total_input_tokens,
                output_tokens: total_output_tokens,
                model: provider.model.clone(),
                latency_ms,
                estimated_cost,
            });

            let _ = app_clone.emit("comparison-stream", ComparisonStreamEvent {
                event: if result.is_ok() { "done" } else { "error" }.into(),
                content: result.err().unwrap_or_default(),
                session_id,
                response_id,
                model: provider.model,
            });
        });
    }

    Ok(session_id)
}

async fn stream_comparison_anthropic(
    client: &reqwest::Client,
    app: &tauri::AppHandle,
    session_id: &str,
    response_id: &str,
    provider: &ResolvedProvider,
    messages: &[ApiMessage],
    system_prompt: &Option<String>,
    full_content: &mut String,
    total_input_tokens: &mut i64,
    total_output_tokens: &mut i64,
) -> Result<(), String> {
    let mut body = serde_json::json!({
        "model": provider.model,
        "max_tokens": 8192,
        "stream": true,
        "messages": messages,
    });
    if let Some(sp) = system_prompt {
        if !sp.trim().is_empty() {
            body["system"] = serde_json::json!(sp);
        }
    }

    let resp = client
        .post(format!("{}/v1/messages", provider.base_url.trim_end_matches('/')))
        .header("x-api-key", &provider.api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .body(body.to_string())
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let err = format!("API error {}", resp.status());
        return Err(err);
    }

    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|e| e.to_string())?;
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
                        "message_start" => {
                            if let Some(usage) = parsed.get("message").and_then(|m| m.get("usage")) {
                                if let Some(it) = usage.get("input_tokens").and_then(|v| v.as_i64()) {
                                    *total_input_tokens += it;
                                }
                            }
                        }
                        "content_block_delta" => {
                            if let Some(delta) = parsed.get("delta") {
                                if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                                    full_content.push_str(text);
                                    let _ = app.emit("comparison-stream", ComparisonStreamEvent {
                                        event: "delta".into(),
                                        content: text.to_string(),
                                        session_id: session_id.to_string(),
                                        response_id: response_id.to_string(),
                                        model: provider.model.clone(),
                                    });
                                }
                            }
                        }
                        "message_delta" => {
                            if let Some(usage) = parsed.get("usage") {
                                if let Some(ot) = usage.get("output_tokens").and_then(|v| v.as_i64()) {
                                    *total_output_tokens = ot;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(())
}

async fn stream_comparison_openai(
    client: &reqwest::Client,
    app: &tauri::AppHandle,
    session_id: &str,
    response_id: &str,
    provider: &ResolvedProvider,
    messages: &[ApiMessage],
    system_prompt: &Option<String>,
    full_content: &mut String,
    total_input_tokens: &mut i64,
    total_output_tokens: &mut i64,
) -> Result<(), String> {
    let mut oai_messages: Vec<serde_json::Value> = Vec::new();
    if let Some(sp) = system_prompt {
        if !sp.trim().is_empty() {
            oai_messages.push(serde_json::json!({"role": "system", "content": sp}));
        }
    }
    for msg in messages {
        let content_str = if let Some(s) = msg.content.as_str() { s.to_string() } else { msg.content.to_string() };
        oai_messages.push(serde_json::json!({"role": msg.role, "content": content_str}));
    }

    let body = serde_json::json!({
        "model": provider.model,
        "stream": true,
        "stream_options": {"include_usage": true},
        "messages": oai_messages,
    });

    let url = format!("{}/v1/chat/completions", provider.base_url.trim_end_matches('/'));
    let mut req = client.post(&url).header("content-type", "application/json");
    if !provider.api_key.is_empty() {
        req = req.header("authorization", format!("Bearer {}", provider.api_key));
    }

    let resp = req.body(body.to_string()).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("API error {}", resp.status()));
    }

    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|e| e.to_string())?;
        buffer.push_str(&String::from_utf8_lossy(&bytes));

        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].trim().to_string();
            buffer = buffer[pos + 1..].to_string();

            if line.starts_with("data: ") {
                let data = &line[6..];
                if data == "[DONE]" { continue; }
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(choices) = parsed.get("choices").and_then(|c| c.as_array()) {
                        if let Some(first) = choices.first() {
                            if let Some(delta) = first.get("delta") {
                                if let Some(text) = delta.get("content").and_then(|c| c.as_str()) {
                                    full_content.push_str(text);
                                    let _ = app.emit("comparison-stream", ComparisonStreamEvent {
                                        event: "delta".into(),
                                        content: text.to_string(),
                                        session_id: session_id.to_string(),
                                        response_id: response_id.to_string(),
                                        model: provider.model.clone(),
                                    });
                                }
                            }
                        }
                    }
                    if let Some(usage) = parsed.get("usage") {
                        if let Some(pt) = usage.get("prompt_tokens").and_then(|v| v.as_i64()) {
                            *total_input_tokens = pt;
                        }
                        if let Some(ct) = usage.get("completion_tokens").and_then(|v| v.as_i64()) {
                            *total_output_tokens = ct;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn import_url(url: String) -> Result<(String, String), String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let resp = client.get(&url)
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) Linux-Claude-Desktop/1.0")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch URL: {}", e))?;

    let html = resp.text().await.map_err(|e| format!("Failed to read response: {}", e))?;

    // Extract title
    let title = extract_html_title(&html).unwrap_or_else(|| url.clone());

    // Strip HTML tags to get text content
    let text = strip_html_tags(&html);

    // Limit to 50K chars
    let text = if text.len() > 50_000 {
        text[..50_000].to_string()
    } else {
        text
    };

    Ok((title, text))
}

fn extract_html_title(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let start = lower.find("<title")?;
    let after_tag = lower[start..].find('>')?;
    let content_start = start + after_tag + 1;
    let end = lower[content_start..].find("</title>")?;
    let title = &html[content_start..content_start + end];
    let title = title.trim();
    if title.is_empty() { None } else { Some(title.to_string()) }
}

fn strip_html_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;
    let lower = html.to_lowercase();
    let chars: Vec<char> = html.chars().collect();
    let lower_chars: Vec<char> = lower.chars().collect();

    let mut i = 0;
    while i < chars.len() {
        if !in_tag && chars[i] == '<' {
            // Check for script/style start
            let remaining: String = lower_chars[i..].iter().take(10).collect();
            if remaining.starts_with("<script") {
                in_script = true;
            } else if remaining.starts_with("<style") {
                in_style = true;
            }
            // Check for script/style end
            if remaining.starts_with("</script") {
                in_script = false;
            } else if remaining.starts_with("</style") {
                in_style = false;
            }
            in_tag = true;
        } else if in_tag && chars[i] == '>' {
            in_tag = false;
        } else if !in_tag && !in_script && !in_style {
            result.push(chars[i]);
        }
        i += 1;
    }

    // Clean up whitespace: collapse multiple newlines/spaces
    let mut cleaned = String::new();
    let mut prev_was_whitespace = false;
    for ch in result.chars() {
        if ch.is_whitespace() {
            if !prev_was_whitespace {
                cleaned.push(if ch == '\n' { '\n' } else { ' ' });
            }
            prev_was_whitespace = true;
        } else {
            cleaned.push(ch);
            prev_was_whitespace = false;
        }
    }

    cleaned.trim().to_string()
}

// ── Voice: TTS / STT ─────────────────────────────────────────────────────────

fn strip_markdown_for_tts(text: &str) -> String {
    let mut result = String::new();
    let mut in_code_block = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block { continue; }
        // Remove heading markers, blockquotes, horizontal rules
        let line = trimmed
            .trim_start_matches('#')
            .trim_start_matches('>')
            .trim();
        if line == "---" || line == "***" || line == "___" { continue; }
        // Remove bold/italic/code markers
        let line = line
            .replace("**", "")
            .replace("__", "")
            .replace('`', "");
        // Collapse * used for italic (single)
        let line: String = line.chars().filter(|&c| c != '*').collect();
        // Strip markdown links [text](url) → text
        let mut cleaned = String::new();
        let mut chars = line.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '[' {
                // Collect link text until ]
                let mut link_text = String::new();
                for ch2 in chars.by_ref() {
                    if ch2 == ']' { break; }
                    link_text.push(ch2);
                }
                // Skip (url)
                if chars.peek() == Some(&'(') {
                    chars.next();
                    for ch2 in chars.by_ref() {
                        if ch2 == ')' { break; }
                    }
                }
                cleaned.push_str(&link_text);
            } else {
                cleaned.push(ch);
            }
        }
        if !cleaned.trim().is_empty() {
            result.push_str(cleaned.trim());
            result.push(' ');
        }
    }
    result.trim().to_string()
}

fn command_exists(name: &str) -> bool {
    std::process::Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[tauri::command]
pub fn check_tts_available() -> bool {
    command_exists("spd-say") || command_exists("espeak-ng")
}

#[tauri::command]
pub fn check_stt_available() -> bool {
    command_exists("arecord") && (command_exists("whisper-cli") || command_exists("whisper"))
}

/// rate: 0–200 (default 100). Maps to spd-say -100..100 or espeak-ng 80..280 wpm.
#[tauri::command]
pub fn speak_text(text: String, rate: i32) -> Result<(), String> {
    let _ = std::process::Command::new("pkill").args(["-f", "spd-say"]).output();
    let _ = std::process::Command::new("pkill").args(["-f", "espeak-ng"]).output();

    let cleaned = strip_markdown_for_tts(&text);
    if cleaned.trim().is_empty() { return Ok(()); }

    // Try spd-say (-w = wait, -r = rate -100..100)
    let spd_rate = (rate - 100).to_string();
    if command_exists("spd-say") {
        let _ = std::process::Command::new("spd-say")
            .args(["-r", &spd_rate, &cleaned])
            .spawn();
        return Ok(());
    }

    // Fallback: espeak-ng (-s = speed in wpm)
    let espeak_rate = (80 + rate).to_string();
    if command_exists("espeak-ng") {
        let _ = std::process::Command::new("espeak-ng")
            .args(["-s", &espeak_rate, &cleaned])
            .spawn();
        return Ok(());
    }

    Err("TTS unavailable: install spd-say or espeak-ng".to_string())
}

#[tauri::command]
pub fn stop_speech() -> Result<(), String> {
    let _ = std::process::Command::new("pkill").args(["-f", "spd-say"]).output();
    let _ = std::process::Command::new("pkill").args(["-f", "espeak-ng"]).output();
    Ok(())
}

#[tauri::command]
pub fn start_recording() -> Result<(), String> {
    let mut guard = RECORDING_CHILD.lock().unwrap();
    if let Some(mut child) = guard.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
    let child = std::process::Command::new("arecord")
        .args(["-f", "S16_LE", "-r", "16000", "-c", "1", "/tmp/lcd-recording.wav"])
        .spawn()
        .map_err(|e| format!("Failed to start recording (install arecord): {e}"))?;
    *guard = Some(child);
    Ok(())
}

#[tauri::command]
pub fn stop_recording_and_transcribe(model_path: String) -> Result<String, String> {
    {
        let mut guard = RECORDING_CHILD.lock().unwrap();
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
    std::thread::sleep(std::time::Duration::from_millis(300));

    let whisper_bin = if command_exists("whisper-cli") { "whisper-cli" } else { "whisper" };

    let output = std::process::Command::new(whisper_bin)
        .args(["-m", &model_path, "-f", "/tmp/lcd-recording.wav", "--output-txt", "--no-prints"])
        .output()
        .map_err(|e| format!("Transcription failed (install whisper.cpp): {e}"))?;

    let _ = std::fs::remove_file("/tmp/lcd-recording.wav");

    // whisper writes a .txt sidecar file
    let txt_path = "/tmp/lcd-recording.wav.txt";
    if let Ok(text) = std::fs::read_to_string(txt_path) {
        let _ = std::fs::remove_file(txt_path);
        return Ok(text.trim().to_string());
    }

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
