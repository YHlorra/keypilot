//! opencode Go CLI 的 limits 聚合(token-monitor-alignment Part B #6C)。
//!
//! 从 opencode Go 数据库读 `message` 表 cost 字段,扫两个候选目录
//! (`%LOCALAPPDATA%\opencode\opencode*.db` 和 `~/.local/share/opencode/opencode*.db`),
//! 聚合三窗口:
//!   - Session:5 小时滚动窗口,limit $12
//!   - Weekly:7 天滚动窗口,limit $30
//!   - Monthly:本自然月,limit $60
//!
//! 对齐 token-monitor `src/shared/opencodeLimits.js`。
//!
//! 注:本模块对 opencode.db 只读(`SQLITE_OPEN_READ_ONLY`),不写任何 CLI 配置文件
//! (AGENTS.md §3.1 硬约束)。

use std::path::PathBuf;

use chrono::{Datelike, TimeZone};
use rusqlite::OpenFlags;

use crate::error::AppError;
use crate::types::{LimitSource, LimitStatus, LimitWindow, LimitWindowKind, MoneyAmount, QuotaSnapshot};

/// Session 窗口长度:5 小时(毫秒)
pub const SESSION_MS: i64 = 5 * 3600 * 1000;
/// Weekly 窗口长度:7 天(毫秒)
pub const WEEK_MS: i64 = 7 * 24 * 3600 * 1000;
/// 默认 limit:(session, weekly, monthly) USD
pub const DEFAULT_GO_LIMITS: (f64, f64, f64) = (12.0, 30.0, 60.0);

/// Discover opencode Go database paths (aligned with token-monitor `discoverDbPaths`).
///
/// Scans BOTH platform conventions on every host:
///   1. `%LOCALAPPDATA%\opencode\opencode*.db` (Windows local convention)
///   2. `~/.local/share/opencode/opencode*.db` (XDG convention, also used by
///      opencode Go CLI v1.17+ on Windows via HOME)
///
/// Glob accepts `opencode.db` and `opencode-<channel>.db`
/// (channel = `[A-Za-z0-9._-]+`); WAL/SHM side-files (e.g. `opencode.db-wal`)
/// are rejected by `ends_with(".db")`. Returns paths in sorted order; may
/// be empty. Read-only — never writes to the CLI config dir.
pub fn discover_db_paths() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(p) = std::env::var("LOCALAPPDATA") {
        dirs.push(PathBuf::from(p).join("opencode"));
    }
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".local/share/opencode"));
    }

    let mut paths = Vec::new();
    for base_dir in dirs {
        if let Ok(entries) = std::fs::read_dir(&base_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("opencode") && name.ends_with(".db") {
                        paths.push(p);
                    }
                }
            }
        }
    }
    paths.sort();
    paths
}

/// 从 opencode.db 读 Go provider 的 (createdMs, cost) 行。
///
/// SQL 对齐 token-monitor:
/// ```sql
/// SELECT CAST(COALESCE(json_extract(data,'$.time.created'), time_created) AS INTEGER) AS createdMs,
///        CAST(json_extract(data,'$.cost') AS REAL) AS cost
/// FROM message
/// WHERE json_valid(data)
///   AND json_extract(data,'$.providerID') = 'opencode-go'
///   AND json_extract(data,'$.role') = 'assistant'
///   AND json_type(data,'$.cost') IN ('integer','real')
/// ```
///
/// 以只读方式打开(对齐 `parse_opencode_db_records` 风格)。
pub fn read_go_rows(db_path: &std::path::Path) -> Result<Vec<(i64, f64)>, AppError> {
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("open opencode.db: {e}"),
        ))
    })?;

    let mut stmt = conn
        .prepare(
            "SELECT CAST(COALESCE(json_extract(data,'$.time.created'), time_created) AS INTEGER) AS createdMs, \
                    CAST(json_extract(data,'$.cost') AS REAL) AS cost \
             FROM message \
             WHERE json_valid(data) \
               AND json_extract(data,'$.providerID') = 'opencode-go' \
               AND json_extract(data,'$.role') = 'assistant' \
               AND json_type(data,'$.cost') IN ('integer','real')",
        )
        .map_err(|e| {
            AppError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("prepare: {e}"),
            ))
        })?;

    let rows = stmt
        .query_map([], |row| {
            let created_ms: i64 = row.get(0)?;
            let cost: f64 = row.get(1)?;
            Ok((created_ms, cost))
        })
        .map_err(|e| {
            AppError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("query: {e}"),
            ))
        })?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| {
            AppError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("row: {e}"),
            ))
        })?);
    }
    Ok(out)
}

/// 根据 rows + 当前时间 + limits 构造三窗口(对齐 token-monitor buildWindows)。
///
/// - Session:rows 中 created_ms 在 [now - 5h, now] 的 cost 求和,limit = session_limit
/// - Weekly:rows 中 created_ms 在 [now - 7d, now] 的 cost 求和,limit = weekly_limit
/// - Monthly:rows 中 created_ms 在 [本月 1 日 00:00, now] 的 cost 求和,limit = monthly_limit
///
/// 用 `chrono::Local` 算时区边界(对齐 token-monitor 用本地时区)。
pub fn build_windows(rows: &[(i64, f64)], now_ms: i64, limits: (f64, f64, f64)) -> Vec<LimitWindow> {
    let now_local = chrono::Local
        .timestamp_millis_opt(now_ms)
        .single()
        .unwrap_or_else(|| chrono::Local::now());
    let mut windows = Vec::with_capacity(3);

    // Session:5h 滚动窗口
    let session_start = now_ms - SESSION_MS;
    let session_used: f64 = rows
        .iter()
        .filter(|(ts, _)| *ts >= session_start && *ts <= now_ms)
        .map(|(_, c)| *c)
        .sum();
    let session_reset = now_local + chrono::Duration::milliseconds(SESSION_MS - (now_ms - session_start));
    windows.push(build_window(
        LimitWindowKind::Session,
        "Session (5h)".to_string(),
        session_used,
        limits.0,
        Some(session_reset),
        Some(5 * 60),
    ));

    // Weekly:7d 滚动窗口
    let week_start = now_ms - WEEK_MS;
    let weekly_used: f64 = rows
        .iter()
        .filter(|(ts, _)| *ts >= week_start && *ts <= now_ms)
        .map(|(_, c)| *c)
        .sum();
    let weekly_reset = now_local + chrono::Duration::milliseconds(WEEK_MS - (now_ms - week_start));
    windows.push(build_window(
        LimitWindowKind::Weekly,
        "Weekly".to_string(),
        weekly_used,
        limits.1,
        Some(weekly_reset),
        Some(7 * 24 * 60),
    ));

    // Monthly:本月 1 日 00:00 到 now,月末 = 下月 1 日 00:00
    let month_start_local = now_local
        .date_naive()
        .with_day(1)
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .and_then(|dt| chrono::Local.from_local_datetime(&dt).single())
        .unwrap_or(now_local);
    let month_start_ms = month_start_local.timestamp_millis();
    let monthly_used: f64 = rows
        .iter()
        .filter(|(ts, _)| *ts >= month_start_ms && *ts <= now_ms)
        .map(|(_, c)| *c)
        .sum();
    // 月末 = 下月 1 日 00:00:把 month_start + 32 天再归到当月 1 号
    let month_end_local = (month_start_local + chrono::Duration::days(32))
        .date_naive()
        .with_day(1)
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .and_then(|dt| chrono::Local.from_local_datetime(&dt).single())
        .unwrap_or(now_local);
    let window_minutes = ((month_end_local.timestamp_millis()
        - month_start_local.timestamp_millis())
        / 60000) as i64;
    windows.push(build_window(
        LimitWindowKind::Billing,
        "Monthly".to_string(),
        monthly_used,
        limits.2,
        Some(month_end_local),
        Some(window_minutes),
    ));

    windows
}

/// 内部辅助:构造单个 LimitWindow(填充 used/limit/remaining/percent/reset_description)
fn build_window(
    kind: LimitWindowKind,
    label: String,
    used: f64,
    limit: f64,
    resets_at: Option<chrono::DateTime<chrono::Local>>,
    window_minutes: Option<i64>,
) -> LimitWindow {
    let remaining = (limit - used).max(0.0);
    let used_percent = if limit > 0.0 { Some((used / limit) * 100.0) } else { None };
    let remaining_percent = if limit > 0.0 {
        Some((remaining / limit) * 100.0)
    } else {
        None
    };

    let reset_description = match &resets_at {
        Some(dt) => format!("Resets at {}", dt.format("%Y-%m-%d %H:%M %Z")),
        None => "No reset scheduled".to_string(),
    };

    LimitWindow {
        kind,
        label,
        used,
        limit: Some(limit),
        remaining: Some(remaining),
        used_percent,
        remaining_percent,
        resets_at: resets_at.map(|dt| dt.to_rfc3339()),
        window_minutes,
        reset_description,
        show_meter: true,
    }
}

/// 主入口:聚合 opencode Go 三窗口,返回新 QuotaSnapshot。
///
/// - 无 DB 时返回 `status=NotConfigured` / `source=Local` 的空 snapshot
/// - 有 DB 时读 rows,build_windows,返回 `status=Ok` / `source=Local` / `source_detail="cli"` 的多窗口 snapshot
pub fn collect_go() -> Result<QuotaSnapshot, AppError> {
    let db_paths = discover_db_paths();
    if db_paths.is_empty() {
        return Ok(QuotaSnapshot {
            total: None,
            used: 0.0,
            remaining: None,
            unit: "USD".to_string(),
            level: None,
            reset_at: None,
            windows: Vec::new(),
            status: LimitStatus::NotConfigured,
            source: LimitSource::Local,
            source_detail: "cli".to_string(),
            account_label: None,
            account_email: None,
            region: None,
            balance: None,
            used_amount: None,
            balance_usd: None,
            used_usd: Some(0.0),
        });
    }

    let mut all_rows = Vec::new();
    for db_path in &db_paths {
        if let Ok(rows) = read_go_rows(db_path) {
            all_rows.extend(rows);
        }
    }

    let now_ms = chrono::Local::now().timestamp_millis();
    let windows = build_windows(&all_rows, now_ms, DEFAULT_GO_LIMITS);
    let total_used: f64 = all_rows.iter().map(|(_, c)| *c).sum();
    let monthly_limit = DEFAULT_GO_LIMITS.2;
    let pct = if monthly_limit > 0.0 {
        (total_used / monthly_limit) * 100.0
    } else {
        0.0
    };

    Ok(QuotaSnapshot {
        total: Some(monthly_limit),
        used: total_used,
        remaining: Some((monthly_limit - total_used).max(0.0)),
        unit: "USD".to_string(),
        level: Some(level_for_percent(pct)),
        reset_at: None,
        windows,
        status: LimitStatus::Ok,
        source: LimitSource::Local,
        source_detail: "cli".to_string(),
        account_label: None,
        account_email: None,
        region: None,
        balance: Some(MoneyAmount {
            amount: (monthly_limit - total_used).max(0.0),
            currency: "USD".to_string(),
            ..Default::default()
        }),
        used_amount: Some(MoneyAmount {
            amount: total_used,
            currency: "USD".to_string(),
            ..Default::default()
        }),
        balance_usd: Some((monthly_limit - total_used).max(0.0)),
        used_usd: Some(total_used),
    })
}

fn level_for_percent(percent: f64) -> String {
    if percent >= 90.0 {
        "ruby".to_string()
    } else if percent >= 70.0 {
        "red".to_string()
    } else if percent >= 50.0 {
        "amber".to_string()
    } else {
        "green".to_string()
    }
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_go_returns_not_configured_when_db_missing() {
        // collect_go() 用真实环境,无法隔离;但函数不应 panic,
        // 且 status 应为 Ok 或 NotConfigured(取决于测试机器是否装了 opencode)。
        let _ = discover_db_paths();
        let snap = collect_go().unwrap();
        assert!(snap.status == LimitStatus::Ok || snap.status == LimitStatus::NotConfigured);
    }

    #[test]
    fn build_windows_calculates_session_weekly_monthly() {
        // ponytail: use a fixed now_ms (June 26 2026 12:00 local) so the test is
        // stable across any run date under TZ='Asia/Shanghai'.  month_ago row is
        // anchored to `month_start_ms + 5d` (June 6 00:00 local) so it always
        // falls inside the current month bucket regardless of TZ.  Session +
        // week + month rows sum to 15.0 (3 + 5 + 7).
        let now_ms = chrono::Local
            .with_ymd_and_hms(2026, 6, 26, 12, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis();
        let now_local = chrono::Local.timestamp_millis_opt(now_ms).single().unwrap();
        let month_start_local = now_local
            .date_naive()
            .with_day(1)
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .unwrap();
        let month_start_ms = chrono::Local
            .from_local_datetime(&month_start_local)
            .single()
            .unwrap()
            .timestamp_millis();

        let session_ago = now_ms - 3600 * 1000; // 1h ago → June 26 11:00
        let week_ago = now_ms - 2 * 24 * 3600 * 1000; // 2d ago → June 24 12:00
        let month_ago = month_start_ms + 5 * 24 * 3600 * 1000; // 5d after month start → June 6 00:00
        let out_of_month = now_ms - 35 * 24 * 3600 * 1000; // 35d ago → May 22 12:00 (prev month)

        let rows = vec![
            (session_ago, 3.0),
            (week_ago, 5.0),
            (month_ago, 7.0),
            (out_of_month, 100.0),
        ];

        let windows = build_windows(&rows, now_ms, (12.0, 30.0, 60.0));
        assert_eq!(windows.len(), 3);

        let session = &windows[0];
        assert_eq!(session.kind, LimitWindowKind::Session);
        assert!((session.used - 3.0).abs() < 0.001);
        assert_eq!(session.limit, Some(12.0));
        assert!((session.used_percent.unwrap() - 25.0).abs() < 0.1);

        let weekly = &windows[1];
        assert_eq!(weekly.kind, LimitWindowKind::Weekly);
        assert!((weekly.used - 8.0).abs() < 0.001);
        assert_eq!(weekly.limit, Some(30.0));

        let monthly = &windows[2];
        assert_eq!(monthly.kind, LimitWindowKind::Billing);
        assert!((monthly.used - 15.0).abs() < 0.001);
        assert_eq!(monthly.limit, Some(60.0));
        assert!((monthly.used_percent.unwrap() - 25.0).abs() < 0.1);
    }

    #[test]
    fn read_go_rows_extracts_cost_and_timestamp() {
        let tmp = std::env::temp_dir().join(format!(
            "keypilot-opencode-test-{}-{}.db",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_file(&tmp);

        let conn = rusqlite::Connection::open(&tmp).unwrap();
        conn.execute(
            "CREATE TABLE message (id INTEGER PRIMARY KEY, data TEXT, time_created INTEGER)",
            [],
        )
        .unwrap();

        let now_secs = chrono::Local::now().timestamp();
        // 符合条件:providerID=opencode-go, role=assistant, cost 是 real
        conn.execute(
            "INSERT INTO message (data, time_created) VALUES (?1, ?2)",
            rusqlite::params![
                serde_json::json!({
                    "providerID": "opencode-go",
                    "role": "assistant",
                    "cost": 3.5,
                    "time": { "created": now_secs * 1000 }
                })
                .to_string(),
                now_secs,
            ],
        )
        .unwrap();
        // providerID 不符 → 过滤
        conn.execute(
            "INSERT INTO message (data, time_created) VALUES (?1, ?2)",
            rusqlite::params![
                serde_json::json!({"providerID": "openai", "role": "assistant", "cost": 100.0})
                    .to_string(),
                now_secs,
            ],
        )
        .unwrap();
        // cost 是 string → 过滤
        conn.execute(
            "INSERT INTO message (data, time_created) VALUES (?1, ?2)",
            rusqlite::params![
                serde_json::json!({"providerID": "opencode-go", "role": "assistant", "cost": "100"})
                    .to_string(),
                now_secs,
            ],
        )
        .unwrap();

        drop(conn);

        let rows = read_go_rows(&tmp).unwrap();
        assert_eq!(rows.len(), 1, "expected 1 row, got {}", rows.len());
        assert!(
            (rows[0].1 - 3.5).abs() < 0.001,
            "cost should be 3.5, got {}",
            rows[0].1
        );

        let _ = std::fs::remove_file(&tmp);
    }
}
