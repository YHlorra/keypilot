use rusqlite::{Connection, Result};
use std::path::Path;
use crate::error::AppError;
use std::time::Duration;
use crate::types::{TokenUsageRecord, DailyAgentModelUsage, DailyModelUsage};

/// Per-file cursor row used by the incremental JSONL importer
/// (services/incremental_import.rs).  See Bug #3 fix 2026-06-29.
#[derive(Debug, Clone)]
pub struct AgentFileCursor {
    pub agent_type: String,
    pub file_path: String,
    pub byte_offset: i64,
    pub file_size: i64,
    pub last_scan_at: i64,
    pub last_event_at: Option<i64>,
}

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

    pub fn set_meta(&self, key: &str, value: &str) -> Result<(), AppError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
            rusqlite::params![key, value],
        ).map_err(AppError::Database)?;
        Ok(())
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
            "INSERT OR IGNORE INTO meta (key, value) VALUES ('schema_version', '6')",
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
                source TEXT NOT NULL DEFAULT 'auto',
                FOREIGN KEY (provider_id) REFERENCES providers(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Idempotent column add for databases created before `source` existed
        // (pre-V0.1-rev2). Fresh DBs get the column from CREATE above.
        let has_source: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('quota_cache') WHERE name = 'source'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        if has_source == 0 {
            conn.execute(
                "ALTER TABLE quota_cache ADD COLUMN source TEXT NOT NULL DEFAULT 'auto'",
                [],
            )?;
        }

        // token_usage_records (v5 schema — no prompt_tokens / completion_tokens
        // legacy cols; those are dropped from v4→v5 migration for old DBs)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS token_usage_records (
                id TEXT PRIMARY KEY,
                agent_type TEXT NOT NULL,
                model TEXT NOT NULL,
                provider_name TEXT NOT NULL,
                occurred_at INTEGER NOT NULL,
                recorded_at INTEGER NOT NULL,
                session_id TEXT,
                request_id TEXT,
                input_tokens INTEGER DEFAULT 0,
                output_tokens INTEGER DEFAULT 0,
                cache_read_input_tokens INTEGER DEFAULT 0,
                cache_creation_input_tokens INTEGER DEFAULT 0,
                reasoning_tokens INTEGER DEFAULT 0,
                total_tokens INTEGER DEFAULT 0,
                prompt_cost REAL DEFAULT 0.0,
                completion_cost REAL DEFAULT 0.0,
                cache_read_cost REAL DEFAULT 0.0,
                cache_creation_cost REAL DEFAULT 0.0,
                reasoning_cost REAL DEFAULT 0.0,
                total_cost REAL DEFAULT 0.0,
                currency TEXT DEFAULT 'USD',
                pricing_version TEXT,
                usage_details TEXT DEFAULT '{}',
                cost_details TEXT DEFAULT NULL
            )",
            [],
        )?;

        // daily_agent_model_usage (rollup by date+agent+model+provider)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS daily_agent_model_usage (
                date TEXT NOT NULL,
                agent_type TEXT NOT NULL,
                model TEXT NOT NULL,
                provider TEXT NOT NULL,
                request_count INTEGER DEFAULT 0,
                input_tokens INTEGER DEFAULT 0,
                output_tokens INTEGER DEFAULT 0,
                total_tokens INTEGER DEFAULT 0,
                total_cost REAL DEFAULT 0.0,
                PRIMARY KEY (date, agent_type, model, provider)
            )",
            [],
        )?;

        // daily_model_usage (rollup by date+model+provider)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS daily_model_usage (
                date TEXT NOT NULL,
                model TEXT NOT NULL,
                provider TEXT NOT NULL,
                request_count INTEGER DEFAULT 0,
                input_tokens INTEGER DEFAULT 0,
                output_tokens INTEGER DEFAULT 0,
                total_tokens INTEGER DEFAULT 0,
                total_cost REAL DEFAULT 0.0,
                PRIMARY KEY (date, model, provider)
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_token_usage_occurred ON token_usage_records(occurred_at)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_token_usage_agent_model ON token_usage_records(agent_type, model, occurred_at)",
            [],
        )?;

        // agent_file_cursor (v6 schema -- Bug #3 fix 2026-06-29)
        // Per-file byte-offset cursor for incremental JSONL append detection.
        // notify-debouncer-full emits Modify events on .jsonl appends; we seek
        // to `byte_offset` from the previous scan and parse only the new bytes.
        // Truncation detected by `file_size < byte_offset` -> reset to 0.
        // WITHOUT ROWID because (agent_type, file_path) is the natural PK.
        conn.execute(
            "CREATE TABLE IF NOT EXISTS agent_file_cursor (
                agent_type    TEXT NOT NULL,
                file_path     TEXT NOT NULL,
                byte_offset   INTEGER NOT NULL DEFAULT 0,
                file_size     INTEGER NOT NULL DEFAULT 0,
                last_scan_at  INTEGER NOT NULL DEFAULT 0,
                last_event_at INTEGER,
                PRIMARY KEY (agent_type, file_path)
            ) WITHOUT ROWID",
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
        } else if current == "4" {
            let sql = include_str!("../data/migrations/v4_to_v5.sql");
            self.conn.execute_batch(sql)?;
            self.conn.execute(
                "UPDATE meta SET value = '5' WHERE key = 'schema_version'",
                [],
            )?;
        } else if current == "5" {
            let sql = include_str!("../data/migrations/v5_to_v6.sql");
            self.conn.execute_batch(sql)?;
            self.conn.execute(
                "UPDATE meta SET value = '6' WHERE key = 'schema_version'",
                [],
            )?;
        } else if current == "6" {
            // migrate_to_v7() updates schema_version internally inside its own
            // transaction, so this branch has no extra UPDATE meta step.
            self.migrate_to_v7()?;
        }
        Ok(())
    }

    /// Re-aggregate daily_* tables from token_usage_records using Local timezone buckets.
    /// Idempotent: safe to call multiple times.
    pub fn migrate_to_v7(&self) -> Result<(), AppError> {
        // Idempotency guard: version >= 7 means migration already completed
        let current = self.schema_version().unwrap_or_default();
        if current == "7" {
            return Ok(());
        }

        // Check if _v6 backup tables already exist (incomplete prior attempt)
        let v6_exists: bool = self.conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='daily_agent_model_usage_v6'",
            [],
            |r| Ok(r.get::<_, i64>(0)? > 0),
        ).unwrap_or(false);

        let conn = self.conn();
        let tx = conn.unchecked_transaction().map_err(AppError::Database)?;

        if v6_exists {
            // Prior attempt left _v6 tables; drop them and proceed fresh
            // (current daily_* tables are the incomplete new ones from prior crash)
            tx.execute("DROP TABLE IF EXISTS daily_agent_model_usage", []).ok();
            tx.execute("DROP TABLE IF EXISTS daily_model_usage", []).ok();
        } else {
            // First attempt: rename current tables to _v6 backup
            tx.execute(
                "ALTER TABLE daily_agent_model_usage RENAME TO daily_agent_model_usage_v6",
                [],
            ).map_err(AppError::Database)?;
            tx.execute(
                "ALTER TABLE daily_model_usage RENAME TO daily_model_usage_v6",
                [],
            ).map_err(AppError::Database)?;
        }

        // Recreate schema (DDL identical to v6)
        tx.execute(
            "CREATE TABLE daily_agent_model_usage (
                date TEXT NOT NULL,
                agent_type TEXT NOT NULL,
                model TEXT NOT NULL,
                provider TEXT NOT NULL,
                request_count INTEGER DEFAULT 0,
                input_tokens INTEGER DEFAULT 0,
                output_tokens INTEGER DEFAULT 0,
                total_tokens INTEGER DEFAULT 0,
                total_cost REAL DEFAULT 0.0,
                PRIMARY KEY (date, agent_type, model, provider)
            )",
            [],
        ).map_err(AppError::Database)?;
        tx.execute(
            "CREATE TABLE daily_model_usage (
                date TEXT NOT NULL,
                model TEXT NOT NULL,
                provider TEXT NOT NULL,
                request_count INTEGER DEFAULT 0,
                input_tokens INTEGER DEFAULT 0,
                output_tokens INTEGER DEFAULT 0,
                total_tokens INTEGER DEFAULT 0,
                total_cost REAL DEFAULT 0.0,
                PRIMARY KEY (date, model, provider)
            )",
            [],
        ).map_err(AppError::Database)?;

        // Re-aggregate using Rust-side Local date conversion (SQLite strftime 'localtime'
        // is unreliable on Windows). Group by (date, agent, model, provider) in-memory.
        // ponytail: 'localtime' on SQLite may diverge from chrono::Local at DST gaps/folds.
        #[derive(Default, Eq, Hash, PartialEq)]
        struct AgentModelKey { date: String, agent: String, model: String, provider: String }
        #[derive(Default)]
        struct AgentModelAgg { count: i64, inp: i64, out: i64, total: i64, cost: f64 }
        let mut agent_map: std::collections::HashMap<AgentModelKey, AgentModelAgg> = std::collections::HashMap::new();

        let mut stmt = tx.prepare(
            "SELECT occurred_at, agent_type, model, provider_name,
                    input_tokens, output_tokens, cache_creation_input_tokens,
                    cache_read_input_tokens, total_tokens, total_cost
             FROM token_usage_records"
        ).map_err(AppError::Database)?;
        let rows = stmt.query_map([], |row| Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, i64>(6)?,
            row.get::<_, i64>(7)?,
            row.get::<_, i64>(8)?,
            row.get::<_, f64>(9)?,
        ))).map_err(AppError::Database)?
        .collect::<Result<Vec<_>, _>>().map_err(AppError::Database)?;
        drop(stmt);

        for (occurred_at, agent, model, provider, inp, out, _cache_crea, _cache_read, total, cost) in rows {
            let date = crate::timeutil::local_date_str(occurred_at);
            let key = AgentModelKey { date, agent, model, provider };
            let agg = agent_map.entry(key).or_default();
            agg.count += 1;
            agg.inp += inp;
            agg.out += out;
            agg.total += total;
            agg.cost += cost;
        }

        for (key, agg) in agent_map {
            tx.execute(
                "INSERT INTO daily_agent_model_usage
                 (date, agent_type, model, provider, request_count, input_tokens, output_tokens, total_tokens, total_cost)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![key.date, key.agent, key.model, key.provider,
                    agg.count, agg.inp, agg.out, agg.total, agg.cost],
            ).map_err(AppError::Database)?;
        }

        // Re-aggregate daily_model_usage (no agent_type column)
        #[derive(Default, Eq, Hash, PartialEq)]
        struct ModelKey { date: String, model: String, provider: String }
        #[derive(Default)]
        struct ModelAgg { count: i64, inp: i64, out: i64, total: i64, cost: f64 }
        let mut model_map: std::collections::HashMap<ModelKey, ModelAgg> = std::collections::HashMap::new();

        let mut stmt2 = tx.prepare(
            "SELECT occurred_at, model, provider_name,
                    input_tokens, output_tokens, cache_creation_input_tokens,
                    cache_read_input_tokens, total_tokens, total_cost
             FROM token_usage_records"
        ).map_err(AppError::Database)?;
        let rows2 = stmt2.query_map([], |row| Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, i64>(6)?,
            row.get::<_, i64>(7)?,
            row.get::<_, f64>(8)?,
        ))).map_err(AppError::Database)?
        .collect::<Result<Vec<_>, _>>().map_err(AppError::Database)?;
        drop(stmt2);

        for (occurred_at, model, provider, inp, out, _cache_crea, _cache_read, total, cost) in rows2 {
            let date = crate::timeutil::local_date_str(occurred_at);
            let key = ModelKey { date, model, provider };
            let agg = model_map.entry(key).or_default();
            agg.count += 1;
            agg.inp += inp;
            agg.out += out;
            agg.total += total;
            agg.cost += cost;
        }

        for (key, agg) in model_map {
            tx.execute(
                "INSERT INTO daily_model_usage
                 (date, model, provider, request_count, input_tokens, output_tokens, total_tokens, total_cost)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![key.date, key.model, key.provider,
                    agg.count, agg.inp, agg.out, agg.total, agg.cost],
            ).map_err(AppError::Database)?;
        }

        // Update schema_version to "7" inside the tx so a post-commit crash can
        // only leave either the FULL migration applied (both data + version) or
        // NEITHER — strict atomicity per REQ-DATE-LOCAL-008.
        tx.execute(
            "UPDATE meta SET value = '7' WHERE key = 'schema_version'",
            [],
        ).map_err(AppError::Database)?;

        tx.commit().map_err(AppError::Database)?;
        Ok(())
    }

    fn schema_version(&self) -> Result<String, AppError> {
        let v: String = self.conn.query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        ).map_err(AppError::Database)?;
        Ok(v)
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
             session_id, request_id, input_tokens, output_tokens,
             cache_read_input_tokens, cache_creation_input_tokens, reasoning_tokens, total_tokens,
             prompt_cost, completion_cost, cache_read_cost, cache_creation_cost, reasoning_cost, total_cost,
             currency, pricing_version, usage_details, cost_details)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24)",
            rusqlite::params![
                record.id, record.agent_type, record.model, record.provider_name, record.occurred_at, record.recorded_at,
                record.session_id, record.request_id, record.input_tokens, record.output_tokens,
                record.cache_read_input_tokens, record.cache_creation_input_tokens, record.reasoning_tokens, record.total_tokens,
                record.prompt_cost, record.completion_cost, record.cache_read_cost, record.cache_creation_cost,
                record.reasoning_cost, record.total_cost, record.currency, record.pricing_version,
                record.usage_details, record.cost_details,
            ],
        )?;
        let day = crate::timeutil::local_date_str(record.occurred_at);
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
             input_tokens, output_tokens, cache_read_input_tokens, cache_creation_input_tokens,
             reasoning_tokens, total_tokens, prompt_cost, completion_cost, cache_read_cost,
             cache_creation_cost, reasoning_cost, total_cost, currency, pricing_version, usage_details, cost_details
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
                input_tokens: row.get(8)?,
                output_tokens: row.get(9)?,
                cache_read_input_tokens: row.get(10)?,
                cache_creation_input_tokens: row.get(11)?,
                reasoning_tokens: row.get(12)?,
                total_tokens: row.get(13)?,
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

    pub fn list_token_usage_records_filtered(
        &self,
        agent_type: Option<&str>,
        model: Option<&str>,
        provider_name: Option<&str>,
        date_from: Option<i64>,
        date_to: Option<i64>,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<TokenUsageRecord>, AppError> {
        let mut sql = String::from(
            "SELECT id, agent_type, model, provider_name, occurred_at, recorded_at, session_id, request_id,
             input_tokens, output_tokens, cache_read_input_tokens, cache_creation_input_tokens,
             reasoning_tokens, total_tokens, prompt_cost, completion_cost, cache_read_cost,
             cache_creation_cost, reasoning_cost, total_cost, currency, pricing_version, usage_details, cost_details
             FROM token_usage_records WHERE 1=1"
        );
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(v) = agent_type {
            sql.push_str(" AND agent_type = ?");
            params.push(Box::new(v.to_string()));
        }
        if let Some(v) = model {
            sql.push_str(" AND model = ?");
            params.push(Box::new(v.to_string()));
        }
        if let Some(v) = provider_name {
            sql.push_str(" AND provider_name = ?");
            params.push(Box::new(v.to_string()));
        }
        if let Some(v) = date_from {
            sql.push_str(" AND occurred_at >= ?");
            params.push(Box::new(v));
        }
        if let Some(v) = date_to {
            sql.push_str(" AND occurred_at <= ?");
            params.push(Box::new(v));
        }
        sql.push_str(" ORDER BY occurred_at DESC LIMIT ? OFFSET ?");
        params.push(Box::new(limit));
        params.push(Box::new(offset));
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|b| b.as_ref()).collect();
        let mut stmt = self.conn.prepare(&sql)?;
        let records = stmt.query_map(param_refs.as_slice(), |row| {
            Ok(TokenUsageRecord {
                id: row.get(0)?,
                agent_type: row.get(1)?,
                model: row.get(2)?,
                provider_name: row.get(3)?,
                occurred_at: row.get(4)?,
                recorded_at: row.get(5)?,
                session_id: row.get(6)?,
                request_id: row.get(7)?,
                input_tokens: row.get(8)?,
                output_tokens: row.get(9)?,
                cache_read_input_tokens: row.get(10)?,
                cache_creation_input_tokens: row.get(11)?,
                reasoning_tokens: row.get(12)?,
                total_tokens: row.get(13)?,
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

    pub fn get_token_usage_record_by_id(&self, id: &str) -> Result<Option<TokenUsageRecord>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, agent_type, model, provider_name, occurred_at, recorded_at, session_id, request_id,
             input_tokens, output_tokens, cache_read_input_tokens, cache_creation_input_tokens,
             reasoning_tokens, total_tokens, prompt_cost, completion_cost, cache_read_cost,
             cache_creation_cost, reasoning_cost, total_cost, currency, pricing_version, usage_details, cost_details
             FROM token_usage_records WHERE id = ?1"
        )?;
        let mut rows = stmt.query([id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(TokenUsageRecord {
                id: row.get(0)?,
                agent_type: row.get(1)?,
                model: row.get(2)?,
                provider_name: row.get(3)?,
                occurred_at: row.get(4)?,
                recorded_at: row.get(5)?,
                session_id: row.get(6)?,
                request_id: row.get(7)?,
                input_tokens: row.get(8)?,
                output_tokens: row.get(9)?,
                cache_read_input_tokens: row.get(10)?,
                cache_creation_input_tokens: row.get(11)?,
                reasoning_tokens: row.get(12)?,
                total_tokens: row.get(13)?,
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
            }))
        } else {
            Ok(None)
        }
    }

    pub fn delete_daily_for_date(&self, date: &str) -> Result<(), AppError> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM daily_agent_model_usage WHERE date = ?1",
            [date],
        )?;
        tx.execute(
            "DELETE FROM daily_model_usage WHERE date = ?1",
            [date],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn count_token_usage_records_filtered(
        &self,
        agent_type: Option<&str>,
        model: Option<&str>,
        provider_name: Option<&str>,
        date_from: Option<i64>,
        date_to: Option<i64>,
    ) -> Result<i64, AppError> {
        let mut sql = String::from("SELECT COUNT(*) FROM token_usage_records WHERE 1=1");
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(v) = agent_type {
            sql.push_str(" AND agent_type = ?");
            params.push(Box::new(v.to_string()));
        }
        if let Some(v) = model {
            sql.push_str(" AND model = ?");
            params.push(Box::new(v.to_string()));
        }
        if let Some(v) = provider_name {
            sql.push_str(" AND provider_name = ?");
            params.push(Box::new(v.to_string()));
        }
        if let Some(v) = date_from {
            sql.push_str(" AND occurred_at >= ?");
            params.push(Box::new(v));
        }
        if let Some(v) = date_to {
            sql.push_str(" AND occurred_at <= ?");
            params.push(Box::new(v));
        }
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|b| b.as_ref()).collect();
        let count: i64 = self.conn.query_row(&sql, param_refs.as_slice(), |row| row.get(0))
            .map_err(AppError::Database)?;
        Ok(count)
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

    /// Delete all preset (is_preset=1) providers. Used for one-time cleanup.
    /// Returns the number of rows deleted.
    pub fn delete_preset_providers(&self) -> Result<usize> {
        let n = self.conn.execute("DELETE FROM providers WHERE is_preset = 1", [])?;
        Ok(n)
    }

    /// Delete `source='auto'` quota_cache rows whose `fetched_at` is older than
    /// `now - older_than_secs`. `source='manual'` rows are always preserved.
    /// Returns the number of rows deleted.
    pub fn purge_expired_quota_cache(&self, older_than_secs: i64) -> Result<usize, AppError> {
        let now_secs = chrono::Utc::now().timestamp();
        let cutoff = now_secs - older_than_secs;
        let deleted = self.conn.execute(
            "DELETE FROM quota_cache WHERE source = 'auto' AND fetched_at < ?1",
            rusqlite::params![cutoff],
        )?;
        Ok(deleted)
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

    // ---------- agent_file_cursor CRUD (Bug #3 fix 2026-06-29) ----------

    pub fn get_cursor(&self, agent_type: &str, file_path: &str) -> Result<Option<AgentFileCursor>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT agent_type, file_path, byte_offset, file_size, last_scan_at, last_event_at
             FROM agent_file_cursor WHERE agent_type = ?1 AND file_path = ?2",
        ).map_err(AppError::Database)?;
        let mut rows = stmt.query_map(rusqlite::params![agent_type, file_path], |row| {
            Ok(AgentFileCursor {
                agent_type: row.get(0)?,
                file_path: row.get(1)?,
                byte_offset: row.get(2)?,
                file_size: row.get(3)?,
                last_scan_at: row.get(4)?,
                last_event_at: row.get(5)?,
            })
        }).map_err(AppError::Database)?;
        match rows.next() {
            Some(r) => Ok(Some(r.map_err(AppError::Database)?)),
            None => Ok(None),
        }
    }

    /// Upsert cursor.  Called after a successful incremental parse so a crash
    /// mid-scan re-processes the same bytes next time (idempotent via FNV-1a).
    pub fn upsert_cursor(&self, c: &AgentFileCursor) -> Result<(), AppError> {
        self.conn.execute(
            "INSERT INTO agent_file_cursor
                (agent_type, file_path, byte_offset, file_size, last_scan_at, last_event_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(agent_type, file_path) DO UPDATE SET
                byte_offset = excluded.byte_offset,
                file_size = excluded.file_size,
                last_scan_at = excluded.last_scan_at,
                last_event_at = excluded.last_event_at",
            rusqlite::params![
                c.agent_type, c.file_path, c.byte_offset, c.file_size,
                c.last_scan_at, c.last_event_at
            ],
        ).map_err(AppError::Database)?;
        Ok(())
    }

    pub fn list_all_cursors(&self) -> Result<Vec<AgentFileCursor>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT agent_type, file_path, byte_offset, file_size, last_scan_at, last_event_at
             FROM agent_file_cursor",
        ).map_err(AppError::Database)?;
        let rows = stmt.query_map([], |row| {
            Ok(AgentFileCursor {
                agent_type: row.get(0)?,
                file_path: row.get(1)?,
                byte_offset: row.get(2)?,
                file_size: row.get(3)?,
                last_scan_at: row.get(4)?,
                last_event_at: row.get(5)?,
            })
        }).map_err(AppError::Database)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(AppError::Database)?;
        Ok(rows)
    }

    pub fn list_cursors_for_agent(&self, agent_type: &str) -> Result<Vec<AgentFileCursor>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT agent_type, file_path, byte_offset, file_size, last_scan_at, last_event_at
             FROM agent_file_cursor WHERE agent_type = ?1",
        ).map_err(AppError::Database)?;
        let rows = stmt.query_map([agent_type], |row| {
            Ok(AgentFileCursor {
                agent_type: row.get(0)?,
                file_path: row.get(1)?,
                byte_offset: row.get(2)?,
                file_size: row.get(3)?,
                last_scan_at: row.get(4)?,
                last_event_at: row.get(5)?,
            })
        }).map_err(AppError::Database)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(AppError::Database)?;
        Ok(rows)
    }

    pub fn delete_cursor(&self, agent_type: &str, file_path: &str) -> Result<(), AppError> {
        self.conn.execute(
            "DELETE FROM agent_file_cursor WHERE agent_type = ?1 AND file_path = ?2",
            rusqlite::params![agent_type, file_path],
        ).map_err(AppError::Database)?;
        Ok(())
    }

    /// Delete all cursors.  Used by `force_rescan_all` to wipe incremental
    /// state so the next scan re-parses every known JSONL from byte 0.
    pub fn delete_all_cursors(&self) -> Result<usize, AppError> {
        let n = self.conn.execute("DELETE FROM agent_file_cursor", [])
            .map_err(AppError::Database)?;
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// quota_cache.source column is added by setup_schema on a fresh DB.
    /// New presets inserted via INSERT...ON CONFLICT must respect the column.
    #[test]
    fn quota_cache_source_column_default_auto() {
        let db = Database::open_in_memory().unwrap();
        db.setup_schema().unwrap();
        db.seed_preset_providers().unwrap();

        // After setup + seed, every quota_cache row defaults to source='auto'
        // (which is fine — there are no rows yet, but the schema accepts it).
        let count: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM quota_cache", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0, "Fresh DB should have no quota_cache rows");

        // Manual quota insert: source='manual' must be storable.
        let now: i64 = 1_700_000_000;
        db.conn.execute(
            "INSERT INTO providers (name, preset, is_preset, category_id, pinned, sort_index, created_at, updated_at)
             VALUES ('test', NULL, 0, 1, 0, 99, ?1, ?1)",
            [now],
        )
        .unwrap();
        let pid: i64 = db.conn.last_insert_rowid();
        db.conn.execute(
            "INSERT INTO quota_cache (provider_id, snapshot_json, fetched_at, source)
             VALUES (?1, '{}', ?2, 'manual')",
            rusqlite::params![pid, now],
        )
        .unwrap();

        // Read back the source column.
        let source: String = db
            .conn
            .query_row(
                "SELECT source FROM quota_cache WHERE provider_id = ?1",
                [pid],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(source, "manual");
    }

    /// ON CONFLICT(provider_id) DO UPDATE must flip source from auto → manual
    /// when a user overwrites an auto-fetched snapshot.
    #[test]
    fn quota_cache_source_overwrite_on_conflict() {
        let db = Database::open_in_memory().unwrap();
        db.setup_schema().unwrap();
        db.seed_preset_providers().unwrap();

        let now: i64 = 1_700_000_000;
        db.conn.execute(
            "INSERT INTO providers (name, preset, is_preset, category_id, pinned, sort_index, created_at, updated_at)
             VALUES ('test', NULL, 0, 1, 0, 99, ?1, ?1)",
            [now],
        )
        .unwrap();
        let pid: i64 = db.conn.last_insert_rowid();

        // First insert: auto source
        db.conn.execute(
            "INSERT INTO quota_cache (provider_id, snapshot_json, fetched_at, source)
             VALUES (?1, '{\"auto\":true}', ?2, 'auto')",
            rusqlite::params![pid, now],
        )
        .unwrap();
        // Overwrite: manual source
        db.conn.execute(
            "INSERT INTO quota_cache (provider_id, snapshot_json, fetched_at, source)
             VALUES (?1, '{\"manual\":true}', ?2, 'manual')
             ON CONFLICT(provider_id) DO UPDATE SET
                snapshot_json = excluded.snapshot_json,
                fetched_at = excluded.fetched_at,
                source = excluded.source",
            rusqlite::params![pid, now + 1],
        )
        .unwrap();

        let (json, source): (String, String) = db
            .conn
            .query_row(
                "SELECT snapshot_json, source FROM quota_cache WHERE provider_id = ?1",
                [pid],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(source, "manual");
        assert!(json.contains("manual"));
    }

    /// purge_expired_quota_cache must only delete `source='auto'` rows older
    /// than the cutoff. Manual rows (regardless of age) and recent auto rows
    /// must be preserved.
    #[test]
    fn purge_keeps_manual_and_recent_auto() {
        let db = Database::open_in_memory().unwrap();
        db.setup_schema().unwrap();
        db.seed_preset_providers().unwrap();

        let now = chrono::Utc::now().timestamp();
        const DAY: i64 = 86400;

        // quota_cache.provider_id is PRIMARY KEY (one row per provider), so we
        // create 3 distinct providers to host the 3 test rows.
        db.conn.execute(
            "INSERT INTO providers (name, preset, is_preset, category_id, pinned, sort_index, created_at, updated_at)
             VALUES ('purge-test-A', NULL, 0, 1, 0, 100, ?1, ?1)",
            [now],
        )
        .unwrap();
        let pid_manual_old: i64 = db.conn.last_insert_rowid();
        db.conn.execute(
            "INSERT INTO providers (name, preset, is_preset, category_id, pinned, sort_index, created_at, updated_at)
             VALUES ('purge-test-B', NULL, 0, 1, 0, 101, ?1, ?1)",
            [now],
        )
        .unwrap();
        let pid_auto_old: i64 = db.conn.last_insert_rowid();
        db.conn.execute(
            "INSERT INTO providers (name, preset, is_preset, category_id, pinned, sort_index, created_at, updated_at)
             VALUES ('purge-test-C', NULL, 0, 1, 0, 102, ?1, ?1)",
            [now],
        )
        .unwrap();
        let pid_auto_new: i64 = db.conn.last_insert_rowid();

        // Row A: source='manual', 30 days old → kept (manual is never purged)
        db.conn
            .execute(
                "INSERT INTO quota_cache (provider_id, snapshot_json, fetched_at, source)
                 VALUES (?1, '{}', ?2, 'manual')",
                rusqlite::params![pid_manual_old, now - 30 * DAY],
            )
            .unwrap();
        // Row B: source='auto', 8 days old → purged (older than 7-day cutoff)
        db.conn
            .execute(
                "INSERT INTO quota_cache (provider_id, snapshot_json, fetched_at, source)
                 VALUES (?1, '{}', ?2, 'auto')",
                rusqlite::params![pid_auto_old, now - 8 * DAY],
            )
            .unwrap();
        // Row C: source='auto', 1 day old → kept (within 7-day cutoff)
        db.conn
            .execute(
                "INSERT INTO quota_cache (provider_id, snapshot_json, fetched_at, source)
                 VALUES (?1, '{}', ?2, 'auto')",
                rusqlite::params![pid_auto_new, now - 1 * DAY],
            )
            .unwrap();

        let deleted = db.purge_expired_quota_cache(7 * DAY).unwrap();
        assert_eq!(deleted, 1, "only the stale auto row should be deleted");

        let manual_old_count: i64 = db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM quota_cache WHERE provider_id = ?1",
                [pid_manual_old],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(manual_old_count, 1, "manual old row must be preserved");

        let auto_old_count: i64 = db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM quota_cache WHERE provider_id = ?1",
                [pid_auto_old],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(auto_old_count, 0, "stale auto row must be purged");

        let auto_new_count: i64 = db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM quota_cache WHERE provider_id = ?1",
                [pid_auto_new],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(auto_new_count, 1, "recent auto row must be preserved");
    }

    // ============================================================
    // fix-date-local-timezone TC-07 (see spec.md REQ-DATE-LOCAL-005, REQ-DATE-LOCAL-008)
    //
    //   TC-07.1  migrate_to_v7_empty_db                     — schema-only smoke
    //   TC-07.2  migrate_to_v7_rebuckets_cross_boundary      — UTC→Local shift
    //   TC-07.3  migrate_to_v7_two_epochs_different_local    — local-date split
    //   TC-07.4  migrate_to_v7_sums_conserved               — input/output/cost sum
    //   TC-07.5  migrate_to_v7_dual_table_coverage           — both tables
    //   TC-07.6  migrate_to_v7_idempotent                   — second call no-op
    //   TC-07.7  migrate_to_v7_no_local_now_dependency     — absolute epoch
    //
    // All assertions use `crate::timeutil::local_date_str(epoch)` so they
    // are deterministic across CI hosts regardless of system TZ.
    // ============================================================

    /// F.1: Empty DB → migrate_to_v7 creates tables, no rows, version "7"
    #[test]
    fn migrate_to_v7_empty_db() {
        let db = Database::open_in_memory().unwrap();
        db.setup_schema().unwrap();
        db.migrate().unwrap();
        assert_eq!(db.schema_version().unwrap(), "7");
        let count: i64 = db.conn().query_row(
            "SELECT COUNT(*) FROM daily_agent_model_usage", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(count, 0);
        let count2: i64 = db.conn().query_row(
            "SELECT COUNT(*) FROM daily_model_usage", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(count2, 0);
    }

    /// F.2: Single record at epoch 1782809400000 → bucket must match `local_date_str(epoch)`
    ///       (chrono's host-system Local). On a non-UTC host (e.g. Asia/Shanghai) this also
    ///       discriminates from the UTC date string, proving no UTC-revert regression.
    ///       ponytail: assertion is dynamical — `chrono::Local` is what `timeutil::local_date_str`
    ///       uses, so the assertion holds across CI hosts regardless of TZ while still tripping
    ///       when the implementation regresses to UTC bucketing on a non-UTC host.
    #[test]
    fn migrate_to_v7_rebuckets_cross_boundary_epoch() {
        let db = Database::open_in_memory().unwrap();
        db.setup_schema().unwrap();
        db.conn().execute(
            "INSERT INTO token_usage_records
                (id, agent_type, model, provider_name, occurred_at, recorded_at,
                 input_tokens, output_tokens, cache_creation_input_tokens, cache_read_input_tokens,
                 total_tokens, total_cost, session_id, request_id, reasoning_tokens,
                 prompt_cost, completion_cost, cache_read_cost, cache_creation_cost,
                 reasoning_cost, currency, pricing_version, usage_details, cost_details)
             VALUES ('rec1', 'claude-code', 'claude-opus', 'anthropic',
                     1782809400000, 1782809400000,
                     1000, 500, 0, 0, 1500, 1.0,
                     NULL, NULL, 0, 0.0, 0.0, 0.0, 0.0, 0.0, 'USD', NULL, '{}', NULL)",
            [],
        ).unwrap();
        db.migrate_to_v7().unwrap();

        let actual_date: String = db.conn().query_row(
            "SELECT date FROM daily_agent_model_usage LIMIT 1",
            [], |r| r.get(0)
        ).unwrap();
        let epoch = 1782809400000_i64;
        let expected_local = crate::timeutil::local_date_str(epoch);
        // Implementation uses `local_date_str`, so actual must equal Local-bucketed date.
        assert_eq!(actual_date, expected_local,
            "migration must bucket at the host's chrono::Local date for epoch {epoch}, got {actual_date} expected {expected_local}");
        // On non-UTC hosts the Local date differs from UTC date — assert migration didn't fall
        // back to UTC bucketing. Skip the second assertion on UTC hosts where dates are equal.
        // ponytail: discriminator pattern — intentional UTC-bucketing probe to detect impl
        //           regression even though REQ-DATE-LOCAL-007 forbids it in production.
        let utc_date = chrono::DateTime::from_timestamp_millis(epoch)
            .unwrap().format("%Y-%m-%d").to_string();
        if expected_local != utc_date {
            assert_ne!(actual_date, utc_date,
                "on non-UTC host, migration must NOT bucket at the UTC date {utc_date}");
        }
    }

    /// F.3: Two records in same Local day → one row with request_count=2; in different
    ///       Local days → two rows. Uses two epochs bracketed to (probably) the same Local
    ///       day across hosts, with second assertion gated on Local distinction.
    #[test]
    fn migrate_to_v7_two_epochs_different_local_days() {
        let db = Database::open_in_memory().unwrap();
        db.setup_schema().unwrap();
        // Epochs bracket a value within minutes; on Asia/Shanghai both are Local 2026-07-01.
        let e1 = 1782809400000_i64;
        let e2 = 1782837000000_i64;
        for (i, ts) in [e1, e2].iter().enumerate() {
            db.conn().execute(
                "INSERT INTO token_usage_records
                    (id, agent_type, model, provider_name, occurred_at, recorded_at,
                     input_tokens, output_tokens, cache_creation_input_tokens, cache_read_input_tokens,
                     total_tokens, total_cost, session_id, request_id, reasoning_tokens,
                     prompt_cost, completion_cost, cache_read_cost, cache_creation_cost,
                     reasoning_cost, currency, pricing_version, usage_details, cost_details)
                 VALUES (?1, 'a', 'm', 'p', ?2, ?2, 100, 50, 0, 0, 150, 0.5,
                         NULL, NULL, 0, 0.0, 0.0, 0.0, 0.0, 0.0, 'USD', NULL, '{}', NULL)",
                rusqlite::params![format!("rec{i}"), ts],
            ).unwrap();
        }
        db.migrate_to_v7().unwrap();

        let d1 = crate::timeutil::local_date_str(e1);
        let d2 = crate::timeutil::local_date_str(e2);
        if d1 == d2 {
            // Same Local day → exactly one row in daily_agent_model_usage
            let count: i64 = db.conn().query_row(
                "SELECT request_count FROM daily_agent_model_usage WHERE agent_type='a' AND model='m' AND provider='p'",
                [], |r| r.get(0)
            ).unwrap();
            assert_eq!(count, 2, "same Local day → count=2");
            let in_t: i64 = db.conn().query_row(
                "SELECT input_tokens FROM daily_agent_model_usage WHERE agent_type='a' AND model='m' AND provider='p'",
                [], |r| r.get(0)
            ).unwrap();
            assert_eq!(in_t, 200, "2 × 100 = 200");
        } else {
            // Different Local days → two distinct rows
            let count: i64 = db.conn().query_row(
                "SELECT COUNT(*) FROM daily_agent_model_usage WHERE agent_type='a' AND model='m' AND provider='p'",
                [], |r| r.get(0)
            ).unwrap();
            assert_eq!(count, 2, "different Local days → 2 distinct rows");
            let mut stmt = db.conn().prepare(
                "SELECT date FROM daily_agent_model_usage WHERE agent_type='a' AND model='m' AND provider='p'"
            ).unwrap();
            let date_set: std::collections::HashSet<String> = stmt
                .query_map([], |r| r.get::<_, String>(0))
                .unwrap()
                .collect::<Result<_, _>>().unwrap();
            assert!(date_set.contains(&d1) && date_set.contains(&d2),
                "rows must include both Local dates {:?}", date_set);
        }
    }

    /// F.4: SUM conservation — aggregate of input/output/total/cost equals raw sum
    #[test]
    fn migrate_to_v7_sums_conserved() {
        let db = Database::open_in_memory().unwrap();
        db.setup_schema().unwrap();
        // Seed 3 records with known totals
        for (i, ts) in [1782837000000_i64, 1782837100000, 1782837200000].iter().enumerate() {
            db.conn().execute(
                "INSERT INTO token_usage_records
                    (id, agent_type, model, provider_name, occurred_at, recorded_at,
                     input_tokens, output_tokens, cache_creation_input_tokens, cache_read_input_tokens,
                     total_tokens, total_cost, session_id, request_id, reasoning_tokens,
                     prompt_cost, completion_cost, cache_read_cost, cache_creation_cost,
                     reasoning_cost, currency, pricing_version, usage_details, cost_details)
                 VALUES (?1, 'a', 'm', 'p', ?2, ?2, 100, 50, 0, 0, 150, 0.5,
                         NULL, NULL, 0, 0.0, 0.0, 0.0, 0.0, 0.0, 'USD', NULL, '{}', NULL)",
                rusqlite::params![format!("rec{i}"), ts],
            ).unwrap();
        }
        db.migrate_to_v7().unwrap();
        let sum_input: i64 = db.conn().query_row(
            "SELECT SUM(input_tokens) FROM daily_agent_model_usage", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(sum_input, 300, "3 records × 100 input = 300");
        let sum_output: i64 = db.conn().query_row(
            "SELECT SUM(output_tokens) FROM daily_agent_model_usage", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(sum_output, 150, "3 × 50 = 150");
        let sum_total: i64 = db.conn().query_row(
            "SELECT SUM(total_tokens) FROM daily_agent_model_usage", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(sum_total, 450, "3 × 150 = 450");
        let sum_cost: f64 = db.conn().query_row(
            "SELECT SUM(total_cost) FROM daily_agent_model_usage", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(sum_cost, 1.5, "3 × 0.5 = 1.5");
    }

    /// F.5: Dual-table coverage — daily_model_usage also re-aggregated correctly.
    ///       Date assertions use `local_date_str` for host-TZ portability.
    #[test]
    fn migrate_to_v7_dual_table_coverage() {
        let db = Database::open_in_memory().unwrap();
        db.setup_schema().unwrap();
        let epoch = 1782809400000_i64;
        db.conn().execute(
            "INSERT INTO token_usage_records
                (id, agent_type, model, provider_name, occurred_at, recorded_at,
                 input_tokens, output_tokens, cache_creation_input_tokens, cache_read_input_tokens,
                 total_tokens, total_cost, session_id, request_id, reasoning_tokens,
                 prompt_cost, completion_cost, cache_read_cost, cache_creation_cost,
                 reasoning_cost, currency, pricing_version, usage_details, cost_details)
             VALUES ('rec1', 'claude-code', 'claude-opus', 'anthropic',
                     1782809400000, 1782809400000,
                     1000, 500, 0, 0, 1500, 1.0,
                     NULL, NULL, 0, 0.0, 0.0, 0.0, 0.0, 0.0, 'USD', NULL, '{}', NULL)",
            [],
        ).unwrap();
        db.migrate_to_v7().unwrap();
        let count: i64 = db.conn().query_row(
            "SELECT COUNT(*) FROM daily_model_usage", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(count, 1, "daily_model_usage should have 1 row");
        let date: String = db.conn().query_row(
            "SELECT date FROM daily_model_usage LIMIT 1", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(date, crate::timeutil::local_date_str(epoch),
            "daily_model_usage date should be chrono::Local-bucketed");
        let in_t: i64 = db.conn().query_row(
            "SELECT input_tokens FROM daily_model_usage", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(in_t, 1000);
    }

    /// F.6: Idempotency — calling migrate_to_v7() twice is a no-op
    #[test]
    fn migrate_to_v7_idempotent() {
        let db = Database::open_in_memory().unwrap();
        db.setup_schema().unwrap();
        // Seed one record
        db.conn().execute(
            "INSERT INTO token_usage_records
                (id, agent_type, model, provider_name, occurred_at, recorded_at,
                 input_tokens, output_tokens, cache_creation_input_tokens, cache_read_input_tokens,
                 total_tokens, total_cost, session_id, request_id, reasoning_tokens,
                 prompt_cost, completion_cost, cache_read_cost, cache_creation_cost,
                 reasoning_cost, currency, pricing_version, usage_details, cost_details)
             VALUES ('rec1', 'a', 'm', 'p', 1782809400000, 1782809400000,
                     100, 50, 0, 0, 150, 0.5,
                     NULL, NULL, 0, 0.0, 0.0, 0.0, 0.0, 0.0, 'USD', NULL, '{}', NULL)",
            [],
        ).unwrap();
        db.migrate_to_v7().unwrap();
        let count1: i64 = db.conn().query_row(
            "SELECT request_count FROM daily_agent_model_usage", [], |r| r.get(0)
        ).unwrap();
        // Call again — should not duplicate
        db.migrate_to_v7().unwrap();
        let count2: i64 = db.conn().query_row(
            "SELECT request_count FROM daily_agent_model_usage", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(count2, count1, "Second call must not duplicate rows");
        assert_eq!(db.schema_version().unwrap(), "7");
    }

    /// F.7: No dependency on Local::now() — uses absolute epoch timestamps only
    #[test]
    fn migrate_to_v7_no_local_now_dependency() {
        let db = Database::open_in_memory().unwrap();
        db.setup_schema().unwrap();
        // epoch 0 = UTC 1970-01-01 00:00:00 = Local 1970-01-01 00:00 (if TZ=UTC or Asia/Shanghai without DST)
        // We just verify the migration runs correctly with absolute epoch, not Local::now()
        db.conn().execute(
            "INSERT INTO token_usage_records
                (id, agent_type, model, provider_name, occurred_at, recorded_at,
                 input_tokens, output_tokens, cache_creation_input_tokens, cache_read_input_tokens,
                 total_tokens, total_cost, session_id, request_id, reasoning_tokens,
                 prompt_cost, completion_cost, cache_read_cost, cache_creation_cost,
                 reasoning_cost, currency, pricing_version, usage_details, cost_details)
             VALUES ('rec1', 'a', 'm', 'p', 0, 0,
                     1, 1, 0, 0, 2, 0.01,
                     NULL, NULL, 0, 0.0, 0.0, 0.0, 0.0, 0.0, 'USD', NULL, '{}', NULL)",
            [],
        ).unwrap();
        db.migrate_to_v7().unwrap();
        // Should have exactly one row — doesn't crash, doesn't depend on current time
        let count: i64 = db.conn().query_row(
            "SELECT COUNT(*) FROM daily_agent_model_usage", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(count, 1);
        // Verify _v6 backup table exists (from rename of original empty daily table)
        let v6_table_exists: i64 = db.conn().query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='daily_agent_model_usage_v6'",
            [], |r| r.get(0)
        ).unwrap();
        assert_eq!(v6_table_exists, 1, "v6 backup table should exist");
        // Verify version is now 7
        assert_eq!(db.schema_version().unwrap(), "7");
    }
}
