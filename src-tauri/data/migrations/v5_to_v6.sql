-- v5 → v6: agent_file_cursor 表(增量导入游标)
-- 原因: Bug #3 修复,从"启动一次性扫描"改为"文件级 byte-cursor + notify watcher + 增量解析"。
--       没有 cursor 的话,DB 满 100 行后扫描就变 no-op,新生成的 JSONL 永远进不来。
-- 备注: agent_file_cursor 表的实际 CREATE TABLE 在 setup_schema() 里以
--       IF NOT EXISTS 形式存在,这样新老 DB 都能拿到表(setup_schema 在
--       migrate() 之前运行,见 lib.rs::setup)。这个迁移文件主要是声明性 +
--       记录 schema 变化。
BEGIN;
COMMIT;