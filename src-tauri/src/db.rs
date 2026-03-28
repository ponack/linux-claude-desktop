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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CustomEndpoint {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub api_format: String,
    pub default_model: String,
    pub is_enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelPricing {
    pub id: String,
    pub model_pattern: String,
    pub input_cost_per_mtok: f64,
    pub output_cost_per_mtok: f64,
    pub provider: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RoutingRule {
    pub id: String,
    pub name: String,
    pub pattern: String,
    pub task_type: String,
    pub target_provider: String,
    pub target_model: String,
    pub priority: i32,
    pub is_enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComparisonSession {
    pub id: String,
    pub prompt: String,
    pub conversation_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComparisonResponse {
    pub id: String,
    pub session_id: String,
    pub provider: String,
    pub model: String,
    pub content: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub latency_ms: i64,
    pub estimated_cost: f64,
    pub rating: Option<i32>,
    pub notes: Option<String>,
    pub created_at: String,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new() -> Result<Self, rusqlite::Error> {
        let base = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let old_dir = base.join("ubuntu-claude-desktop");
        let data_dir = base.join("linux-claude-desktop");

        // Migrate data from old name if it exists and new doesn't
        if old_dir.exists() && !data_dir.exists() {
            std::fs::rename(&old_dir, &data_dir).ok();
        }

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
            CREATE TABLE IF NOT EXISTS scheduled_prompts (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                prompt TEXT NOT NULL,
                interval_ms INTEGER NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                last_run TEXT,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS token_usage (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                conversation_id TEXT NOT NULL,
                message_id TEXT NOT NULL,
                input_tokens INTEGER NOT NULL DEFAULT 0,
                output_tokens INTEGER NOT NULL DEFAULT 0,
                model TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL,
                FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS artifacts (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                title TEXT NOT NULL DEFAULT 'Untitled',
                artifact_type TEXT NOT NULL,
                language TEXT,
                current_version INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS artifact_versions (
                id TEXT PRIMARY KEY,
                artifact_id TEXT NOT NULL,
                version INTEGER NOT NULL,
                content TEXT NOT NULL,
                source TEXT NOT NULL DEFAULT 'claude',
                message_id TEXT,
                created_at TEXT NOT NULL,
                FOREIGN KEY (artifact_id) REFERENCES artifacts(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_artifact_versions_artifact ON artifact_versions(artifact_id, version);
            CREATE INDEX IF NOT EXISTS idx_artifacts_conversation ON artifacts(conversation_id);
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

        // Migration: create scheduled_prompts table if missing
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS scheduled_prompts (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                prompt TEXT NOT NULL,
                interval_ms INTEGER NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                last_run TEXT,
                created_at TEXT NOT NULL
            );"
        ).ok();

        // Migration: create token_usage table if missing
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS token_usage (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                conversation_id TEXT NOT NULL,
                message_id TEXT NOT NULL,
                input_tokens INTEGER NOT NULL DEFAULT 0,
                output_tokens INTEGER NOT NULL DEFAULT 0,
                model TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL,
                FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
            );"
        ).ok();

        // Migration: create projects table if missing
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                context TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL
            );"
        ).ok();

        // Migration: add workspace profile columns to projects
        let has_provider: bool = conn
            .prepare("SELECT sql FROM sqlite_master WHERE type='table' AND name='projects'")
            .and_then(|mut s| s.query_row([], |row| row.get::<_, String>(0)))
            .map(|sql| sql.contains("provider"))
            .unwrap_or(false);
        if !has_provider {
            conn.execute_batch(
                "ALTER TABLE projects ADD COLUMN provider TEXT;
                 ALTER TABLE projects ADD COLUMN api_key TEXT;
                 ALTER TABLE projects ADD COLUMN model TEXT;
                 ALTER TABLE projects ADD COLUMN system_prompt TEXT;"
            ).ok();
        }

        // Migration: create knowledge & memory tables
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS conversation_memory (
                id TEXT PRIMARY KEY,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_memory_key ON conversation_memory(key);

            CREATE TABLE IF NOT EXISTS knowledge_base (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                source_type TEXT NOT NULL DEFAULT 'manual',
                source_url TEXT,
                file_path TEXT,
                project_id TEXT,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE SET NULL
            );
            CREATE INDEX IF NOT EXISTS idx_knowledge_project ON knowledge_base(project_id);

            CREATE TABLE IF NOT EXISTS file_watches (
                id TEXT PRIMARY KEY,
                file_path TEXT NOT NULL UNIQUE,
                knowledge_id TEXT NOT NULL,
                last_modified TEXT,
                created_at TEXT NOT NULL,
                FOREIGN KEY (knowledge_id) REFERENCES knowledge_base(id) ON DELETE CASCADE
            );"
        ).ok();

        // Migration: Phase 10 — Multi-Model & Comparison tables
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS custom_endpoints (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                base_url TEXT NOT NULL,
                api_key TEXT NOT NULL DEFAULT '',
                api_format TEXT NOT NULL DEFAULT 'openai',
                default_model TEXT NOT NULL DEFAULT '',
                is_enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS model_pricing (
                id TEXT PRIMARY KEY,
                model_pattern TEXT NOT NULL,
                input_cost_per_mtok REAL NOT NULL DEFAULT 0.0,
                output_cost_per_mtok REAL NOT NULL DEFAULT 0.0,
                provider TEXT NOT NULL DEFAULT '',
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS routing_rules (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                pattern TEXT NOT NULL,
                task_type TEXT NOT NULL DEFAULT 'custom',
                target_provider TEXT NOT NULL,
                target_model TEXT NOT NULL,
                priority INTEGER NOT NULL DEFAULT 0,
                is_enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS comparison_sessions (
                id TEXT PRIMARY KEY,
                prompt TEXT NOT NULL,
                conversation_id TEXT,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS comparison_responses (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                provider TEXT NOT NULL,
                model TEXT NOT NULL,
                content TEXT NOT NULL DEFAULT '',
                input_tokens INTEGER NOT NULL DEFAULT 0,
                output_tokens INTEGER NOT NULL DEFAULT 0,
                latency_ms INTEGER NOT NULL DEFAULT 0,
                estimated_cost REAL NOT NULL DEFAULT 0.0,
                rating INTEGER,
                notes TEXT,
                created_at TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES comparison_sessions(id) ON DELETE CASCADE
            );"
        ).ok();

        // Seed default model pricing if empty
        let pricing_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM model_pricing", [], |row| row.get(0)
        ).unwrap_or(0);
        if pricing_count == 0 {
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute_batch(&format!(
                "INSERT OR IGNORE INTO model_pricing (id, model_pattern, input_cost_per_mtok, output_cost_per_mtok, provider, updated_at) VALUES
                ('p1', 'claude-opus-4-6', 15.0, 75.0, 'anthropic', '{now}'),
                ('p2', 'claude-sonnet-4-6', 3.0, 15.0, 'anthropic', '{now}'),
                ('p3', 'claude-haiku-4-5', 0.8, 4.0, 'anthropic', '{now}'),
                ('p4', 'gpt-4o', 2.5, 10.0, 'openai', '{now}'),
                ('p5', 'gpt-4o-mini', 0.15, 0.6, 'openai', '{now}'),
                ('p6', 'gpt-4.1', 2.0, 8.0, 'openai', '{now}'),
                ('p7', 'gpt-4.1-mini', 0.4, 1.6, 'openai', '{now}'),
                ('p8', 'gpt-4.1-nano', 0.1, 0.4, 'openai', '{now}'),
                ('p9', 'o3', 2.0, 8.0, 'openai', '{now}'),
                ('p10', 'o4-mini', 1.1, 4.4, 'openai', '{now}');"
            )).ok();
        }

        // Migration: add estimated_cost to token_usage
        conn.execute_batch("ALTER TABLE token_usage ADD COLUMN estimated_cost REAL NOT NULL DEFAULT 0.0;").ok();

        // Migration: create artifacts tables if missing
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS artifacts (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                title TEXT NOT NULL DEFAULT 'Untitled',
                artifact_type TEXT NOT NULL,
                language TEXT,
                current_version INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS artifact_versions (
                id TEXT PRIMARY KEY,
                artifact_id TEXT NOT NULL,
                version INTEGER NOT NULL,
                content TEXT NOT NULL,
                source TEXT NOT NULL DEFAULT 'claude',
                message_id TEXT,
                created_at TEXT NOT NULL,
                FOREIGN KEY (artifact_id) REFERENCES artifacts(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_artifact_versions_artifact ON artifact_versions(artifact_id, version);
            CREATE INDEX IF NOT EXISTS idx_artifacts_conversation ON artifacts(conversation_id);"
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

    pub fn list_messages_paginated(&self, conversation_id: &str, limit: i64, offset: i64) -> Result<Vec<Message>, rusqlite::Error> {
        // Returns the most recent `limit` messages, offset from the end
        // We query descending then reverse so the caller gets chronological order
        let mut stmt = self.conn.prepare(
            "SELECT id, conversation_id, role, content, created_at FROM messages WHERE conversation_id = ?1 ORDER BY created_at DESC LIMIT ?2 OFFSET ?3"
        )?;
        let rows = stmt.query_map(params![conversation_id, limit, offset], |row| {
            Ok(Message {
                id: row.get(0)?,
                conversation_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;
        let mut msgs: Vec<Message> = rows.collect::<Result<Vec<_>, _>>()?;
        msgs.reverse();
        Ok(msgs)
    }

    pub fn count_messages(&self, conversation_id: &str) -> Result<i64, rusqlite::Error> {
        self.conn.query_row(
            "SELECT COUNT(*) FROM messages WHERE conversation_id = ?1",
            params![conversation_id],
            |row| row.get(0),
        )
    }

    pub fn get_db_path(&self) -> String {
        self.conn.path().unwrap_or("").to_string()
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
            "SELECT id, name, context, created_at, provider, api_key, model, system_prompt FROM projects ORDER BY name ASC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                context: row.get(2)?,
                created_at: row.get(3)?,
                provider: row.get(4)?,
                api_key: row.get(5)?,
                model: row.get(6)?,
                system_prompt: row.get(7)?,
            })
        })?;
        rows.collect()
    }

    pub fn insert_project(&self, id: &str, name: &str, context: &str, provider: Option<&str>, api_key: Option<&str>, model: Option<&str>, system_prompt: Option<&str>) -> Result<(), rusqlite::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO projects (id, name, context, created_at, provider, api_key, model, system_prompt) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![id, name, context, &now, provider, api_key, model, system_prompt],
        )?;
        Ok(())
    }

    pub fn update_project(&self, id: &str, name: &str, context: &str, provider: Option<&str>, api_key: Option<&str>, model: Option<&str>, system_prompt: Option<&str>) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE projects SET name = ?1, context = ?2, provider = ?3, api_key = ?4, model = ?5, system_prompt = ?6 WHERE id = ?7",
            params![name, context, provider, api_key, model, system_prompt, id],
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

    pub fn get_project(&self, project_id: &str) -> Result<Option<Project>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, context, created_at, provider, api_key, model, system_prompt FROM projects WHERE id = ?1"
        )?;
        let mut rows = stmt.query_map(params![project_id], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                context: row.get(2)?,
                created_at: row.get(3)?,
                provider: row.get(4)?,
                api_key: row.get(5)?,
                model: row.get(6)?,
                system_prompt: row.get(7)?,
            })
        })?;
        match rows.next() {
            Some(Ok(p)) => Ok(Some(p)),
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

    pub fn list_scheduled_prompts(&self) -> Result<Vec<ScheduledPrompt>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, prompt, interval_ms, enabled, last_run, created_at FROM scheduled_prompts ORDER BY name ASC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ScheduledPrompt {
                id: row.get(0)?,
                name: row.get(1)?,
                prompt: row.get(2)?,
                interval_ms: row.get(3)?,
                enabled: row.get::<_, i64>(4)? != 0,
                last_run: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        rows.collect()
    }

    pub fn insert_scheduled_prompt(&self, id: &str, name: &str, prompt: &str, interval_ms: i64) -> Result<(), rusqlite::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO scheduled_prompts (id, name, prompt, interval_ms, enabled, created_at) VALUES (?1, ?2, ?3, ?4, 1, ?5)",
            params![id, name, prompt, interval_ms, &now],
        )?;
        Ok(())
    }

    pub fn update_scheduled_prompt(&self, id: &str, name: &str, prompt: &str, interval_ms: i64, enabled: bool) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE scheduled_prompts SET name = ?1, prompt = ?2, interval_ms = ?3, enabled = ?4 WHERE id = ?5",
            params![name, prompt, interval_ms, enabled as i64, id],
        )?;
        Ok(())
    }

    pub fn delete_scheduled_prompt(&self, id: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute("DELETE FROM scheduled_prompts WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn update_scheduled_prompt_last_run(&self, id: &str) -> Result<(), rusqlite::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE scheduled_prompts SET last_run = ?1 WHERE id = ?2",
            params![&now, id],
        )?;
        Ok(())
    }

    pub fn get_due_scheduled_prompts(&self) -> Result<Vec<ScheduledPrompt>, rusqlite::Error> {
        let now = chrono::Utc::now();
        let all = self.list_scheduled_prompts()?;
        Ok(all.into_iter().filter(|sp| {
            if !sp.enabled { return false; }
            match &sp.last_run {
                Some(lr) => {
                    if let Ok(last) = chrono::DateTime::parse_from_rfc3339(lr) {
                        let elapsed = now.signed_duration_since(last).num_milliseconds();
                        elapsed >= sp.interval_ms
                    } else {
                        true
                    }
                }
                None => true,
            }
        }).collect())
    }

    pub fn fork_conversation(&self, source_conversation_id: &str, at_message_id: &str, new_title: &str) -> Result<String, rusqlite::Error> {
        let new_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        // Get the created_at of the fork point message
        let fork_at: String = self.conn.query_row(
            "SELECT created_at FROM messages WHERE id = ?1",
            params![at_message_id],
            |row| row.get(0),
        )?;

        // Create the new conversation
        self.conn.execute(
            "INSERT INTO conversations (id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params![&new_id, new_title, &now, &now],
        )?;

        // Copy the source project association
        let project_id: Option<String> = self.conn.query_row(
            "SELECT project_id FROM conversations WHERE id = ?1",
            params![source_conversation_id],
            |row| row.get(0),
        ).unwrap_or(None);
        if let Some(pid) = project_id {
            self.conn.execute(
                "UPDATE conversations SET project_id = ?1 WHERE id = ?2",
                params![&pid, &new_id],
            )?;
        }

        // Copy messages up to and including the fork point
        let mut stmt = self.conn.prepare(
            "SELECT id, role, content, created_at FROM messages WHERE conversation_id = ?1 AND created_at <= ?2 ORDER BY created_at ASC"
        )?;
        let messages: Vec<(String, String, String, String)> = stmt.query_map(
            params![source_conversation_id, &fork_at],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?.filter_map(|r| r.ok()).collect();

        for (_old_id, role, content, created_at) in &messages {
            let msg_id = uuid::Uuid::new_v4().to_string();
            self.conn.execute(
                "INSERT INTO messages (id, conversation_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![&msg_id, &new_id, role, content, created_at],
            )?;
        }

        Ok(new_id)
    }

    pub fn insert_token_usage(
        &self,
        conversation_id: &str,
        message_id: &str,
        input_tokens: i64,
        output_tokens: i64,
        model: &str,
    ) -> Result<(), rusqlite::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        let cost = self.estimate_cost(model, input_tokens, output_tokens).unwrap_or(0.0);
        self.conn.execute(
            "INSERT INTO token_usage (conversation_id, message_id, input_tokens, output_tokens, model, estimated_cost, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![conversation_id, message_id, input_tokens, output_tokens, model, cost, &now],
        )?;
        Ok(())
    }

    pub fn get_conversation_token_usage(&self, conversation_id: &str) -> Result<TokenUsageSummary, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT COALESCE(SUM(input_tokens), 0), COALESCE(SUM(output_tokens), 0), COUNT(*) FROM token_usage WHERE conversation_id = ?1"
        )?;
        stmt.query_row(params![conversation_id], |row| {
            Ok(TokenUsageSummary {
                input_tokens: row.get(0)?,
                output_tokens: row.get(1)?,
                total_tokens: row.get::<_, i64>(0)? + row.get::<_, i64>(1)?,
                message_count: row.get(2)?,
            })
        })
    }

    pub fn get_total_token_usage(&self) -> Result<TokenUsageSummary, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT COALESCE(SUM(input_tokens), 0), COALESCE(SUM(output_tokens), 0), COUNT(*) FROM token_usage"
        )?;
        stmt.query_row([], |row| {
            Ok(TokenUsageSummary {
                input_tokens: row.get(0)?,
                output_tokens: row.get(1)?,
                total_tokens: row.get::<_, i64>(0)? + row.get::<_, i64>(1)?,
                message_count: row.get(2)?,
            })
        })
    }

    // --- Artifact methods ---

    pub fn insert_artifact(&self, id: &str, conversation_id: &str, title: &str, artifact_type: &str, language: Option<&str>) -> Result<(), rusqlite::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO artifacts (id, conversation_id, title, artifact_type, language, current_version, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?7)",
            params![id, conversation_id, title, artifact_type, language, &now, &now],
        )?;
        Ok(())
    }

    pub fn insert_artifact_version(&self, id: &str, artifact_id: &str, version: i64, content: &str, source: &str, message_id: Option<&str>) -> Result<(), rusqlite::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO artifact_versions (id, artifact_id, version, content, source, message_id, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, artifact_id, version, content, source, message_id, &now],
        )?;
        self.conn.execute(
            "UPDATE artifacts SET current_version = ?1, updated_at = ?2 WHERE id = ?3",
            params![version, &now, artifact_id],
        )?;
        Ok(())
    }

    pub fn list_artifacts_for_conversation(&self, conversation_id: &str) -> Result<Vec<Artifact>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, conversation_id, title, artifact_type, language, current_version, created_at, updated_at FROM artifacts WHERE conversation_id = ?1 ORDER BY created_at ASC"
        )?;
        let rows = stmt.query_map(params![conversation_id], |row| {
            Ok(Artifact {
                id: row.get(0)?,
                conversation_id: row.get(1)?,
                title: row.get(2)?,
                artifact_type: row.get(3)?,
                language: row.get(4)?,
                current_version: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_artifact_latest_content(&self, artifact_id: &str) -> Result<Option<String>, rusqlite::Error> {
        let current_version: i64 = self.conn.query_row(
            "SELECT current_version FROM artifacts WHERE id = ?1",
            params![artifact_id],
            |row| row.get(0),
        )?;
        let mut stmt = self.conn.prepare(
            "SELECT content FROM artifact_versions WHERE artifact_id = ?1 AND version = ?2"
        )?;
        let mut rows = stmt.query_map(params![artifact_id, current_version], |row| row.get::<_, String>(0))?;
        match rows.next() {
            Some(Ok(c)) => Ok(Some(c)),
            _ => Ok(None),
        }
    }

    pub fn list_artifact_versions(&self, artifact_id: &str) -> Result<Vec<ArtifactVersion>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, artifact_id, version, content, source, message_id, created_at FROM artifact_versions WHERE artifact_id = ?1 ORDER BY version ASC"
        )?;
        let rows = stmt.query_map(params![artifact_id], |row| {
            Ok(ArtifactVersion {
                id: row.get(0)?,
                artifact_id: row.get(1)?,
                version: row.get(2)?,
                content: row.get(3)?,
                source: row.get(4)?,
                message_id: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        rows.collect()
    }

    pub fn update_artifact_title(&self, id: &str, title: &str) -> Result<(), rusqlite::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE artifacts SET title = ?1, updated_at = ?2 WHERE id = ?3",
            params![title, &now, id],
        )?;
        Ok(())
    }

    pub fn delete_artifact(&self, id: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute("DELETE FROM artifact_versions WHERE artifact_id = ?1", params![id])?;
        self.conn.execute("DELETE FROM artifacts WHERE id = ?1", params![id])?;
        Ok(())
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

    // --- Conversation Memory ---

    pub fn list_memory_entries(&self) -> Result<Vec<MemoryEntry>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, key, value, created_at, updated_at FROM conversation_memory ORDER BY key ASC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(MemoryEntry {
                id: row.get(0)?,
                key: row.get(1)?,
                value: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;
        rows.collect()
    }

    pub fn upsert_memory_entry(&self, key: &str, value: &str) -> Result<String, rusqlite::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        // Check if key exists
        let existing: Option<String> = self.conn.query_row(
            "SELECT id FROM conversation_memory WHERE key = ?1",
            params![key],
            |row| row.get(0),
        ).ok();
        if let Some(id) = existing {
            self.conn.execute(
                "UPDATE conversation_memory SET value = ?1, updated_at = ?2 WHERE id = ?3",
                params![value, &now, &id],
            )?;
            Ok(id)
        } else {
            let id = uuid::Uuid::new_v4().to_string();
            self.conn.execute(
                "INSERT INTO conversation_memory (id, key, value, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![&id, key, value, &now, &now],
            )?;
            Ok(id)
        }
    }

    pub fn delete_memory_entry(&self, id: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute("DELETE FROM conversation_memory WHERE id = ?1", params![id])?;
        Ok(())
    }

    // --- Custom Endpoints ---

    pub fn list_custom_endpoints(&self) -> Result<Vec<CustomEndpoint>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, base_url, api_key, api_format, default_model, is_enabled, created_at, updated_at FROM custom_endpoints ORDER BY name ASC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(CustomEndpoint {
                id: row.get(0)?, name: row.get(1)?, base_url: row.get(2)?,
                api_key: row.get(3)?, api_format: row.get(4)?, default_model: row.get(5)?,
                is_enabled: row.get::<_, i32>(6)? != 0, created_at: row.get(7)?, updated_at: row.get(8)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_custom_endpoint(&self, id: &str) -> Result<Option<CustomEndpoint>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, base_url, api_key, api_format, default_model, is_enabled, created_at, updated_at FROM custom_endpoints WHERE id = ?1"
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(CustomEndpoint {
                id: row.get(0)?, name: row.get(1)?, base_url: row.get(2)?,
                api_key: row.get(3)?, api_format: row.get(4)?, default_model: row.get(5)?,
                is_enabled: row.get::<_, i32>(6)? != 0, created_at: row.get(7)?, updated_at: row.get(8)?,
            })
        })?;
        match rows.next() {
            Some(Ok(ep)) => Ok(Some(ep)),
            _ => Ok(None),
        }
    }

    // --- Model Pricing ---

    pub fn list_model_pricing(&self) -> Result<Vec<ModelPricing>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, model_pattern, input_cost_per_mtok, output_cost_per_mtok, provider, updated_at FROM model_pricing ORDER BY provider, model_pattern"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ModelPricing {
                id: row.get(0)?, model_pattern: row.get(1)?, input_cost_per_mtok: row.get(2)?,
                output_cost_per_mtok: row.get(3)?, provider: row.get(4)?, updated_at: row.get(5)?,
            })
        })?;
        rows.collect()
    }

    pub fn estimate_cost(&self, model: &str, input_tokens: i64, output_tokens: i64) -> Result<f64, rusqlite::Error> {
        // Try exact match first, then prefix match
        let pricing: Option<(f64, f64)> = self.conn.query_row(
            "SELECT input_cost_per_mtok, output_cost_per_mtok FROM model_pricing WHERE model_pattern = ?1",
            params![model], |row| Ok((row.get(0)?, row.get(1)?)),
        ).ok().or_else(|| {
            // Try prefix match (e.g. "claude-sonnet-4-6" matches "claude-sonnet-4-6-20260101")
            self.conn.query_row(
                "SELECT input_cost_per_mtok, output_cost_per_mtok FROM model_pricing WHERE ?1 LIKE model_pattern || '%' ORDER BY LENGTH(model_pattern) DESC LIMIT 1",
                params![model], |row| Ok((row.get(0)?, row.get(1)?)),
            ).ok()
        });

        match pricing {
            Some((input_cost, output_cost)) => {
                let cost = (input_tokens as f64 * input_cost + output_tokens as f64 * output_cost) / 1_000_000.0;
                Ok(cost)
            }
            None => Ok(0.0),
        }
    }

    // --- Routing Rules ---

    pub fn list_routing_rules(&self) -> Result<Vec<RoutingRule>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, pattern, task_type, target_provider, target_model, priority, is_enabled, created_at FROM routing_rules ORDER BY priority DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(RoutingRule {
                id: row.get(0)?, name: row.get(1)?, pattern: row.get(2)?,
                task_type: row.get(3)?, target_provider: row.get(4)?, target_model: row.get(5)?,
                priority: row.get(6)?, is_enabled: row.get::<_, i32>(7)? != 0, created_at: row.get(8)?,
            })
        })?;
        rows.collect()
    }

    pub fn match_routing_rule(&self, prompt: &str) -> Result<Option<RoutingRule>, rusqlite::Error> {
        let rules = self.list_routing_rules()?;
        let prompt_lower = prompt.to_lowercase();
        for rule in rules {
            if !rule.is_enabled { continue; }
            // Simple keyword matching (case-insensitive)
            let patterns: Vec<&str> = rule.pattern.split('|').collect();
            for pat in patterns {
                if prompt_lower.contains(pat.trim().to_lowercase().as_str()) {
                    return Ok(Some(rule));
                }
            }
        }
        Ok(None)
    }

    // --- Comparison ---

    pub fn create_comparison_session(&self, id: &str, prompt: &str, created_at: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO comparison_sessions (id, prompt, created_at) VALUES (?1, ?2, ?3)",
            params![id, prompt, created_at],
        )?;
        Ok(())
    }

    pub fn create_comparison_response(&self, id: &str, session_id: &str, provider: &str, model: &str, created_at: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO comparison_responses (id, session_id, provider, model, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, session_id, provider, model, created_at],
        )?;
        Ok(())
    }

    pub fn update_comparison_response(&self, id: &str, content: &str, input_tokens: i64, output_tokens: i64, latency_ms: i64, estimated_cost: f64) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE comparison_responses SET content=?1, input_tokens=?2, output_tokens=?3, latency_ms=?4, estimated_cost=?5 WHERE id=?6",
            params![content, input_tokens, output_tokens, latency_ms, estimated_cost, id],
        )?;
        Ok(())
    }

    pub fn list_comparison_sessions(&self) -> Result<Vec<ComparisonSession>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, prompt, conversation_id, created_at FROM comparison_sessions ORDER BY created_at DESC LIMIT 50"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ComparisonSession {
                id: row.get(0)?, prompt: row.get(1)?, conversation_id: row.get(2)?, created_at: row.get(3)?,
            })
        })?;
        rows.collect()
    }

    pub fn list_comparison_responses(&self, session_id: &str) -> Result<Vec<ComparisonResponse>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, provider, model, content, input_tokens, output_tokens, latency_ms, estimated_cost, rating, notes, created_at FROM comparison_responses WHERE session_id = ?1"
        )?;
        let rows = stmt.query_map(params![session_id], |row| {
            Ok(ComparisonResponse {
                id: row.get(0)?, session_id: row.get(1)?, provider: row.get(2)?,
                model: row.get(3)?, content: row.get(4)?, input_tokens: row.get(5)?,
                output_tokens: row.get(6)?, latency_ms: row.get(7)?, estimated_cost: row.get(8)?,
                rating: row.get(9)?, notes: row.get(10)?, created_at: row.get(11)?,
            })
        })?;
        rows.collect()
    }

    // --- Knowledge Base ---

    pub fn list_knowledge_entries(&self, project_id: Option<&str>) -> Result<Vec<KnowledgeEntry>, rusqlite::Error> {
        match project_id {
            Some(pid) => {
                let mut stmt = self.conn.prepare(
                    "SELECT id, title, content, source_type, source_url, file_path, project_id, enabled, created_at, updated_at FROM knowledge_base WHERE project_id = ?1 ORDER BY title ASC"
                )?;
                let rows = stmt.query_map(params![pid], |row| {
                    Ok(KnowledgeEntry {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        content: row.get(2)?,
                        source_type: row.get(3)?,
                        source_url: row.get(4)?,
                        file_path: row.get(5)?,
                        project_id: row.get(6)?,
                        enabled: row.get::<_, i32>(7)? != 0,
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                    })
                })?;
                rows.collect()
            }
            None => {
                let mut stmt = self.conn.prepare(
                    "SELECT id, title, content, source_type, source_url, file_path, project_id, enabled, created_at, updated_at FROM knowledge_base ORDER BY title ASC"
                )?;
                let rows = stmt.query_map([], |row| {
                    Ok(KnowledgeEntry {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        content: row.get(2)?,
                        source_type: row.get(3)?,
                        source_url: row.get(4)?,
                        file_path: row.get(5)?,
                        project_id: row.get(6)?,
                        enabled: row.get::<_, i32>(7)? != 0,
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                    })
                })?;
                rows.collect()
            }
        }
    }

    pub fn insert_knowledge_entry(
        &self, id: &str, title: &str, content: &str, source_type: &str,
        source_url: Option<&str>, file_path: Option<&str>, project_id: Option<&str>,
    ) -> Result<(), rusqlite::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO knowledge_base (id, title, content, source_type, source_url, file_path, project_id, enabled, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8, ?9)",
            params![id, title, content, source_type, source_url, file_path, project_id, &now, &now],
        )?;
        Ok(())
    }

    pub fn update_knowledge_entry(
        &self, id: &str, title: &str, content: &str, enabled: bool,
    ) -> Result<(), rusqlite::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE knowledge_base SET title = ?1, content = ?2, enabled = ?3, updated_at = ?4 WHERE id = ?5",
            params![title, content, if enabled { 1 } else { 0 }, &now, id],
        )?;
        Ok(())
    }

    pub fn delete_knowledge_entry(&self, id: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute("DELETE FROM file_watches WHERE knowledge_id = ?1", params![id])?;
        self.conn.execute("DELETE FROM knowledge_base WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_enabled_knowledge_for_context(&self, project_id: Option<&str>) -> Result<Vec<KnowledgeEntry>, rusqlite::Error> {
        match project_id {
            Some(pid) => {
                let mut stmt = self.conn.prepare(
                    "SELECT id, title, content, source_type, source_url, file_path, project_id, enabled, created_at, updated_at FROM knowledge_base WHERE enabled = 1 AND (project_id IS NULL OR project_id = ?1) ORDER BY title ASC"
                )?;
                let rows = stmt.query_map(params![pid], |row| {
                    Ok(KnowledgeEntry {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        content: row.get(2)?,
                        source_type: row.get(3)?,
                        source_url: row.get(4)?,
                        file_path: row.get(5)?,
                        project_id: row.get(6)?,
                        enabled: row.get::<_, i32>(7)? != 0,
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                    })
                })?;
                rows.collect()
            }
            None => {
                let mut stmt = self.conn.prepare(
                    "SELECT id, title, content, source_type, source_url, file_path, project_id, enabled, created_at, updated_at FROM knowledge_base WHERE enabled = 1 AND project_id IS NULL ORDER BY title ASC"
                )?;
                let rows = stmt.query_map([], |row| {
                    Ok(KnowledgeEntry {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        content: row.get(2)?,
                        source_type: row.get(3)?,
                        source_url: row.get(4)?,
                        file_path: row.get(5)?,
                        project_id: row.get(6)?,
                        enabled: row.get::<_, i32>(7)? != 0,
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                    })
                })?;
                rows.collect()
            }
        }
    }

    pub fn update_knowledge_content(&self, id: &str, content: &str) -> Result<(), rusqlite::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE knowledge_base SET content = ?1, updated_at = ?2 WHERE id = ?3",
            params![content, &now, id],
        )?;
        Ok(())
    }

    // --- File Watches ---

    pub fn list_file_watches(&self) -> Result<Vec<FileWatch>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, file_path, knowledge_id, last_modified, created_at FROM file_watches ORDER BY file_path ASC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(FileWatch {
                id: row.get(0)?,
                file_path: row.get(1)?,
                knowledge_id: row.get(2)?,
                last_modified: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;
        rows.collect()
    }

    pub fn insert_file_watch(&self, id: &str, file_path: &str, knowledge_id: &str, last_modified: Option<&str>) -> Result<(), rusqlite::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT OR REPLACE INTO file_watches (id, file_path, knowledge_id, last_modified, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, file_path, knowledge_id, last_modified, &now],
        )?;
        Ok(())
    }

    pub fn delete_file_watch(&self, id: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute("DELETE FROM file_watches WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn update_file_watch_modified(&self, id: &str, last_modified: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE file_watches SET last_modified = ?1 WHERE id = ?2",
            params![last_modified, id],
        )?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub context: String,
    pub created_at: String,
    pub provider: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub system_prompt: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Artifact {
    pub id: String,
    pub conversation_id: String,
    pub title: String,
    pub artifact_type: String,
    pub language: Option<String>,
    pub current_version: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArtifactVersion {
    pub id: String,
    pub artifact_id: String,
    pub version: i64,
    pub content: String,
    pub source: String,
    pub message_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Prompt {
    pub id: String,
    pub name: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScheduledPrompt {
    pub id: String,
    pub name: String,
    pub prompt: String,
    pub interval_ms: i64,
    pub enabled: bool,
    pub last_run: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TokenUsageSummary {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub message_count: i64,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MemoryEntry {
    pub id: String,
    pub key: String,
    pub value: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KnowledgeEntry {
    pub id: String,
    pub title: String,
    pub content: String,
    pub source_type: String,
    pub source_url: Option<String>,
    pub file_path: Option<String>,
    pub project_id: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileWatch {
    pub id: String,
    pub file_path: String,
    pub knowledge_id: String,
    pub last_modified: Option<String>,
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
pub fn get_messages_paginated(state: tauri::State<AppState>, conversation_id: String, limit: i64, offset: i64) -> Result<Vec<Message>, String> {
    state.db.lock().unwrap().list_messages_paginated(&conversation_id, limit, offset).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_message_count(state: tauri::State<AppState>, conversation_id: String) -> Result<i64, String> {
    state.db.lock().unwrap().count_messages(&conversation_id).map_err(|e| e.to_string())
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
pub fn get_custom_endpoint_id(state: tauri::State<AppState>) -> Result<String, String> {
    state.db.lock().unwrap()
        .get_setting("custom_endpoint_id").map_err(|e| e.to_string())
        .map(|v| v.unwrap_or_default())
}

#[tauri::command]
pub fn set_custom_endpoint_id(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    state.db.lock().unwrap().set_setting("custom_endpoint_id", &id).map_err(|e| e.to_string())
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
pub fn get_font_size(state: tauri::State<AppState>) -> Result<String, String> {
    state.db.lock().unwrap()
        .get_setting("font_size")
        .map_err(|e| e.to_string())
        .map(|v| v.unwrap_or_else(|| "14".to_string()))
}

#[tauri::command]
pub fn set_font_size(state: tauri::State<AppState>, size: String) -> Result<(), String> {
    state.db.lock().unwrap().set_setting("font_size", &size).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_reduce_motion(state: tauri::State<AppState>) -> Result<bool, String> {
    let v = state.db.lock().unwrap()
        .get_setting("reduce_motion")
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    Ok(v == "true")
}

#[tauri::command]
pub fn set_reduce_motion(state: tauri::State<AppState>, enabled: bool) -> Result<(), String> {
    state.db.lock().unwrap()
        .set_setting("reduce_motion", if enabled { "true" } else { "false" })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_high_contrast(state: tauri::State<AppState>) -> Result<bool, String> {
    let v = state.db.lock().unwrap()
        .get_setting("high_contrast")
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    Ok(v == "true")
}

#[tauri::command]
pub fn set_high_contrast(state: tauri::State<AppState>, enabled: bool) -> Result<(), String> {
    state.db.lock().unwrap()
        .set_setting("high_contrast", if enabled { "true" } else { "false" })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_cu_model(state: tauri::State<AppState>) -> Result<String, String> {
    state.db.lock().unwrap()
        .get_setting("cu_model").map_err(|e| e.to_string())
        .map(|v| v.unwrap_or_else(|| "claude-opus-4-6".to_string()))
}

#[tauri::command]
pub fn set_cu_model(state: tauri::State<AppState>, model: String) -> Result<(), String> {
    state.db.lock().unwrap().set_setting("cu_model", &model).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_tts_enabled(state: tauri::State<AppState>) -> Result<bool, String> {
    let v = state.db.lock().unwrap()
        .get_setting("tts_enabled").map_err(|e| e.to_string())?.unwrap_or_default();
    Ok(v == "true")
}

#[tauri::command]
pub fn set_tts_enabled(state: tauri::State<AppState>, enabled: bool) -> Result<(), String> {
    state.db.lock().unwrap()
        .set_setting("tts_enabled", if enabled { "true" } else { "false" })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_tts_rate(state: tauri::State<AppState>) -> Result<i32, String> {
    let v = state.db.lock().unwrap()
        .get_setting("tts_rate").map_err(|e| e.to_string())?
        .unwrap_or_else(|| "100".to_string());
    Ok(v.parse().unwrap_or(100))
}

#[tauri::command]
pub fn set_tts_rate(state: tauri::State<AppState>, rate: i32) -> Result<(), String> {
    state.db.lock().unwrap().set_setting("tts_rate", &rate.to_string()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_stt_enabled(state: tauri::State<AppState>) -> Result<bool, String> {
    let v = state.db.lock().unwrap()
        .get_setting("stt_enabled").map_err(|e| e.to_string())?.unwrap_or_default();
    Ok(v == "true")
}

#[tauri::command]
pub fn set_stt_enabled(state: tauri::State<AppState>, enabled: bool) -> Result<(), String> {
    state.db.lock().unwrap()
        .set_setting("stt_enabled", if enabled { "true" } else { "false" })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_whisper_model_path(state: tauri::State<AppState>) -> Result<String, String> {
    state.db.lock().unwrap()
        .get_setting("whisper_model_path").map_err(|e| e.to_string())
        .map(|v| v.unwrap_or_default())
}

#[tauri::command]
pub fn set_whisper_model_path(state: tauri::State<AppState>, path: String) -> Result<(), String> {
    state.db.lock().unwrap().set_setting("whisper_model_path", &path).map_err(|e| e.to_string())
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
pub fn create_project(state: tauri::State<AppState>, name: String, context: String, provider: Option<String>, api_key: Option<String>, model: Option<String>, system_prompt: Option<String>) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    state.db.lock().unwrap().insert_project(&id, &name, &context, provider.as_deref(), api_key.as_deref(), model.as_deref(), system_prompt.as_deref()).map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub fn update_project(state: tauri::State<AppState>, id: String, name: String, context: String, provider: Option<String>, api_key: Option<String>, model: Option<String>, system_prompt: Option<String>) -> Result<(), String> {
    state.db.lock().unwrap().update_project(&id, &name, &context, provider.as_deref(), api_key.as_deref(), model.as_deref(), system_prompt.as_deref()).map_err(|e| e.to_string())
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

// --- Scheduled Prompts ---

#[tauri::command]
pub fn get_scheduled_prompts(state: tauri::State<AppState>) -> Result<Vec<ScheduledPrompt>, String> {
    state.db.lock().unwrap().list_scheduled_prompts().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_scheduled_prompt(state: tauri::State<AppState>, name: String, prompt: String, interval_ms: i64) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    state.db.lock().unwrap().insert_scheduled_prompt(&id, &name, &prompt, interval_ms).map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub fn update_scheduled_prompt(state: tauri::State<AppState>, id: String, name: String, prompt: String, interval_ms: i64, enabled: bool) -> Result<(), String> {
    state.db.lock().unwrap().update_scheduled_prompt(&id, &name, &prompt, interval_ms, enabled).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_scheduled_prompt(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    state.db.lock().unwrap().delete_scheduled_prompt(&id).map_err(|e| e.to_string())
}

// --- Conversation Branching ---

#[tauri::command]
pub fn fork_conversation(state: tauri::State<AppState>, conversation_id: String, message_id: String) -> Result<String, String> {
    let db = state.db.lock().unwrap();
    // Generate a title for the fork
    let title = db.conn.query_row(
        "SELECT title FROM conversations WHERE id = ?1",
        params![conversation_id],
        |row| row.get::<_, String>(0),
    ).map(|t| format!("{} (fork)", t))
    .unwrap_or_else(|_| "Forked conversation".to_string());

    db.fork_conversation(&conversation_id, &message_id, &title).map_err(|e| e.to_string())
}

// --- Token Usage ---

#[tauri::command]
pub fn get_conversation_usage(state: tauri::State<AppState>, conversation_id: String) -> Result<TokenUsageSummary, String> {
    state.db.lock().unwrap().get_conversation_token_usage(&conversation_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_total_usage(state: tauri::State<AppState>) -> Result<TokenUsageSummary, String> {
    state.db.lock().unwrap().get_total_token_usage().map_err(|e| e.to_string())
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

// --- Update Settings ---

#[tauri::command]
pub fn get_update_interval(state: tauri::State<AppState>) -> Result<String, String> {
    state.db.lock().unwrap()
        .get_setting("update_interval")
        .map_err(|e| e.to_string())
        .map(|v| v.unwrap_or_else(|| "86400000".to_string()))
}

#[tauri::command]
pub fn set_update_interval(state: tauri::State<AppState>, interval: String) -> Result<(), String> {
    state.db.lock().unwrap().set_setting("update_interval", &interval).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_skipped_version(state: tauri::State<AppState>) -> Result<String, String> {
    state.db.lock().unwrap()
        .get_setting("skipped_version")
        .map_err(|e| e.to_string())
        .map(|v| v.unwrap_or_default())
}

#[tauri::command]
pub fn set_skipped_version(state: tauri::State<AppState>, version: String) -> Result<(), String> {
    if version.is_empty() {
        state.db.lock().unwrap().remove_setting("skipped_version").map_err(|e| e.to_string())
    } else {
        state.db.lock().unwrap().set_setting("skipped_version", &version).map_err(|e| e.to_string())
    }
}

// --- Database Backup & Restore ---

#[tauri::command]
pub fn backup_database(state: tauri::State<AppState>, destination: String) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    let db_path = db.get_db_path();
    if db_path.is_empty() {
        return Err("Cannot determine database path".into());
    }
    // Checkpoint WAL to ensure all data is flushed
    db.conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").map_err(|e| e.to_string())?;
    std::fs::copy(&db_path, &destination).map_err(|e| format!("Backup failed: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn restore_database(state: tauri::State<AppState>, source: String) -> Result<(), String> {
    // Validate the source file is a valid SQLite database
    let test_conn = rusqlite::Connection::open(&source)
        .map_err(|e| format!("Invalid database file: {}", e))?;
    test_conn.query_row("SELECT COUNT(*) FROM conversations", [], |_| Ok(()))
        .map_err(|_| "Invalid LCD database: missing conversations table".to_string())?;
    drop(test_conn);

    let db = state.db.lock().unwrap();
    let db_path = db.get_db_path();
    if db_path.is_empty() {
        return Err("Cannot determine database path".into());
    }
    // Copy source over current database
    std::fs::copy(&source, &db_path).map_err(|e| format!("Restore failed: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn get_database_path(state: tauri::State<AppState>) -> Result<String, String> {
    let db = state.db.lock().unwrap();
    Ok(db.get_db_path())
}

#[tauri::command]
pub fn get_database_size(state: tauri::State<AppState>) -> Result<u64, String> {
    let db = state.db.lock().unwrap();
    let db_path = db.get_db_path();
    if db_path.is_empty() {
        return Ok(0);
    }
    let metadata = std::fs::metadata(&db_path).map_err(|e| e.to_string())?;
    Ok(metadata.len())
}

// --- Artifacts ---

#[tauri::command]
pub fn create_artifact(
    state: tauri::State<AppState>,
    conversation_id: String,
    title: String,
    artifact_type: String,
    language: Option<String>,
    content: String,
    source: String,
    message_id: Option<String>,
) -> Result<String, String> {
    let db = state.db.lock().unwrap();
    let artifact_id = uuid::Uuid::new_v4().to_string();
    let version_id = uuid::Uuid::new_v4().to_string();
    db.insert_artifact(&artifact_id, &conversation_id, &title, &artifact_type, language.as_deref())
        .map_err(|e| e.to_string())?;
    db.insert_artifact_version(&version_id, &artifact_id, 1, &content, &source, message_id.as_deref())
        .map_err(|e| e.to_string())?;
    Ok(artifact_id)
}

#[tauri::command]
pub fn get_artifacts(state: tauri::State<AppState>, conversation_id: String) -> Result<Vec<Artifact>, String> {
    state.db.lock().unwrap().list_artifacts_for_conversation(&conversation_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_artifact_content(state: tauri::State<AppState>, artifact_id: String) -> Result<Option<String>, String> {
    state.db.lock().unwrap().get_artifact_latest_content(&artifact_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_artifact_versions(state: tauri::State<AppState>, artifact_id: String) -> Result<Vec<ArtifactVersion>, String> {
    state.db.lock().unwrap().list_artifact_versions(&artifact_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_artifact_version(
    state: tauri::State<AppState>,
    artifact_id: String,
    content: String,
    source: String,
) -> Result<i64, String> {
    let db = state.db.lock().unwrap();
    // Get current version to determine next
    let versions = db.list_artifact_versions(&artifact_id).map_err(|e| e.to_string())?;
    let next_version = versions.iter().map(|v| v.version).max().unwrap_or(0) + 1;
    let version_id = uuid::Uuid::new_v4().to_string();
    db.insert_artifact_version(&version_id, &artifact_id, next_version, &content, &source, None)
        .map_err(|e| e.to_string())?;
    Ok(next_version)
}

#[tauri::command]
pub fn update_artifact_title(state: tauri::State<AppState>, artifact_id: String, title: String) -> Result<(), String> {
    state.db.lock().unwrap().update_artifact_title(&artifact_id, &title).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_artifact(state: tauri::State<AppState>, artifact_id: String) -> Result<(), String> {
    state.db.lock().unwrap().delete_artifact(&artifact_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_artifact_to_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, &content).map_err(|e| format!("Failed to write file: {}", e))
}

#[tauri::command]
pub fn open_artifact_external(content: String, language: String) -> Result<(), String> {
    let ext = match language.as_str() {
        "javascript" | "js" => "js",
        "typescript" | "ts" => "ts",
        "python" | "py" => "py",
        "rust" | "rs" => "rs",
        "html" => "html",
        "css" => "css",
        "json" => "json",
        "markdown" | "md" => "md",
        "svg" => "svg",
        "mermaid" => "mmd",
        _ => "txt",
    };
    let path = format!("/tmp/ucd-artifact-{}.{}", uuid::Uuid::new_v4(), ext);
    std::fs::write(&path, &content).map_err(|e| format!("Failed to write temp file: {}", e))?;
    std::process::Command::new("xdg-open")
        .arg(&path)
        .spawn()
        .map_err(|e| format!("Failed to open external editor: {}", e))?;
    Ok(())
}

// --- Conversation Memory ---

#[tauri::command]
pub fn get_memory_entries(state: tauri::State<AppState>) -> Result<Vec<MemoryEntry>, String> {
    state.db.lock().unwrap().list_memory_entries().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_memory_entry(state: tauri::State<AppState>, key: String, value: String) -> Result<String, String> {
    state.db.lock().unwrap().upsert_memory_entry(&key, &value).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_memory_entry(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    state.db.lock().unwrap().delete_memory_entry(&id).map_err(|e| e.to_string())
}

// --- Custom Endpoints ---

#[tauri::command]
pub fn get_custom_endpoints(state: tauri::State<AppState>) -> Result<Vec<CustomEndpoint>, String> {
    state.db.lock().unwrap().list_custom_endpoints().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_custom_endpoint(
    state: tauri::State<AppState>, name: String, base_url: String,
    api_key: String, api_format: String, default_model: String,
) -> Result<String, String> {
    let db = state.db.lock().unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    db.conn.execute(
        "INSERT INTO custom_endpoints (id, name, base_url, api_key, api_format, default_model, is_enabled, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?7)",
        params![id, name, base_url, api_key, api_format, default_model, now],
    ).map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub fn update_custom_endpoint(
    state: tauri::State<AppState>, id: String, name: String, base_url: String,
    api_key: String, api_format: String, default_model: String, is_enabled: bool,
) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    let now = chrono::Utc::now().to_rfc3339();
    db.conn.execute(
        "UPDATE custom_endpoints SET name=?1, base_url=?2, api_key=?3, api_format=?4, default_model=?5, is_enabled=?6, updated_at=?7 WHERE id=?8",
        params![name, base_url, api_key, api_format, default_model, is_enabled, now, id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_custom_endpoint(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    state.db.lock().unwrap().conn.execute("DELETE FROM custom_endpoints WHERE id=?1", params![id]).map_err(|e| e.to_string())?;
    Ok(())
}

// --- Model Pricing ---

#[tauri::command]
pub fn get_model_pricing(state: tauri::State<AppState>) -> Result<Vec<ModelPricing>, String> {
    state.db.lock().unwrap().list_model_pricing().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_model_pricing(
    state: tauri::State<AppState>, model_pattern: String,
    input_cost: f64, output_cost: f64, provider: String,
) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    let now = chrono::Utc::now().to_rfc3339();
    let id = uuid::Uuid::new_v4().to_string();
    db.conn.execute(
        "INSERT OR REPLACE INTO model_pricing (id, model_pattern, input_cost_per_mtok, output_cost_per_mtok, provider, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, model_pattern, input_cost, output_cost, provider, now],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_model_pricing(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    state.db.lock().unwrap().conn.execute("DELETE FROM model_pricing WHERE id=?1", params![id]).map_err(|e| e.to_string())?;
    Ok(())
}

// --- Routing Rules ---

#[tauri::command]
pub fn get_routing_rules(state: tauri::State<AppState>) -> Result<Vec<RoutingRule>, String> {
    state.db.lock().unwrap().list_routing_rules().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_routing_rule(
    state: tauri::State<AppState>, name: String, pattern: String,
    task_type: String, target_provider: String, target_model: String, priority: i32,
) -> Result<String, String> {
    let db = state.db.lock().unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    db.conn.execute(
        "INSERT INTO routing_rules (id, name, pattern, task_type, target_provider, target_model, priority, is_enabled, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8)",
        params![id, name, pattern, task_type, target_provider, target_model, priority, now],
    ).map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub fn update_routing_rule(
    state: tauri::State<AppState>, id: String, name: String, pattern: String,
    task_type: String, target_provider: String, target_model: String, priority: i32, is_enabled: bool,
) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    db.conn.execute(
        "UPDATE routing_rules SET name=?1, pattern=?2, task_type=?3, target_provider=?4, target_model=?5, priority=?6, is_enabled=?7 WHERE id=?8",
        params![name, pattern, task_type, target_provider, target_model, priority, is_enabled, id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_routing_rule(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    state.db.lock().unwrap().conn.execute("DELETE FROM routing_rules WHERE id=?1", params![id]).map_err(|e| e.to_string())?;
    Ok(())
}

// --- Comparison ---

#[tauri::command]
pub fn get_comparison_sessions(state: tauri::State<AppState>) -> Result<Vec<ComparisonSession>, String> {
    state.db.lock().unwrap().list_comparison_sessions().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_comparison_responses(state: tauri::State<AppState>, session_id: String) -> Result<Vec<ComparisonResponse>, String> {
    state.db.lock().unwrap().list_comparison_responses(&session_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rate_comparison_response(state: tauri::State<AppState>, response_id: String, rating: i32) -> Result<(), String> {
    state.db.lock().unwrap().conn.execute(
        "UPDATE comparison_responses SET rating=?1 WHERE id=?2", params![rating, response_id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_comparison_session(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    db.conn.execute("DELETE FROM comparison_responses WHERE session_id=?1", params![id]).map_err(|e| e.to_string())?;
    db.conn.execute("DELETE FROM comparison_sessions WHERE id=?1", params![id]).map_err(|e| e.to_string())?;
    Ok(())
}

// --- Cost Queries ---

#[tauri::command]
pub fn get_conversation_cost(state: tauri::State<AppState>, conversation_id: String) -> Result<f64, String> {
    let db = state.db.lock().unwrap();
    let cost: f64 = db.conn.query_row(
        "SELECT COALESCE(SUM(estimated_cost), 0.0) FROM token_usage WHERE conversation_id=?1",
        params![conversation_id], |row| row.get(0),
    ).map_err(|e| e.to_string())?;
    Ok(cost)
}

#[tauri::command]
pub fn get_cost_summary(state: tauri::State<AppState>) -> Result<Vec<(String, f64, i64, i64)>, String> {
    let db = state.db.lock().unwrap();
    let mut stmt = db.conn.prepare(
        "SELECT model, COALESCE(SUM(estimated_cost), 0.0), COALESCE(SUM(input_tokens), 0), COALESCE(SUM(output_tokens), 0) FROM token_usage GROUP BY model ORDER BY SUM(estimated_cost) DESC"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?, row.get::<_, i64>(2)?, row.get::<_, i64>(3)?))
    }).map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }
    Ok(result)
}

// --- Knowledge Base ---

#[tauri::command]
pub fn get_knowledge_entries(state: tauri::State<AppState>, project_id: Option<String>) -> Result<Vec<KnowledgeEntry>, String> {
    state.db.lock().unwrap().list_knowledge_entries(project_id.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_knowledge_entry(
    state: tauri::State<AppState>,
    title: String,
    content: String,
    source_type: String,
    source_url: Option<String>,
    file_path: Option<String>,
    project_id: Option<String>,
) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    state.db.lock().unwrap().insert_knowledge_entry(
        &id, &title, &content, &source_type,
        source_url.as_deref(), file_path.as_deref(), project_id.as_deref(),
    ).map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub fn update_knowledge_entry(state: tauri::State<AppState>, id: String, title: String, content: String, enabled: bool) -> Result<(), String> {
    state.db.lock().unwrap().update_knowledge_entry(&id, &title, &content, enabled).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_knowledge_entry(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    state.db.lock().unwrap().delete_knowledge_entry(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn toggle_knowledge_entry(state: tauri::State<AppState>, id: String, enabled: bool) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    let now = chrono::Utc::now().to_rfc3339();
    db.conn.execute(
        "UPDATE knowledge_base SET enabled = ?1, updated_at = ?2 WHERE id = ?3",
        params![if enabled { 1 } else { 0 }, &now, &id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn import_file_to_knowledge(state: tauri::State<AppState>, path: String, project_id: Option<String>, watch: bool) -> Result<String, String> {
    let file_path = std::path::Path::new(&path);
    let title = file_path.file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "Imported File".to_string());
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    // Limit to 50K chars
    let content = if content.len() > 50_000 { content[..50_000].to_string() } else { content };
    let id = uuid::Uuid::new_v4().to_string();
    let db = state.db.lock().unwrap();
    db.insert_knowledge_entry(
        &id, &title, &content, "file",
        None, Some(&path), project_id.as_deref(),
    ).map_err(|e| e.to_string())?;
    if watch {
        let watch_id = uuid::Uuid::new_v4().to_string();
        let modified = std::fs::metadata(&path).ok()
            .and_then(|m| m.modified().ok())
            .map(|t| {
                let dt: chrono::DateTime<chrono::Utc> = t.into();
                dt.to_rfc3339()
            });
        db.insert_file_watch(&watch_id, &path, &id, modified.as_deref())
            .map_err(|e| e.to_string())?;
    }
    Ok(id)
}

// --- File Watches ---

#[tauri::command]
pub fn get_file_watches(state: tauri::State<AppState>) -> Result<Vec<FileWatch>, String> {
    state.db.lock().unwrap().list_file_watches().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_file_watch(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    state.db.lock().unwrap().delete_file_watch(&id).map_err(|e| e.to_string())
}
