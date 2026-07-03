



use crate::AppError;
use chrono::{Local, NaiveDate, TimeZone};


pub fn now_secs() -> i64 {
    chrono::Utc::now().timestamp()
}


pub fn now_millis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}




pub fn local_date_str(epoch_ms: i64) -> String {
    Local.timestamp_millis_opt(epoch_ms).single()
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "1970-01-01".to_string())
}





pub fn local_date_to_epoch(s: &str, exclusive: bool) -> Result<i64, AppError> {
    let date = NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|e| AppError::TokenUsageInvalidFormat(format!("invalid date '{s}': {e}")))?;
    let target = (if exclusive { date.succ_opt() } else { Some(date) })
        .ok_or_else(|| AppError::TokenUsageInvalidFormat("date overflow".into()))?;
    Ok(target
        .and_hms_opt(0, 0, 0).unwrap()        
        .and_local_timezone(Local).single()
        .ok_or_else(|| AppError::TokenUsageInvalidFormat(format!("local datetime '{s}' ambiguous")))?
        .timestamp_millis())
}