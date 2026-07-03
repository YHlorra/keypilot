











use chrono::{Datelike, Local, TimeZone};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::AppError;


pub const RETENTION_MS: i64 = 40 * 24 * 3600 * 1000;


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Consumption {
    pub today_spend: f64,
    pub month_spend: f64,
    pub month_since_tracking: bool,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceSnapshot {
    pub ts: i64,
    pub paid: f64,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountEntry {
    pub currency: String,
    pub snapshots: Vec<BalanceSnapshot>,
}


pub type BalanceStore = BTreeMap<String, AccountEntry>;




pub fn store_path() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("com.keypilot.app").join("deepseek-balance.json")
}


fn read_store(path: &Path) -> Result<BalanceStore, AppError> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(serde_json::from_str(&text)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(BalanceStore::new()),
        Err(e) => Err(AppError::Io(e)),
    }
}


fn write_store_atomic(path: &Path, store: &BalanceStore) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp_path = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(store)?;
    let mut file = fs::File::create(&tmp_path)?;
    file.write_all(json.as_bytes())?;
    file.sync_all()?;
    drop(file);
    fs::rename(&tmp_path, path)?;
    Ok(())
}







pub fn record_consumption(
    account_key: &str,
    currency: &str,
    paid: f64,
    now_ms: i64,
) -> Result<Consumption, AppError> {
    let path = store_path();
    let mut store = read_store(&path)?;

    let entry = store.entry(account_key.to_string()).or_insert_with(|| AccountEntry {
        currency: currency.to_string(),
        snapshots: Vec::new(),
    });
    
    if entry.currency != currency {
        entry.currency = currency.to_string();
        entry.snapshots.clear();
    }
    entry.snapshots.push(BalanceSnapshot { ts: now_ms, paid });

    
    entry.snapshots.retain(|s| s.ts >= now_ms - RETENTION_MS);
    entry.snapshots.sort_by_key(|s| s.ts);

    let snapshots: Vec<(i64, f64)> = entry.snapshots.iter().map(|s| (s.ts, s.paid)).collect();
    let consumption = compute_consumption(&snapshots, now_ms);

    write_store_atomic(&path, &store)?;
    Ok(consumption)
}







pub fn compute_consumption(snapshots: &[(i64, f64)], now_ms: i64) -> Consumption {
    if snapshots.is_empty() {
        return Consumption {
            today_spend: 0.0,
            month_spend: 0.0,
            month_since_tracking: false,
        };
    }

    
    let mut sorted: Vec<(i64, f64)> = snapshots.to_vec();
    sorted.sort_by_key(|s| s.0);

    let now_local = Local.timestamp_millis_opt(now_ms).unwrap();
    let today_start = now_local.date_naive().and_hms_opt(0, 0, 0).unwrap();
    let today_start_ms = Local
        .from_local_datetime(&today_start)
        .unwrap()
        .timestamp_millis();
    let month_start = today_start.with_day(1).unwrap_or(today_start);
    let month_start_ms = Local
        .from_local_datetime(&month_start)
        .unwrap()
        .timestamp_millis();

    let mut today_spend = 0.0_f64;
    let mut month_spend = 0.0_f64;
    for i in 1..sorted.len() {
        let prev = sorted[i - 1].1;
        let cur = sorted[i].1;
        let drop = (prev - cur).max(0.0);
        if drop <= 0.0 {
            continue;
        }
        let ts = sorted[i].0;
        if ts >= today_start_ms && ts < today_start_ms + 24 * 3600 * 1000 {
            today_spend += drop;
        }
        if ts >= month_start_ms {
            month_spend += drop;
        }
    }

    let earliest_ts = sorted[0].0;
    let month_since_tracking = earliest_ts > month_start_ms;

    Consumption {
        today_spend: round2(today_spend),
        month_spend: round2(month_spend),
        month_since_tracking,
    }
}


fn round2(value: f64) -> f64 {
    ((value + f64::EPSILON) * 100.0).round() / 100.0
}


pub(crate) fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts_local(year: i32, month: u32, day: u32, hour: u32, min: u32) -> i64 {
        use chrono::NaiveDate;
        let date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
        let dt = date.and_hms_opt(hour, min, 0).unwrap();
        Local.from_local_datetime(&dt).unwrap().timestamp_millis()
    }

    
    fn cleanup_test_account(account_key: &str) {
        let path = store_path();
        if let Ok(mut store) = read_store(&path) {
            if store.remove(account_key).is_some() {
                let _ = write_store_atomic(&path, &store);
            }
        }
    }

    #[test]
    fn compute_consumption_first_query_returns_zero() {
        
        let now = ts_local(2026, 6, 29, 12, 0);
        let snapshots = vec![(now, 100.0)];
        let consumption = compute_consumption(&snapshots, now);
        assert_eq!(consumption.today_spend, 0.0);
        assert_eq!(consumption.month_spend, 0.0);
        
        assert!(consumption.month_since_tracking);
    }

    #[test]
    fn compute_consumption_drop_today() {
        
        let yesterday = ts_local(2026, 6, 28, 12, 0);
        let today = ts_local(2026, 6, 29, 12, 0);
        let snapshots = vec![(yesterday, 100.0), (today, 70.0)];
        let consumption = compute_consumption(&snapshots, today);
        assert_eq!(consumption.today_spend, 30.0);
        assert_eq!(consumption.month_spend, 30.0);
    }

    #[test]
    fn compute_consumption_drop_this_month() {
        
        let last_month = ts_local(2026, 5, 29, 12, 0);
        let earlier_this_month = ts_local(2026, 6, 15, 12, 0);
        let now = ts_local(2026, 6, 29, 12, 0);
        let snapshots = vec![(last_month, 100.0), (earlier_this_month, 70.0)];
        let consumption = compute_consumption(&snapshots, now);
        assert_eq!(consumption.today_spend, 0.0); 
        assert_eq!(consumption.month_spend, 30.0); 
        assert!(!consumption.month_since_tracking); 
    }

    #[test]
    fn compute_consumption_topup_does_not_count() {
        
        let t1 = ts_local(2026, 6, 29, 10, 0);
        let t2 = ts_local(2026, 6, 29, 12, 0);
        let snapshots = vec![(t1, 70.0), (t2, 120.0)];
        let consumption = compute_consumption(&snapshots, t2);
        assert_eq!(consumption.today_spend, 0.0);
        assert_eq!(consumption.month_spend, 0.0);
    }

    #[test]
    fn compute_consumption_empty_returns_zero() {
        let now = ts_local(2026, 6, 29, 12, 0);
        let consumption = compute_consumption(&[], now);
        assert_eq!(consumption.today_spend, 0.0);
        assert_eq!(consumption.month_spend, 0.0);
        assert!(!consumption.month_since_tracking);
    }

    #[test]
    fn record_consumption_creates_store_file() {
        
        
        
        let account_key = "sha256:test_record_consumption_creates_store_file";
        cleanup_test_account(account_key); 

        let now = chrono::Local::now().timestamp_millis();
        let result = record_consumption(account_key, "CNY", 100.0, now);
        assert!(result.is_ok());
        let consumption = result.unwrap();
        assert_eq!(consumption.today_spend, 0.0); 
        assert!(consumption.month_since_tracking);

        cleanup_test_account(account_key); 
    }

    #[test]
    fn record_consumption_filters_old_snapshots() {
        
        let now = ts_local(2026, 6, 29, 12, 0);
        let old = now - RETENTION_MS - 1000; 
        let snapshots = vec![(old, 100.0), (now - 86400000, 90.0), (now, 70.0)];
        
        let consumption = compute_consumption(&snapshots, now);
        
        
        
        
        assert_eq!(consumption.today_spend, 20.0);
        assert_eq!(consumption.month_spend, 30.0);
    }
}
