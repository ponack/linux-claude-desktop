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
