// src-tauri/src/timeutil.rs
//
// Single source of truth for epoch → Local date-string conversions in keypilot.
// REQ-DATE-LOCAL-003 contract: Local is system timezone; see openspec.
use crate::AppError;
use chrono::{Local, NaiveDate, TimeZone};

/// Current Unix epoch time in seconds.
pub fn now_secs() -> i64 {
    chrono::Utc::now().timestamp()
}

/// Current Unix epoch time in milliseconds.
pub fn now_millis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// Epoch ms → Local 系统时区的 "YYYY-MM-DD" 字符串。
/// Local 是 OS TZ,桌面应用系统时区。
/// 非法 epoch 返回 "1970-01-01"(见 REQ-DATE-LOCAL-003)。
pub fn local_date_str(epoch_ms: i64) -> String {
    Local.timestamp_millis_opt(epoch_ms).single()
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "1970-01-01".to_string())
}

/// "YYYY-MM-DD" 字符串 → Local 当日 00:00 的 epoch ms。
/// exclusive=true 时返回次日 00:00(用于 half-open `[from, to)` 区间)。
/// ponytail: 真午夜的 Local wall-clock 永不歧义(DST 跳变在 02–03 时区),
///           所以 .single() 在本 codebase 内是 total,.latest() YAGNI。
pub fn local_date_to_epoch(s: &str, exclusive: bool) -> Result<i64, AppError> {
    let date = NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|e| AppError::TokenUsageInvalidFormat(format!("invalid date '{s}': {e}")))?;
    let target = (if exclusive { date.succ_opt() } else { Some(date) })
        .ok_or_else(|| AppError::TokenUsageInvalidFormat("date overflow".into()))?;
    Ok(target
        .and_hms_opt(0, 0, 0).unwrap()        // and_hms_opt is total for valid NaiveDate
        .and_local_timezone(Local).single()
        .ok_or_else(|| AppError::TokenUsageInvalidFormat(format!("local datetime '{s}' ambiguous")))?
        .timestamp_millis())
}