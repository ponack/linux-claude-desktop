use serde::Serialize;
use std::process::Stdio;
use std::time::Duration;

const RUNTIMES: &[(&str, &str, &str)] = &[
    // (language alias, command, install hint)
    ("python",     "python3", "sudo apt install python3"),
    ("python3",    "python3", "sudo apt install python3"),
    ("py",         "python3", "sudo apt install python3"),
    ("javascript", "node",    "sudo apt install nodejs"),
    ("js",         "node",    "sudo apt install nodejs"),
    ("node",       "node",    "sudo apt install nodejs"),
    ("bash",       "bash",    "pre-installed"),
    ("sh",         "sh",      "pre-installed"),
    ("ruby",       "ruby",    "sudo apt install ruby"),
    ("rb",         "ruby",    "sudo apt install ruby"),
];

fn find_runtime(lang: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let lower = lang.to_lowercase();
    RUNTIMES.iter().find(|(alias, _, _)| *alias == lower.as_str()).copied()
}

fn cmd_exists(name: &str) -> bool {
    std::process::Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// ── Public API ────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct RuntimeInfo {
    pub available: bool,
    pub command: String,
    pub install_hint: String,
}

#[tauri::command]
pub fn check_runtime_available(language: String) -> RuntimeInfo {
    match find_runtime(&language) {
        Some((_, cmd, hint)) => {
            let available = cmd_exists(cmd);
            RuntimeInfo {
                available,
                command: cmd.to_string(),
                install_hint: if available { String::new() } else { hint.to_string() },
            }
        }
        None => RuntimeInfo {
            available: false,
            command: String::new(),
            install_hint: String::new(),
        },
    }
}

#[derive(Serialize)]
pub struct ExecutionResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub timed_out: bool,
}

#[tauri::command]
pub async fn execute_code(
    language: String,
    code: String,
    timeout_secs: Option<u32>,
) -> Result<ExecutionResult, String> {
    let timeout = timeout_secs.unwrap_or(30);

    let (_, cmd, _) = find_runtime(&language)
        .ok_or_else(|| format!("Unsupported language: {language}"))?;

    if !cmd_exists(cmd) {
        return Err(format!(
            "{cmd} not found. Install it first (e.g. sudo apt install {}).",
            cmd
        ));
    }

    let ext = match language.to_lowercase().as_str() {
        "python" | "python3" | "py" => "py",
        "javascript" | "js" | "node" => "js",
        "bash" | "sh" => "sh",
        "ruby" | "rb" => "rb",
        _ => "txt",
    };

    let tmp_dir = format!("/tmp/lcd-exec-{}", uuid::Uuid::new_v4());
    std::fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;
    let code_file = format!("{tmp_dir}/code.{ext}");
    std::fs::write(&code_file, &code).map_err(|e| e.to_string())?;

    let child = std::process::Command::new(cmd)
        .arg(&code_file)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(&tmp_dir)
        .spawn()
        .map_err(|e| e.to_string())?;

    // Store PID before moving child into blocking task (needed to kill on timeout)
    let pid = child.id();

    let wait_future =
        tokio::task::spawn_blocking(move || child.wait_with_output());

    let result =
        tokio::time::timeout(Duration::from_secs(timeout as u64), wait_future).await;

    let _ = std::fs::remove_dir_all(&tmp_dir);

    match result {
        Ok(Ok(Ok(output))) => Ok(ExecutionResult {
            stdout: String::from_utf8_lossy(&output.stdout).trim_end().to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim_end().to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            timed_out: false,
        }),
        Ok(Ok(Err(e))) => Err(e.to_string()),
        _ => {
            // Timeout — kill the process
            let _ = std::process::Command::new("kill")
                .args(["-9", &pid.to_string()])
                .output();
            Ok(ExecutionResult {
                stdout: String::new(),
                stderr: format!("Execution timed out after {timeout}s"),
                exit_code: -1,
                timed_out: true,
            })
        }
    }
}
