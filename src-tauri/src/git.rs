use serde::Serialize;
use std::process::Command;

fn run_git(
    args: impl IntoIterator<Item = impl AsRef<std::ffi::OsStr>>,
    cwd: &str,
) -> Result<String, String> {
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

// ── Check availability ────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct GitAvailability {
    pub available: bool,
    pub version: String,
}

#[tauri::command]
pub fn check_git_available() -> GitAvailability {
    match run_git(["--version"], ".") {
        Ok(v) => GitAvailability { available: true, version: v.trim().to_string() },
        Err(_) => GitAvailability { available: false, version: String::new() },
    }
}

// ── Repo info ─────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct GitRepoInfo {
    pub branch: String,
    pub remote_url: String,
    pub ahead: i32,
    pub behind: i32,
    pub is_git_repo: bool,
}

#[tauri::command]
pub fn git_repo_info(path: String) -> GitRepoInfo {
    let branch = match run_git(["rev-parse", "--abbrev-ref", "HEAD"], &path) {
        Ok(s) => s.trim().to_string(),
        Err(_) => {
            return GitRepoInfo {
                branch: String::new(),
                remote_url: String::new(),
                ahead: 0,
                behind: 0,
                is_git_repo: false,
            }
        }
    };

    let remote_url = run_git(["remote", "get-url", "origin"], &path)
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    let (ahead, behind) = run_git(
        ["rev-list", "--left-right", "--count", "@{u}...HEAD"],
        &path,
    )
    .map(|s| {
        let parts: Vec<&str> = s.trim().split_whitespace().collect();
        let b = parts.first().and_then(|x| x.parse().ok()).unwrap_or(0);
        let a = parts.get(1).and_then(|x| x.parse().ok()).unwrap_or(0);
        (a, b)
    })
    .unwrap_or((0, 0));

    GitRepoInfo { branch, remote_url, ahead, behind, is_git_repo: true }
}

// ── Status ────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct GitFileStatus {
    pub path: String,
    pub orig_path: Option<String>,
    pub status: String,
    pub staged: bool,
}

#[tauri::command]
pub fn git_status(path: String) -> Result<Vec<GitFileStatus>, String> {
    let output = run_git(["status", "--porcelain=v1", "-u"], &path)?;
    let mut files: Vec<GitFileStatus> = Vec::new();

    for line in output.lines() {
        if line.len() < 3 {
            continue;
        }
        let x = line.chars().next().unwrap_or(' ');
        let y = line.chars().nth(1).unwrap_or(' ');
        let rest = &line[3..];

        let (orig, fpath) = if let Some(idx) = rest.find(" -> ") {
            (Some(rest[..idx].to_string()), rest[idx + 4..].to_string())
        } else {
            (None, rest.to_string())
        };

        if x == '?' && y == '?' {
            files.push(GitFileStatus {
                path: fpath,
                orig_path: None,
                status: "untracked".to_string(),
                staged: false,
            });
            continue;
        }

        // Staged (index)
        if x != ' ' {
            let status = match x {
                'M' => "modified",
                'A' => "added",
                'D' => "deleted",
                'R' => "renamed",
                'C' => "copied",
                _ => "unknown",
            };
            files.push(GitFileStatus {
                path: fpath.clone(),
                orig_path: orig.clone(),
                status: status.to_string(),
                staged: true,
            });
        }

        // Unstaged (worktree)
        if y != ' ' {
            let status = match y {
                'M' => "modified",
                'D' => "deleted",
                _ => "unknown",
            };
            files.push(GitFileStatus {
                path: fpath,
                orig_path: None,
                status: status.to_string(),
                staged: false,
            });
        }
    }

    Ok(files)
}

// ── Log ───────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct GitCommit {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub date: String,
    pub message: String,
}

#[tauri::command]
pub fn git_log(path: String, limit: Option<u32>) -> Result<Vec<GitCommit>, String> {
    let n = limit.unwrap_or(50).to_string();
    let format = "--format=%H\x1f%h\x1f%an\x1f%ai\x1f%s";
    let output = run_git(["log", format, "-n", &n], &path)?;

    let commits = output
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(5, '\x1f').collect();
            if parts.len() < 5 {
                return None;
            }
            Some(GitCommit {
                hash: parts[0].to_string(),
                short_hash: parts[1].to_string(),
                author: parts[2].to_string(),
                date: parts[3][..10].to_string(), // just the date portion
                message: parts[4].to_string(),
            })
        })
        .collect();

    Ok(commits)
}

// ── Diff ──────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn git_diff(path: String, file: Option<String>, staged: bool) -> Result<String, String> {
    let mut args: Vec<String> = vec!["diff".to_string()];
    if staged {
        args.push("--cached".to_string());
    }
    if let Some(f) = file {
        args.push("--".to_string());
        args.push(f);
    }
    run_git(&args, &path)
}

// ── Stage / Unstage ───────────────────────────────────────────────────────────

#[tauri::command]
pub fn git_stage(path: String, files: Vec<String>) -> Result<(), String> {
    if files.is_empty() {
        return run_git(["add", "-A"], &path).map(|_| ());
    }
    let mut args = vec!["add".to_string(), "--".to_string()];
    args.extend(files);
    run_git(&args, &path).map(|_| ())
}

#[tauri::command]
pub fn git_unstage(path: String, files: Vec<String>) -> Result<(), String> {
    if files.is_empty() {
        return run_git(["reset", "HEAD"], &path).map(|_| ());
    }
    let mut args = vec!["restore".to_string(), "--staged".to_string(), "--".to_string()];
    args.extend(files);
    run_git(&args, &path).map(|_| ())
}

// ── Commit ────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct CommitResult {
    pub short_hash: String,
    pub summary: String,
}

#[tauri::command]
pub fn git_commit(path: String, message: String) -> Result<CommitResult, String> {
    let output = run_git(["commit", "-m", &message], &path)?;
    // "[branch abc1234] subject" → extract abc1234
    let short_hash = output
        .lines()
        .find(|l| l.starts_with('['))
        .and_then(|l| l.split_whitespace().nth(1))
        .unwrap_or("")
        .trim_end_matches(']')
        .to_string();
    Ok(CommitResult { short_hash, summary: output.trim().to_string() })
}

// ── Push / Pull ───────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn git_push(path: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || run_git(["push"], &path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn git_pull(path: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || run_git(["pull"], &path))
        .await
        .map_err(|e| e.to_string())?
}
