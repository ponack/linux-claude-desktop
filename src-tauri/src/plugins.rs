use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ── Manifest schema ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_main")]
    pub main: String,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub hooks: Vec<String>,
}

fn default_main() -> String {
    "index.js".to_string()
}

// ── Plugin record (manifest + on-disk state) ──────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PluginInfo {
    pub manifest: PluginManifest,
    pub path: String,
    pub main_path: String,
    pub enabled: bool,
}

// ── Directory helpers ─────────────────────────────────────────────────────────

fn plugins_dir() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("linux-claude-desktop").join("plugins");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn read_manifest(plugin_path: &std::path::Path) -> Result<PluginManifest, String> {
    let manifest_path = plugin_path.join("manifest.json");
    let content = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Cannot read manifest.json: {e}"))?;
    serde_json::from_str::<PluginManifest>(&content)
        .map_err(|e| format!("Invalid manifest.json: {e}"))
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_plugins_dir() -> String {
    plugins_dir().to_string_lossy().to_string()
}

#[derive(Serialize)]
pub struct PluginScanResult {
    pub plugins: Vec<PluginInfo>,
    pub errors: Vec<PluginScanError>,
}

#[derive(Serialize)]
pub struct PluginScanError {
    pub path: String,
    pub error: String,
}

#[tauri::command]
pub fn list_plugins(
    state: tauri::State<crate::AppState>,
) -> Result<PluginScanResult, String> {
    let dir = plugins_dir();
    let mut plugins = Vec::new();
    let mut errors = Vec::new();

    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return Ok(PluginScanResult { plugins, errors }),
    };

    let disabled = state
        .db
        .lock()
        .unwrap()
        .get_disabled_plugins()
        .unwrap_or_default();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        match read_manifest(&path) {
            Ok(manifest) => {
                let main_path = path.join(&manifest.main).to_string_lossy().to_string();
                let enabled = !disabled.contains(&manifest.id);
                plugins.push(PluginInfo {
                    manifest,
                    path: path.to_string_lossy().to_string(),
                    main_path,
                    enabled,
                });
            }
            Err(e) => {
                errors.push(PluginScanError {
                    path: path.to_string_lossy().to_string(),
                    error: e,
                });
            }
        }
    }

    plugins.sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));
    Ok(PluginScanResult { plugins, errors })
}

#[tauri::command]
pub fn read_plugin_source(main_path: String) -> Result<String, String> {
    // Constrain reads to the plugins directory to prevent arbitrary FS reads.
    let dir = plugins_dir();
    let canonical_dir = dir.canonicalize().map_err(|e| e.to_string())?;
    let canonical_file = std::path::Path::new(&main_path)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if !canonical_file.starts_with(&canonical_dir) {
        return Err("Plugin source path is outside the plugins directory".to_string());
    }
    std::fs::read_to_string(&canonical_file).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_plugin_enabled(
    state: tauri::State<crate::AppState>,
    id: String,
    enabled: bool,
) -> Result<(), String> {
    state
        .db
        .lock()
        .unwrap()
        .set_plugin_enabled(&id, enabled)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_plugins_folder(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_shell::ShellExt;
    let dir = get_plugins_dir();
    app.shell()
        .open(dir, None)
        .map_err(|e| e.to_string())
}

// ── Install / Uninstall ───────────────────────────────────────────────────────

const MAX_PLUGIN_ZIP_BYTES: u64 = 20 * 1024 * 1024; // 20 MB cap

#[derive(Serialize)]
pub struct InstallResult {
    pub id: String,
    pub name: String,
    pub version: String,
}

/// Download a plugin .zip from a URL, validate its manifest, and extract it
/// into the plugins directory under the plugin's `id`.
///
/// Validation:
///   - URL must be http(s)
///   - Body capped at MAX_PLUGIN_ZIP_BYTES
///   - Zip must contain a manifest.json at the root (or inside a single
///     top-level folder), parseable as a PluginManifest
///   - All extracted paths must stay within the target plugin directory
///     (zip-slip mitigation)
#[tauri::command]
pub async fn install_plugin_from_url(url: String) -> Result<InstallResult, String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("URL must be http(s)".to_string());
    }

    let resp = reqwest::Client::new()
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Download failed: HTTP {}", resp.status().as_u16()));
    }

    if let Some(len) = resp.content_length() {
        if len > MAX_PLUGIN_ZIP_BYTES {
            return Err(format!(
                "Plugin zip too large ({} bytes, max {})",
                len, MAX_PLUGIN_ZIP_BYTES
            ));
        }
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Read response failed: {e}"))?;

    if bytes.len() as u64 > MAX_PLUGIN_ZIP_BYTES {
        return Err(format!(
            "Plugin zip too large ({} bytes, max {})",
            bytes.len(),
            MAX_PLUGIN_ZIP_BYTES
        ));
    }

    install_plugin_from_bytes(bytes.to_vec())
}

fn install_plugin_from_bytes(bytes: Vec<u8>) -> Result<InstallResult, String> {
    use std::io::{Cursor, Read};

    let reader = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| format!("Not a valid zip: {e}"))?;

    // Detect single top-level directory (common for GitHub release zips).
    // If every entry shares a single root segment, we strip it on extract.
    let strip_root: Option<String> = {
        let mut root: Option<String> = None;
        let mut all_share_root = true;
        for i in 0..archive.len() {
            let entry = archive
                .by_index(i)
                .map_err(|e| format!("Zip entry read failed: {e}"))?;
            let name = entry.name();
            let first = name.split('/').next().unwrap_or("");
            if first.is_empty() {
                continue;
            }
            match &root {
                None => root = Some(first.to_string()),
                Some(r) if r == first => {}
                _ => {
                    all_share_root = false;
                    break;
                }
            }
        }
        if all_share_root { root } else { None }
    };

    // Find manifest.json and parse it before extracting anything.
    let manifest_path_in_zip = match &strip_root {
        Some(r) => format!("{r}/manifest.json"),
        None => "manifest.json".to_string(),
    };

    let manifest: PluginManifest = {
        let mut entry = archive
            .by_name(&manifest_path_in_zip)
            .map_err(|_| format!("Zip is missing {manifest_path_in_zip}"))?;
        let mut buf = String::new();
        entry
            .read_to_string(&mut buf)
            .map_err(|e| format!("Read manifest failed: {e}"))?;
        serde_json::from_str(&buf).map_err(|e| format!("Invalid manifest.json: {e}"))?
    };

    if manifest.id.is_empty() || !is_safe_id(&manifest.id) {
        return Err(format!("Invalid plugin id: {:?}", manifest.id));
    }

    let plugins_root = plugins_dir();
    let target_dir = plugins_root.join(&manifest.id);
    let canonical_root = plugins_root
        .canonicalize()
        .map_err(|e| format!("Cannot canonicalize plugins dir: {e}"))?;

    // If a previous version exists, remove it first (clean reinstall).
    if target_dir.exists() {
        std::fs::remove_dir_all(&target_dir)
            .map_err(|e| format!("Remove existing plugin failed: {e}"))?;
    }
    std::fs::create_dir_all(&target_dir)
        .map_err(|e| format!("Create plugin dir failed: {e}"))?;

    // Extract all files, stripping the single root prefix if any.
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("Zip entry {i} read failed: {e}"))?;
        let raw_name = entry.name().to_string();

        let rel = match &strip_root {
            Some(r) => {
                let prefix = format!("{r}/");
                if raw_name == *r || raw_name == prefix {
                    continue;
                }
                if let Some(stripped) = raw_name.strip_prefix(&prefix) {
                    stripped.to_string()
                } else {
                    continue;
                }
            }
            None => raw_name.clone(),
        };

        if rel.is_empty() {
            continue;
        }
        // Reject path traversal explicitly. Zip-slip mitigation.
        if rel.contains("..") || rel.starts_with('/') {
            return Err(format!("Unsafe entry in zip: {raw_name}"));
        }

        let out_path = target_dir.join(&rel);

        // Belt-and-braces: ensure the resolved path is still inside the plugin dir.
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Create dir {parent:?} failed: {e}"))?;
        }
        // Canonicalize the parent (which exists now) and check that the full
        // path resolves inside canonical_root.
        let parent_canon = out_path
            .parent()
            .and_then(|p| p.canonicalize().ok())
            .unwrap_or_else(|| target_dir.clone());
        if !parent_canon.starts_with(&canonical_root) {
            return Err(format!("Path escape: {raw_name}"));
        }

        if entry.is_dir() {
            std::fs::create_dir_all(&out_path)
                .map_err(|e| format!("mkdir {out_path:?} failed: {e}"))?;
        } else {
            let mut out = std::fs::File::create(&out_path)
                .map_err(|e| format!("Write {out_path:?} failed: {e}"))?;
            std::io::copy(&mut entry, &mut out)
                .map_err(|e| format!("Copy {out_path:?} failed: {e}"))?;
        }
    }

    Ok(InstallResult {
        id: manifest.id,
        name: manifest.name,
        version: manifest.version,
    })
}

fn is_safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        && !id.starts_with('.')
        && !id.contains("..")
        && id != "/"
}

#[tauri::command]
pub fn uninstall_plugin(
    state: tauri::State<crate::AppState>,
    id: String,
) -> Result<(), String> {
    if !is_safe_id(&id) {
        return Err(format!("Invalid plugin id: {id:?}"));
    }
    let plugins_root = plugins_dir();
    let target_dir = plugins_root.join(&id);

    // Refuse to act unless the target resolves inside the plugins dir.
    let canonical_root = plugins_root
        .canonicalize()
        .map_err(|e| format!("Cannot canonicalize plugins dir: {e}"))?;
    let canonical_target = match target_dir.canonicalize() {
        Ok(p) => p,
        Err(_) => return Err(format!("Plugin not found: {id}")),
    };
    if !canonical_target.starts_with(&canonical_root) {
        return Err("Refusing to delete: target outside plugins directory".to_string());
    }

    std::fs::remove_dir_all(&canonical_target)
        .map_err(|e| format!("Remove failed: {e}"))?;

    // Clean up enabled/disabled state row. Storage rows are kept on purpose:
    // they're harmless and let a user reinstall and recover their data.
    let _ = state.db.lock().unwrap().delete_plugin_state(&id);
    Ok(())
}
