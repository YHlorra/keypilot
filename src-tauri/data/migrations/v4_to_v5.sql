-- v4 → v5: 删除 token_usage_records.prompt_tokens / completion_tokens 历史遗留列
-- 原因: V0.1 早期为兼容 Codex 命名,实际所有读写都走 input_tokens / output_tokens
-- 重建表模式: SQLite 不支持 DROP COLUMN(3.35.0 以下),用 CREATE+COPY+DROP+RENAME

BEGIN;

-- 1. 创建新表(不含 prompt_tokens / completion_tokens,列顺序按 v5 规范)
CREATE TABLE token_usage_records_new (
    id                            TEXT    PRIMARY KEY,
    agent_type                    TEXT    NOT NULL,
    model                         TEXT    NOT NULL,
    provider_name                 TEXT    NOT NULL,
    occurred_at                   INTEGER NOT NULL,
    recorded_at                   INTEGER NOT NULL,
    session_id                    TEXT,
    request_id                    TEXT,
    input_tokens                  INTEGER DEFAULT 0,
    output_tokens                 INTEGER DEFAULT 0,
    cache_read_input_tokens       INTEGER DEFAULT 0,
    cache_creation_input_tokens   INTEGER DEFAULT 0,
    reasoning_tokens              INTEGER DEFAULT 0,
    total_tokens                  INTEGER DEFAULT 0,
    prompt_cost                   REAL    DEFAULT 0.0,
    completion_cost               REAL    DEFAULT 0.0,
    cache_read_cost               REAL    DEFAULT 0.0,
    cache_creation_cost           REAL    DEFAULT 0.0,
    reasoning_cost                REAL    DEFAULT 0.0,
    total_cost                    REAL    DEFAULT 0.0,
    currency                      TEXT    DEFAULT 'USD',
    pricing_version               TEXT,
    usage_details                 TEXT    DEFAULT '{}',
    cost_details                  TEXT    DEFAULT NULL
);

-- 2. 复制数据(忽略 prompt_tokens / completion_tokens 列;显式列出列名以匹配新表顺序)
INSERT INTO token_usage_records_new (
    id, agent_type, model, provider_name, occurred_at, recorded_at,
    session_id, request_id,
    input_tokens, output_tokens,
    cache_read_input_tokens, cache_creation_input_tokens, reasoning_tokens, total_tokens,
    prompt_cost, completion_cost, cache_read_cost, cache_creation_cost, reasoning_cost, total_cost,
    currency, pricing_version, usage_details, cost_details
)
SELECT
    id, agent_type, model, provider_name, occurred_at, recorded_at,
    session_id, request_id,
    input_tokens, output_tokens,
    cache_read_input_tokens, cache_creation_input_tokens, reasoning_tokens, total_tokens,
    prompt_cost, completion_cost, cache_read_cost, cache_creation_cost, reasoning_cost, total_cost,
    currency, pricing_version, usage_details, cost_details
FROM token_usage_records;

-- 3. 替换旧表
DROP TABLE token_usage_records;
ALTER TABLE token_usage_records_new RENAME TO token_usage_records;

-- 4. 重建索引
CREATE INDEX IF NOT EXISTS idx_token_usage_occurred     ON token_usage_records(occurred_at);
CREATE INDEX IF NOT EXISTS idx_token_usage_agent_model ON token_usage_records(agent_type, model, occurred_at);

COMMIT;
