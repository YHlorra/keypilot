CREATE TABLE IF NOT EXISTS token_usage_records (
    id TEXT PRIMARY KEY,
    agent_type TEXT NOT NULL,
    model TEXT NOT NULL,
    provider_name TEXT NOT NULL,
    occurred_at INTEGER NOT NULL,
    recorded_at INTEGER NOT NULL,
    session_id TEXT,
    request_id TEXT,
    prompt_tokens INTEGER DEFAULT 0,
    completion_tokens INTEGER DEFAULT 0,
    total_tokens INTEGER DEFAULT 0,
    cache_read_input_tokens INTEGER DEFAULT 0,
    cache_creation_input_tokens INTEGER DEFAULT 0,
    reasoning_tokens INTEGER DEFAULT 0,
    input_tokens INTEGER DEFAULT 0,
    output_tokens INTEGER DEFAULT 0,
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
);

CREATE TABLE IF NOT EXISTS daily_agent_model_usage (
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
);

CREATE TABLE IF NOT EXISTS daily_model_usage (
    date TEXT NOT NULL,
    model TEXT NOT NULL,
    provider TEXT NOT NULL,
    request_count INTEGER DEFAULT 0,
    input_tokens INTEGER DEFAULT 0,
    output_tokens INTEGER DEFAULT 0,
    total_tokens INTEGER DEFAULT 0,
    total_cost REAL DEFAULT 0.0,
    PRIMARY KEY (date, model, provider)
);

CREATE INDEX IF NOT EXISTS idx_token_usage_occurred ON token_usage_records(occurred_at);
CREATE INDEX IF NOT EXISTS idx_token_usage_agent_model ON token_usage_records(agent_type, model, occurred_at);
