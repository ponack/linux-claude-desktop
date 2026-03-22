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
                updated_at TEXT NOT NULL
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
            PRAGMA foreign_keys = ON;"
        )?;

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
pub fn set_system_prompt(state: tauri::State<AppState>, prompt: String) -> Result<(), String> {
    if prompt.trim().is_empty() {
        state.db.lock().unwrap().remove_setting("system_prompt").map_err(|e| e.to_string())
    } else {
        state.db.lock().unwrap().set_setting("system_prompt", &prompt).map_err(|e| e.to_string())
    }
}
