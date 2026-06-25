use rusqlite::{Connection, Result};
use std::path::Path;
use crate::error::AppError;
use std::time::Duration;

pub struct Database {
    pub conn: Connection,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.busy_timeout(Duration::from_secs(5))?;
        Ok(Self { conn })
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.busy_timeout(Duration::from_secs(5))?;
        Ok(Self { conn })
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn get_meta(&self, key: &str) -> Result<String, AppError> {
        let value: String = self.conn.query_row(
            "SELECT value FROM meta WHERE key = ?1",
            [key],
            |row| row.get(0),
        ).map_err(AppError::Database)?;
        Ok(value)
    }

    pub fn setup_schema(&self) -> Result<()> {
        let conn = &self.conn;

        // meta
        conn.execute(
            "CREATE TABLE IF NOT EXISTS meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;
        conn.execute(
            "INSERT OR IGNORE INTO meta (key, value) VALUES ('schema_version', '3')",
            [],
        )?;
        conn.execute(
            "INSERT OR IGNORE INTO meta (key, value) VALUES ('preset_seeded', '0')",
            [],
        )?;
        conn.execute(
            "INSERT OR IGNORE INTO meta (key, value) VALUES ('theme', 'auto')",
            [],
        )?;

        // categories
        conn.execute(
            "CREATE TABLE IF NOT EXISTS categories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                is_default INTEGER NOT NULL DEFAULT 0,
                sort_index INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )?;
        conn.execute(
            "INSERT OR IGNORE INTO categories (id, name, is_default, sort_index, created_at, updated_at)
             VALUES (1, '凭证', 1, 0, strftime('%s','now'), strftime('%s','now'))",
            [],
        )?;

        // providers
        conn.execute(
            "CREATE TABLE IF NOT EXISTS providers (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                preset TEXT,
                is_preset INTEGER NOT NULL DEFAULT 0,
                category_id INTEGER NOT NULL,
                pinned INTEGER NOT NULL DEFAULT 0,
                notes TEXT,
                icon TEXT,
                icon_color TEXT,
                sort_index INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                FOREIGN KEY (category_id) REFERENCES categories(id) ON DELETE RESTRICT
            )",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_providers_category ON providers(category_id, sort_index)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_providers_preset ON providers(preset)",
            [],
        )?;

        // provider_fields
        conn.execute(
            "CREATE TABLE IF NOT EXISTS provider_fields (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                provider_id INTEGER NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                visibility TEXT NOT NULL DEFAULT 'visible',
                sort_index INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                FOREIGN KEY (provider_id) REFERENCES providers(id) ON DELETE CASCADE
            )",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_pf_provider ON provider_fields(provider_id, sort_index)",
            [],
        )?;

        // quota_cache
        conn.execute(
            "CREATE TABLE IF NOT EXISTS quota_cache (
                provider_id INTEGER PRIMARY KEY,
                snapshot_json TEXT NOT NULL,
                fetched_at INTEGER NOT NULL,
                FOREIGN KEY (provider_id) REFERENCES providers(id) ON DELETE CASCADE
            )",
            [],
        )?;

        Ok(())
    }

    pub fn seed_preset_providers(&self) -> Result<()> {
        // Check if already seeded
        let seeded: String = self.conn.query_row(
            "SELECT value FROM meta WHERE key = 'preset_seeded'",
            [],
            |row| row.get(0),
        )?;
        if seeded == "1" {
            return Ok(());
        }

        let now = chrono::Utc::now().timestamp();

        // OpenAI: base_url + api_key
        self.conn.execute(
            "INSERT INTO providers (name, preset, is_preset, category_id, pinned, icon, icon_color, sort_index, created_at, updated_at)
             VALUES ('OpenAI', 'openai', 1, 1, 1, '🤖', '#10a37f', 0, ?1, ?1)",
            [now],
        )?;
        let openai_id: i64 = self.conn.last_insert_rowid();
        self.add_field(openai_id, "base_url", "https://api.openai.com/v1", "visible", 0, now)?;
        self.add_field(openai_id, "api_key", "", "masked", 1, now)?;

        // DeepSeek: base_url + api_key
        self.conn.execute(
            "INSERT INTO providers (name, preset, is_preset, category_id, pinned, icon, icon_color, sort_index, created_at, updated_at)
             VALUES ('DeepSeek', 'deepseek', 1, 1, 1, '🔍', '#0066cc', 1, ?1, ?1)",
            [now],
        )?;
        let deepseek_id: i64 = self.conn.last_insert_rowid();
        self.add_field(deepseek_id, "base_url", "https://api.deepseek.com/v1", "visible", 0, now)?;
        self.add_field(deepseek_id, "api_key", "", "masked", 1, now)?;

        // Anthropic: base_url + api_key
        self.conn.execute(
            "INSERT INTO providers (name, preset, is_preset, category_id, pinned, icon, icon_color, sort_index, created_at, updated_at)
             VALUES ('Anthropic', 'anthropic', 1, 1, 1, '🧠', '#d91666', 2, ?1, ?1)",
            [now],
        )?;
        let anthropic_id: i64 = self.conn.last_insert_rowid();
        self.add_field(anthropic_id, "base_url", "https://api.anthropic.com", "visible", 0, now)?;
        self.add_field(anthropic_id, "api_key", "", "masked", 1, now)?;

        // GitHub: base_url + api_key
        self.conn.execute(
            "INSERT INTO providers (name, preset, is_preset, category_id, pinned, icon, icon_color, sort_index, created_at, updated_at)
             VALUES ('GitHub', 'github', 1, 1, 1, '🐙', '#24292e', 3, ?1, ?1)",
            [now],
        )?;
        let github_id: i64 = self.conn.last_insert_rowid();
        self.add_field(github_id, "base_url", "https://api.github.com", "visible", 0, now)?;
        self.add_field(github_id, "api_key", "", "masked", 1, now)?;

        // PostgreSQL: host + port + database + user + password
        self.conn.execute(
            "INSERT INTO providers (name, preset, is_preset, category_id, pinned, icon, icon_color, sort_index, created_at, updated_at)
             VALUES ('PostgreSQL', 'postgres', 1, 1, 0, '🗄️', '#336791', 4, ?1, ?1)",
            [now],
        )?;
        let postgres_id: i64 = self.conn.last_insert_rowid();
        self.add_field(postgres_id, "host", "localhost", "visible", 0, now)?;
        self.add_field(postgres_id, "port", "5432", "visible", 1, now)?;
        self.add_field(postgres_id, "database", "", "visible", 2, now)?;
        self.add_field(postgres_id, "user", "", "visible", 3, now)?;
        self.add_field(postgres_id, "password", "", "masked", 4, now)?;

        // Mark as seeded
        self.conn.execute(
            "UPDATE meta SET value = '1' WHERE key = 'preset_seeded'",
            [],
        )?;

        Ok(())
    }

    fn add_field(
        &self,
        provider_id: i64,
        key: &str,
        value: &str,
        visibility: &str,
        sort_index: i64,
        now: i64,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO provider_fields (provider_id, key, value, visibility, sort_index, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            rusqlite::params![provider_id, key, value, visibility, sort_index, now],
        )?;
        Ok(())
    }
}
