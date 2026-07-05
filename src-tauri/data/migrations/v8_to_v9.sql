-- v8 → v9: custom_spec 列 (用户自定义供应商配置)
-- 原因: 五层抽象落地的最小存储扩展。
--       preset 列此前已能存任意字符串 (catalog id 或 'custom-...' marker),
--       但缺少其对应的 base_url/api/auth/validate_probe 覆写。
--       custom_spec 是 JSON 字符串,仅在用户从 UI 添加"自定义供应商"时填充。
--       所有现有行 (preset = 'openai' 等) 不受影响,新列 NULL。
--
-- 幂等: 新 DB 的 CREATE TABLE 已经包含该列,这条 ALTER 必须容忍已存在。
BEGIN;
ALTER TABLE providers ADD COLUMN custom_spec TEXT;
COMMIT;
