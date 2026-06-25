use rusqlite::{Connection, Result};
use std::path::Path;
use crate::error::AppError;
use std::time::Duration;
use crate::types::{TokenUsageRecord, DailyAgentModelUsage, DailyModelUsage};

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

    pub fn migrate(&self) -> Result<(), AppError> {
        let current: String = self.conn.query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )?;
        if current == "3" {
            let sql = include_str!("../data/migrations/v3_to_v4.sql");
            self.conn.execute_batch(sql)?;
            self.conn.execute(
                "UPDATE meta SET value = '4' WHERE key = 'schema_version'",
                [],
            )?;
        }
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

    pub fn insert_token_usage(&self, record: &TokenUsageRecord) -> Result<(), AppError> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO token_usage_records (id, agent_type, model, provider_name, occurred_at, recorded_at,
             session_id, request_id, prompt_tokens, completion_tokens, total_tokens,
             cache_read_input_tokens, cache_creation_input_tokens, reasoning_tokens,
             input_tokens, output_tokens, prompt_cost, completion_cost, cache_read_cost,
             cache_creation_cost, reasoning_cost, total_cost, currency, pricing_version, usage_details, cost_details)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26)",
            rusqlite::params![
                record.id, record.agent_type, record.model, record.provider_name, record.occurred_at, record.recorded_at,
                record.session_id, record.request_id, record.input_tokens, record.output_tokens, record.total_tokens,
                record.cache_read_input_tokens, record.cache_creation_input_tokens, record.reasoning_tokens,
                record.input_tokens, record.output_tokens, record.prompt_cost, record.completion_cost, record.cache_read_cost,
                record.cache_creation_cost, record.reasoning_cost, record.total_cost, record.currency, record.pricing_version,
                record.usage_details, record.cost_details,
            ],
        )?;
        let day = chrono::DateTime::from_timestamp(record.occurred_at, 0)
            .unwrap_or_default()
            .format("%Y-%m-%d")
            .to_string();
        let day_copy = day.clone();
        tx.execute(
            "INSERT OR REPLACE INTO daily_agent_model_usage
             (date, agent_type, model, provider, request_count, input_tokens, output_tokens, total_tokens, total_cost)
             VALUES (?1, ?2, ?3, ?4,
              COALESCE((SELECT request_count FROM daily_agent_model_usage WHERE date=?1 AND agent_type=?2 AND model=?3 AND provider=?4), 0) + 1,
              COALESCE((SELECT input_tokens FROM daily_agent_model_usage WHERE date=?1 AND agent_type=?2 AND model=?3 AND provider=?4), 0) + ?5,
              COALESCE((SELECT output_tokens FROM daily_agent_model_usage WHERE date=?1 AND agent_type=?2 AND model=?3 AND provider=?4), 0) + ?6,
              COALESCE((SELECT total_tokens FROM daily_agent_model_usage WHERE date=?1 AND agent_type=?2 AND model=?3 AND provider=?4), 0) + ?7,
              COALESCE((SELECT total_cost FROM daily_agent_model_usage WHERE date=?1 AND agent_type=?2 AND model=?3 AND provider=?4), 0) + ?8)",
            rusqlite::params![day_copy, record.agent_type, record.model, record.provider_name,
                record.input_tokens, record.output_tokens, record.total_tokens, record.total_cost],
        )?;
        let day2 = day.clone();
        tx.execute(
            "INSERT OR REPLACE INTO daily_model_usage
             (date, model, provider, request_count, input_tokens, output_tokens, total_tokens, total_cost)
             VALUES (?1, ?2, ?3,
              COALESCE((SELECT request_count FROM daily_model_usage WHERE date=?1 AND model=?2 AND provider=?3), 0) + 1,
              COALESCE((SELECT input_tokens FROM daily_model_usage WHERE date=?1 AND model=?2 AND provider=?3), 0) + ?4,
              COALESCE((SELECT output_tokens FROM daily_model_usage WHERE date=?1 AND model=?2 AND provider=?3), 0) + ?5,
              COALESCE((SELECT total_tokens FROM daily_model_usage WHERE date=?1 AND model=?2 AND provider=?3), 0) + ?6,
              COALESCE((SELECT total_cost FROM daily_model_usage WHERE date=?1 AND model=?2 AND provider=?3), 0) + ?7)",
            rusqlite::params![day2, record.model, record.provider_name,
                record.input_tokens, record.output_tokens, record.total_tokens, record.total_cost],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn list_token_usage_records(&self, offset: i64, limit: i64) -> Result<Vec<TokenUsageRecord>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, agent_type, model, provider_name, occurred_at, recorded_at, session_id, request_id,
             total_tokens, cache_read_input_tokens, cache_creation_input_tokens, reasoning_tokens,
             input_tokens, output_tokens, prompt_cost, completion_cost, cache_read_cost, cache_creation_cost, reasoning_cost,
             total_cost, currency, pricing_version, usage_details, cost_details
             FROM token_usage_records ORDER BY occurred_at DESC LIMIT ?1 OFFSET ?2"
        )?;
        let records = stmt.query_map([limit, offset], |row| {
            Ok(TokenUsageRecord {
                id: row.get(0)?,
                agent_type: row.get(1)?,
                model: row.get(2)?,
                provider_name: row.get(3)?,
                occurred_at: row.get(4)?,
                recorded_at: row.get(5)?,
                session_id: row.get(6)?,
                request_id: row.get(7)?,
                total_tokens: row.get(8)?,
                cache_read_input_tokens: row.get(9)?,
                cache_creation_input_tokens: row.get(10)?,
                reasoning_tokens: row.get(11)?,
                input_tokens: row.get(12)?,
                output_tokens: row.get(13)?,
                prompt_cost: row.get(14)?,
                completion_cost: row.get(15)?,
                cache_read_cost: row.get(16)?,
                cache_creation_cost: row.get(17)?,
                reasoning_cost: row.get(18)?,
                total_cost: row.get(19)?,
                currency: row.get(20)?,
                pricing_version: row.get(21)?,
                usage_details: row.get(22)?,
                cost_details: row.get(23)?,
            })
        })?.collect::<Result<Vec<_>, _>>().map_err(AppError::Database)?;
        Ok(records)
    }

    pub fn get_daily_usage_summary(&self, date: &str) -> Result<Vec<DailyAgentModelUsage>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT date, agent_type, model, provider, request_count, input_tokens, output_tokens, total_tokens, total_cost
             FROM daily_agent_model_usage WHERE date = ?1 ORDER BY total_tokens DESC"
        )?;
        let rows = stmt.query_map([date], |row| {
            Ok(DailyAgentModelUsage {
                date: row.get(0)?,
                agent_type: row.get(1)?,
                model: row.get(2)?,
                provider: row.get(3)?,
                request_count: row.get(4)?,
                input_tokens: row.get(5)?,
                output_tokens: row.get(6)?,
                total_tokens: row.get(7)?,
                total_cost: row.get(8)?,
            })
        })?.collect::<Result<Vec<_>, _>>().map_err(AppError::Database)?;
        Ok(rows)
    }

    pub fn get_model_usage_summary(&self, date: &str) -> Result<Vec<DailyModelUsage>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT date, model, provider, request_count, input_tokens, output_tokens, total_tokens, total_cost
             FROM daily_model_usage WHERE date = ?1 ORDER BY total_tokens DESC"
        )?;
        let rows = stmt.query_map([date], |row| {
            Ok(DailyModelUsage {
                date: row.get(0)?,
                model: row.get(1)?,
                provider: row.get(2)?,
                request_count: row.get(3)?,
                input_tokens: row.get(4)?,
                output_tokens: row.get(5)?,
                total_tokens: row.get(6)?,
                total_cost: row.get(7)?,
            })
        })?.collect::<Result<Vec<_>, _>>().map_err(AppError::Database)?;
        Ok(rows)
    }
}
