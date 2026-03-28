use crate::AppState;
use crate::api::encode_file_to_base64;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Emitter;

static CU_STOP_FLAG: AtomicBool = AtomicBool::new(false);

const MAX_ITERATIONS: u32 = 30;
const COMPUTER_USE_TOOL_TYPE: &str = "computer_20250124";
const COMPUTER_USE_BETA_HEADER: &str = "computer-use-2025-01-24";

// ── Events ────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Clone)]
pub struct ComputerUseEvent {
    pub event_type: String,         // "action" | "screenshot" | "text" | "done" | "error"
    pub content: String,
    pub screenshot: Option<String>, // base64 PNG
    pub iteration: u32,
}

// ── Tool input (what Claude sends back) ──────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum ToolInput {
    Screenshot,
    MouseMove { coordinate: [i32; 2] },
    LeftClick { coordinate: [i32; 2] },
    RightClick { coordinate: [i32; 2] },
    DoubleClick { coordinate: [i32; 2] },
    LeftClickDrag { start_coordinate: [i32; 2], coordinate: [i32; 2] },
    Key { text: String },
    #[serde(rename = "type")]
    TypeText { text: String },
    Scroll { coordinate: [i32; 2], direction: String, amount: Option<i32> },
    CursorPosition,
    Wait,
}

// ── Availability check ────────────────────────────────────────────────────────

fn cmd_exists(name: &str) -> bool {
    std::process::Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[derive(Debug, Serialize)]
pub struct CuAvailability {
    pub available: bool,
    pub xdotool: bool,
    pub screenshot: bool,
    pub message: String,
}

#[tauri::command]
pub fn check_computer_use_available() -> CuAvailability {
    let xdotool = cmd_exists("xdotool");
    let screenshot = cmd_exists("scrot") || cmd_exists("import");
    let available = xdotool && screenshot;
    let message = if !xdotool {
        "Install xdotool (sudo apt install xdotool)".into()
    } else if !screenshot {
        "Install scrot or imagemagick (sudo apt install scrot)".into()
    } else {
        "Ready".into()
    };
    CuAvailability { available, xdotool, screenshot, message }
}

// ── Screen geometry ───────────────────────────────────────────────────────────

fn screen_size() -> (u32, u32) {
    if let Ok(out) = std::process::Command::new("xdotool")
        .arg("getdisplaygeometry").output()
    {
        let s = String::from_utf8_lossy(&out.stdout);
        let mut parts = s.trim().split_whitespace();
        if let (Some(w), Some(h)) = (parts.next(), parts.next()) {
            if let (Ok(w), Ok(h)) = (w.parse::<u32>(), h.parse::<u32>()) {
                return (w, h);
            }
        }
    }
    (1920, 1080)
}

// ── Full-screen screenshot ────────────────────────────────────────────────────

fn fullscreen_screenshot() -> Result<String, String> {
    let tmp = format!("/tmp/lcd-cu-{}.png", uuid::Uuid::new_v4());
    let captured = [
        ("scrot",  vec![tmp.as_str()]),
        ("import", vec!["-window", "root", tmp.as_str()]),
        ("gnome-screenshot", vec!["-f", tmp.as_str()]),
    ].iter().any(|(cmd, args)| {
        std::process::Command::new(cmd).args(args).status()
            .map(|s| s.success() && std::path::Path::new(&tmp).exists())
            .unwrap_or(false)
    });
    if !captured {
        return Err("No screenshot tool found. Install scrot: sudo apt install scrot".into());
    }
    let data = encode_file_to_base64(&tmp)?;
    let _ = std::fs::remove_file(&tmp);
    Ok(data)
}

// ── xdotool helpers ───────────────────────────────────────────────────────────

fn xdotool(args: &[&str]) -> Result<(), String> {
    let ok = std::process::Command::new("xdotool").args(args)
        .status().map(|s| s.success()).unwrap_or(false);
    if !ok { return Err(format!("xdotool {:?} failed", args)); }
    Ok(())
}

fn pause(ms: u64) {
    std::thread::sleep(std::time::Duration::from_millis(ms));
}

// ── Execute a single computer action ─────────────────────────────────────────

fn execute_action(action: &ToolInput) -> Result<String, String> {
    match action {
        ToolInput::Screenshot => Ok("Taking screenshot".into()),

        ToolInput::MouseMove { coordinate: [x, y] } => {
            xdotool(&["mousemove", &x.to_string(), &y.to_string()])?;
            Ok(format!("Mouse moved to ({x}, {y})"))
        }

        ToolInput::LeftClick { coordinate: [x, y] } => {
            xdotool(&["mousemove", &x.to_string(), &y.to_string()])?;
            pause(50);
            xdotool(&["click", "1"])?;
            Ok(format!("Left clicked ({x}, {y})"))
        }

        ToolInput::RightClick { coordinate: [x, y] } => {
            xdotool(&["mousemove", &x.to_string(), &y.to_string()])?;
            pause(50);
            xdotool(&["click", "3"])?;
            Ok(format!("Right clicked ({x}, {y})"))
        }

        ToolInput::DoubleClick { coordinate: [x, y] } => {
            xdotool(&["mousemove", &x.to_string(), &y.to_string()])?;
            pause(50);
            xdotool(&["click", "--repeat", "2", "--delay", "100", "1"])?;
            Ok(format!("Double clicked ({x}, {y})"))
        }

        ToolInput::LeftClickDrag { start_coordinate: [sx, sy], coordinate: [ex, ey] } => {
            xdotool(&["mousemove", &sx.to_string(), &sy.to_string()])?;
            xdotool(&["mousedown", "1"])?;
            pause(80);
            xdotool(&["mousemove", &ex.to_string(), &ey.to_string()])?;
            pause(80);
            xdotool(&["mouseup", "1"])?;
            Ok(format!("Dragged ({sx},{sy}) → ({ex},{ey})"))
        }

        ToolInput::Key { text } => {
            xdotool(&["key", "--clearmodifiers", text.as_str()])?;
            Ok(format!("Key: {text}"))
        }

        ToolInput::TypeText { text } => {
            xdotool(&["type", "--clearmodifiers", "--delay", "30", text.as_str()])?;
            let preview = if text.len() > 60 { &text[..60] } else { text.as_str() };
            Ok(format!("Typed: \"{preview}\""))
        }

        ToolInput::Scroll { coordinate: [x, y], direction, amount } => {
            xdotool(&["mousemove", &x.to_string(), &y.to_string()])?;
            let btn = match direction.as_str() {
                "up" => "4", "down" => "5", "left" => "6", _ => "7"
            };
            let n = amount.unwrap_or(3).to_string();
            xdotool(&["click", "--repeat", n.as_str(), btn])?;
            Ok(format!("Scrolled {direction} ×{} at ({x},{y})", amount.unwrap_or(3)))
        }

        ToolInput::CursorPosition => {
            let out = std::process::Command::new("xdotool")
                .arg("getmouselocation").output().map_err(|e| e.to_string())?;
            Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
        }

        ToolInput::Wait => {
            pause(1000);
            Ok("Waited 1s".into())
        }
    }
}

// ── Main agentic loop ─────────────────────────────────────────────────────────

#[tauri::command]
pub async fn run_computer_use(
    app: tauri::AppHandle,
    task: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    CU_STOP_FLAG.store(false, Ordering::SeqCst);

    let api_key = {
        let db = state.db.lock().unwrap();
        db.get_setting("api_key").map_err(|e| e.to_string())?
            .ok_or_else(|| "No Anthropic API key configured".to_string())?
    };
    let cu_model = {
        let db = state.db.lock().unwrap();
        db.get_setting("cu_model").map_err(|e| e.to_string())?
            .unwrap_or_else(|| "claude-opus-4-6".to_string())
    };

    let (width, height) = screen_size();

    let emit = |event_type: &str, content: &str, screenshot: Option<String>, iteration: u32| {
        let _ = app.emit("computer-use-event", ComputerUseEvent {
            event_type: event_type.to_string(),
            content: content.to_string(),
            screenshot,
            iteration,
        });
    };

    emit("text", &format!("Starting task on a {width}×{height} display…"), None, 0);

    let mut messages: Vec<serde_json::Value> = vec![
        serde_json::json!({ "role": "user", "content": task })
    ];

    let client = reqwest::Client::new();

    for iteration in 1..=MAX_ITERATIONS {
        if CU_STOP_FLAG.load(Ordering::SeqCst) {
            emit("done", "Stopped by user.", None, iteration);
            return Ok(());
        }

        let body = serde_json::json!({
            "model": cu_model,
            "max_tokens": 4096,
            "tools": [{
                "type": COMPUTER_USE_TOOL_TYPE,
                "name": "computer",
                "display_width_px": width,
                "display_height_px": height,
                "display_number": 1,
            }],
            "messages": messages,
        });

        let resp = client.post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("anthropic-beta", COMPUTER_USE_BETA_HEADER)
            .json(&body)
            .send().await
            .map_err(|e| e.to_string())?;

        let http_status = resp.status();
        let resp_json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

        if !http_status.is_success() {
            let msg = resp_json["error"]["message"].as_str()
                .unwrap_or("API error").to_string();
            emit("error", &msg, None, iteration);
            return Err(msg);
        }

        let stop_reason = resp_json["stop_reason"].as_str().unwrap_or("end_turn");
        let blocks = resp_json["content"].as_array().cloned().unwrap_or_default();

        // Keep assistant turn in history
        messages.push(serde_json::json!({ "role": "assistant", "content": blocks }));

        // Emit text to UI
        for block in &blocks {
            if block["type"] == "text" {
                if let Some(t) = block["text"].as_str() {
                    emit("text", t, None, iteration);
                }
            }
        }

        if stop_reason == "end_turn" {
            emit("done", "Task complete.", None, iteration);
            return Ok(());
        }

        if stop_reason != "tool_use" {
            emit("done", &format!("Finished ({stop_reason})."), None, iteration);
            return Ok(());
        }

        // Execute tool calls and collect results
        let mut tool_results: Vec<serde_json::Value> = Vec::new();

        for block in &blocks {
            if block["type"] != "tool_use" { continue; }
            let id = block["id"].as_str().unwrap_or("").to_string();

            let tool_input: ToolInput = match serde_json::from_value(block["input"].clone()) {
                Ok(v) => v,
                Err(e) => {
                    emit("error", &format!("Bad tool input: {e}"), None, iteration);
                    continue;
                }
            };

            // Execute non-screenshot actions
            if !matches!(tool_input, ToolInput::Screenshot) {
                let desc = execute_action(&tool_input)
                    .unwrap_or_else(|e| format!("Action error: {e}"));
                emit("action", &desc, None, iteration);
                pause(400); // let the UI settle after an action
            }

            // Always return a screenshot as the tool result
            match fullscreen_screenshot() {
                Ok(data) => {
                    emit("screenshot", &format!("Screenshot (iteration {iteration})"),
                        Some(data.clone()), iteration);
                    tool_results.push(serde_json::json!({
                        "type": "tool_result",
                        "tool_use_id": id,
                        "content": [{
                            "type": "image",
                            "source": {
                                "type": "base64",
                                "media_type": "image/png",
                                "data": data,
                            }
                        }]
                    }));
                }
                Err(e) => {
                    emit("error", &format!("Screenshot failed: {e}"), None, iteration);
                    tool_results.push(serde_json::json!({
                        "type": "tool_result",
                        "tool_use_id": id,
                        "is_error": true,
                        "content": format!("Screenshot failed: {e}"),
                    }));
                }
            }
        }

        if tool_results.is_empty() {
            emit("done", "Task complete.", None, iteration);
            return Ok(());
        }

        messages.push(serde_json::json!({ "role": "user", "content": tool_results }));
    }

    emit("done", "Reached the maximum iteration limit.", None, MAX_ITERATIONS);
    Ok(())
}

#[tauri::command]
pub fn stop_computer_use() {
    CU_STOP_FLAG.store(true, Ordering::SeqCst);
}
