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
        self.conn.execute(
            "INSERT INTO token_usage (conversation_id, message_id, input_tokens, output_tokens, model, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![conversation_id, message_id, input_tokens, output_tokens, model, &now],
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
