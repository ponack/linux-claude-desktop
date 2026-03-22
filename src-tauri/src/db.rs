use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new() -> Result<Self, rusqlite::Error> {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("ubuntu-claude-desktop");
        std::fs::create_dir_all(&data_dir).ok();

        let db_path = data_dir.join("claude-desktop.db");
        let conn = Connection::open(db_path)?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS conversations (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                project_id TEXT
            );
            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                context TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS prompts (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            PRAGMA foreign_keys = ON;"
        )?;

        // Migration: add project_id column if missing (existing DBs)
        let has_project_id: bool = conn
            .prepare("SELECT sql FROM sqlite_master WHERE type='table' AND name='conversations'")
            .and_then(|mut s| s.query_row([], |row| row.get::<_, String>(0)))
            .map(|sql| sql.contains("project_id"))
            .unwrap_or(false);
        if !has_project_id {
            conn.execute_batch("ALTER TABLE conversations ADD COLUMN project_id TEXT;").ok();
        }

        // Migration: create projects table if missing
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                context TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL
            );"
        ).ok();

        Ok(Self { conn })
    }

    pub fn list_conversations(&self) -> Result<Vec<Conversation>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, created_at, updated_at FROM conversations ORDER BY updated_at DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Conversation {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
            })
        })?;
        rows.collect()
    }

    pub fn list_messages(&self, conversation_id: &str) -> Result<Vec<Message>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, conversation_id, role, content, created_at FROM messages WHERE conversation_id = ?1 ORDER BY created_at ASC"
        )?;
        let rows = stmt.query_map(params![conversation_id], |row| {
            Ok(Message {
                id: row.get(0)?,
                conversation_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;
        rows.collect()
    }

    pub fn insert_conversation(&self, id: &str, title: &str) -> Result<(), rusqlite::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO conversations (id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, title, &now, &now],
        )?;
        Ok(())
    }

    pub fn insert_message(&self, id: &str, conversation_id: &str, role: &str, content: &str) -> Result<(), rusqlite::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO messages (id, conversation_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, conversation_id, role, content, &now],
        )?;
        self.conn.execute(
            "UPDATE conversations SET updated_at = ?1 WHERE id = ?2",
            params![&now, conversation_id],
        )?;
        Ok(())
    }

    pub fn delete_conversation_by_id(&self, id: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute("DELETE FROM messages WHERE conversation_id = ?1", params![id])?;
        self.conn.execute("DELETE FROM conversations WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn rename_conversation_by_id(&self, id: &str, title: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE conversations SET title = ?1 WHERE id = ?2",
            params![title, id],
        )?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, rusqlite::Error> {
        let mut stmt = self.conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query_map(params![key], |row| row.get::<_, String>(0))?;
        match rows.next() {
            Some(Ok(val)) => Ok(Some(val)),
            _ => Ok(None),
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn remove_setting(&self, key: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute("DELETE FROM settings WHERE key = ?1", params![key])?;
        Ok(())
    }

    pub fn delete_messages_from(&self, conversation_id: &str, message_id: &str) -> Result<(), rusqlite::Error> {
        // Get the created_at of the target message
        let created_at: String = self.conn.query_row(
            "SELECT created_at FROM messages WHERE id = ?1",
            params![message_id],
            |row| row.get(0),
        )?;
        // Delete this message and all after it
        self.conn.execute(
            "DELETE FROM messages WHERE conversation_id = ?1 AND created_at >= ?2",
            params![conversation_id, &created_at],
        )?;
        Ok(())
    }

    pub fn update_message_content(&self, id: &str, content: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE messages SET content = ?1 WHERE id = ?2",
            params![content, id],
        )?;
        Ok(())
    }

    pub fn list_projects(&self) -> Result<Vec<Project>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, context, created_at FROM projects ORDER BY name ASC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                context: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        rows.collect()
    }

    pub fn insert_project(&self, id: &str, name: &str, context: &str) -> Result<(), rusqlite::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO projects (id, name, context, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, name, context, &now],
        )?;
        Ok(())
    }

    pub fn update_project(&self, id: &str, name: &str, context: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE projects SET name = ?1, context = ?2 WHERE id = ?3",
            params![name, context, id],
        )?;
        Ok(())
    }

    pub fn delete_project_by_id(&self, id: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute("UPDATE conversations SET project_id = NULL WHERE project_id = ?1", params![id])?;
        self.conn.execute("DELETE FROM projects WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_project_context(&self, project_id: &str) -> Result<Option<String>, rusqlite::Error> {
        let mut stmt = self.conn.prepare("SELECT context FROM projects WHERE id = ?1")?;
        let mut rows = stmt.query_map(params![project_id], |row| row.get::<_, String>(0))?;
        match rows.next() {
            Some(Ok(val)) if !val.is_empty() => Ok(Some(val)),
            _ => Ok(None),
        }
    }

    pub fn set_conversation_project(&self, conversation_id: &str, project_id: Option<&str>) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE conversations SET project_id = ?1 WHERE id = ?2",
            params![project_id, conversation_id],
        )?;
        Ok(())
    }

    pub fn get_conversation_project_id(&self, conversation_id: &str) -> Result<Option<String>, rusqlite::Error> {
        let mut stmt = self.conn.prepare("SELECT project_id FROM conversations WHERE id = ?1")?;
        let mut rows = stmt.query_map(params![conversation_id], |row| row.get::<_, Option<String>>(0))?;
        match rows.next() {
            Some(Ok(val)) => Ok(val),
            _ => Ok(None),
        }
    }

    pub fn list_prompts(&self) -> Result<Vec<Prompt>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, content, created_at FROM prompts ORDER BY name ASC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Prompt {
                id: row.get(0)?,
                name: row.get(1)?,
                content: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        rows.collect()
    }

    pub fn insert_prompt(&self, id: &str, name: &str, content: &str) -> Result<(), rusqlite::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO prompts (id, name, content, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, name, content, &now],
        )?;
        Ok(())
    }

    pub fn update_prompt_by_id(&self, id: &str, name: &str, content: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE prompts SET name = ?1, content = ?2 WHERE id = ?3",
            params![name, content, id],
        )?;
        Ok(())
    }

    pub fn delete_prompt_by_id(&self, id: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute("DELETE FROM prompts WHERE id = ?1", params![id])?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub context: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Prompt {
    pub id: String,
    pub name: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ExportedConversation {
    pub title: String,
    pub created_at: String,
    pub messages: Vec<ExportedMessage>,
}

#[derive(Debug, Serialize)]
pub struct ExportedMessage {
    pub role: String,
    pub content: String,
    pub created_at: String,
}

// --- Tauri commands ---

use crate::AppState;

#[tauri::command]
pub fn get_conversations(state: tauri::State<AppState>) -> Result<Vec<Conversation>, String> {
    state.db.lock().unwrap().list_conversations().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_messages(state: tauri::State<AppState>, conversation_id: String) -> Result<Vec<Message>, String> {
    state.db.lock().unwrap().list_messages(&conversation_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_conversation(state: tauri::State<AppState>, title: String) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    state.db.lock().unwrap().insert_conversation(&id, &title).map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub fn delete_conversation(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    state.db.lock().unwrap().delete_conversation_by_id(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rename_conversation(state: tauri::State<AppState>, id: String, title: String) -> Result<(), String> {
    state.db.lock().unwrap().rename_conversation_by_id(&id, &title).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_api_key(state: tauri::State<AppState>) -> Result<Option<String>, String> {
    state.db.lock().unwrap().get_setting("api_key").map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_api_key(state: tauri::State<AppState>, key: String) -> Result<(), String> {
    state.db.lock().unwrap().set_setting("api_key", &key).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_model(state: tauri::State<AppState>) -> Result<String, String> {
    state.db.lock().unwrap()
        .get_setting("model")
        .map_err(|e| e.to_string())
        .map(|v| v.unwrap_or_else(|| "claude-sonnet-4-6".to_string()))
}

#[tauri::command]
pub fn set_model(state: tauri::State<AppState>, model: String) -> Result<(), String> {
    state.db.lock().unwrap().set_setting("model", &model).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_messages_from(state: tauri::State<AppState>, conversation_id: String, message_id: String) -> Result<(), String> {
    state.db.lock().unwrap().delete_messages_from(&conversation_id, &message_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_theme(state: tauri::State<AppState>) -> Result<String, String> {
    state.db.lock().unwrap()
        .get_setting("theme")
        .map_err(|e| e.to_string())
        .map(|v| v.unwrap_or_else(|| "dark".to_string()))
}

#[tauri::command]
pub fn set_theme(state: tauri::State<AppState>, theme: String) -> Result<(), String> {
    state.db.lock().unwrap().set_setting("theme", &theme).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_system_prompt(state: tauri::State<AppState>) -> Result<Option<String>, String> {
    state.db.lock().unwrap().get_setting("system_prompt").map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_mcp_servers(state: tauri::State<AppState>) -> Result<Vec<crate::mcp::McpServerConfig>, String> {
    let db = state.db.lock().unwrap();
    let json = db.get_setting("mcp_servers").map_err(|e| e.to_string())?;
    match json {
        Some(s) => serde_json::from_str(&s).map_err(|e| e.to_string()),
        None => Ok(Vec::new()),
    }
}

#[tauri::command]
pub fn set_mcp_servers(state: tauri::State<AppState>, servers: Vec<crate::mcp::McpServerConfig>) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    let json = serde_json::to_string(&servers).map_err(|e| e.to_string())?;
    db.set_setting("mcp_servers", &json).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_system_prompt(state: tauri::State<AppState>, prompt: String) -> Result<(), String> {
    if prompt.trim().is_empty() {
        state.db.lock().unwrap().remove_setting("system_prompt").map_err(|e| e.to_string())
    } else {
        state.db.lock().unwrap().set_setting("system_prompt", &prompt).map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn get_projects(state: tauri::State<AppState>) -> Result<Vec<Project>, String> {
    state.db.lock().unwrap().list_projects().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_project(state: tauri::State<AppState>, name: String, context: String) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    state.db.lock().unwrap().insert_project(&id, &name, &context).map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub fn update_project(state: tauri::State<AppState>, id: String, name: String, context: String) -> Result<(), String> {
    state.db.lock().unwrap().update_project(&id, &name, &context).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_project(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    state.db.lock().unwrap().delete_project_by_id(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_conversation_project(state: tauri::State<AppState>, conversation_id: String, project_id: Option<String>) -> Result<(), String> {
    state.db.lock().unwrap().set_conversation_project(&conversation_id, project_id.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_conversation_project(state: tauri::State<AppState>, conversation_id: String) -> Result<Option<String>, String> {
    state.db.lock().unwrap().get_conversation_project_id(&conversation_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_conversation(state: tauri::State<AppState>, conversation_id: String, format: String) -> Result<String, String> {
    let db = state.db.lock().unwrap();

    let conv: Conversation = db.conn.query_row(
        "SELECT id, title, created_at, updated_at FROM conversations WHERE id = ?1",
        params![conversation_id],
        |row| Ok(Conversation {
            id: row.get(0)?,
            title: row.get(1)?,
            created_at: row.get(2)?,
            updated_at: row.get(3)?,
        }),
    ).map_err(|e| e.to_string())?;

    let messages = db.list_messages(&conversation_id).map_err(|e| e.to_string())?;

    match format.as_str() {
        "markdown" => {
            let mut md = format!("# {}\n\n", conv.title);
            md.push_str(&format!("*Exported: {}*\n\n---\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M UTC")));
            for msg in &messages {
                let label = match msg.role.as_str() {
                    "user" => "**You**",
                    "assistant" => "**Claude**",
                    _ => "**System**",
                };
                md.push_str(&format!("{}\n\n{}\n\n---\n\n", label, msg.content));
            }
            Ok(md)
        }
        "json" => {
            let export = ExportedConversation {
                title: conv.title,
                created_at: conv.created_at,
                messages: messages.into_iter().map(|m| ExportedMessage {
                    role: m.role,
                    content: m.content,
                    created_at: m.created_at,
                }).collect(),
            };
            serde_json::to_string_pretty(&export).map_err(|e| e.to_string())
        }
        _ => Err("Unsupported format. Use 'markdown' or 'json'.".into()),
    }
}

// --- Provider settings ---

#[tauri::command]
pub fn get_provider(state: tauri::State<AppState>) -> Result<String, String> {
    state.db.lock().unwrap()
        .get_setting("provider")
        .map_err(|e| e.to_string())
        .map(|v| v.unwrap_or_else(|| "anthropic".to_string()))
}

#[tauri::command]
pub fn set_provider(state: tauri::State<AppState>, provider: String) -> Result<(), String> {
    state.db.lock().unwrap().set_setting("provider", &provider).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_openai_api_key(state: tauri::State<AppState>) -> Result<Option<String>, String> {
    state.db.lock().unwrap().get_setting("openai_api_key").map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_openai_api_key(state: tauri::State<AppState>, key: String) -> Result<(), String> {
    state.db.lock().unwrap().set_setting("openai_api_key", &key).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_openai_base_url(state: tauri::State<AppState>) -> Result<String, String> {
    state.db.lock().unwrap()
        .get_setting("openai_base_url")
        .map_err(|e| e.to_string())
        .map(|v| v.unwrap_or_else(|| "https://api.openai.com".to_string()))
}

#[tauri::command]
pub fn set_openai_base_url(state: tauri::State<AppState>, url: String) -> Result<(), String> {
    state.db.lock().unwrap().set_setting("openai_base_url", &url).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_ollama_base_url(state: tauri::State<AppState>) -> Result<String, String> {
    state.db.lock().unwrap()
        .get_setting("ollama_base_url")
        .map_err(|e| e.to_string())
        .map(|v| v.unwrap_or_else(|| "http://localhost:11434".to_string()))
}

#[tauri::command]
pub fn set_ollama_base_url(state: tauri::State<AppState>, url: String) -> Result<(), String> {
    state.db.lock().unwrap().set_setting("ollama_base_url", &url).map_err(|e| e.to_string())
}

// --- Custom CSS ---

#[tauri::command]
pub fn get_custom_css(state: tauri::State<AppState>) -> Result<String, String> {
    state.db.lock().unwrap()
        .get_setting("custom_css")
        .map_err(|e| e.to_string())
        .map(|v| v.unwrap_or_default())
}

#[tauri::command]
pub fn set_custom_css(state: tauri::State<AppState>, css: String) -> Result<(), String> {
    if css.trim().is_empty() {
        state.db.lock().unwrap().remove_setting("custom_css").map_err(|e| e.to_string())
    } else {
        state.db.lock().unwrap().set_setting("custom_css", &css).map_err(|e| e.to_string())
    }
}

// --- Prompts ---

#[tauri::command]
pub fn get_prompts(state: tauri::State<AppState>) -> Result<Vec<Prompt>, String> {
    state.db.lock().unwrap().list_prompts().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_prompt(state: tauri::State<AppState>, name: String, content: String) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    state.db.lock().unwrap().insert_prompt(&id, &name, &content).map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub fn update_prompt(state: tauri::State<AppState>, id: String, name: String, content: String) -> Result<(), String> {
    state.db.lock().unwrap().update_prompt_by_id(&id, &name, &content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_prompt(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    state.db.lock().unwrap().delete_prompt_by_id(&id).map_err(|e| e.to_string())
}

// --- Custom Commands (Plugin System) ---

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CustomCommand {
    pub name: String,
    pub command: String,
    pub description: String,
}

#[tauri::command]
pub fn get_custom_commands(state: tauri::State<AppState>) -> Result<Vec<CustomCommand>, String> {
    let db = state.db.lock().unwrap();
    let json = db.get_setting("custom_commands").map_err(|e| e.to_string())?;
    match json {
        Some(s) => serde_json::from_str(&s).map_err(|e| e.to_string()),
        None => Ok(Vec::new()),
    }
}

#[tauri::command]
pub fn set_custom_commands(state: tauri::State<AppState>, commands: Vec<CustomCommand>) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    let json = serde_json::to_string(&commands).map_err(|e| e.to_string())?;
    db.set_setting("custom_commands", &json).map_err(|e| e.to_string())
}
