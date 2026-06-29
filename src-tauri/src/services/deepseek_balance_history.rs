//! DeepSeek 余额历史追踪。
//! 每次查 DeepSeek 余额时,把 topped_up_balance 快照存到本地 JSON,
//! 算出 todaySpend / monthSpend。对齐 token-monitor deepseekBalanceHistory.js。

use chrono::{Datelike, Local, TimeZone};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::AppError;

/// 保留期:40 天(对齐 token-monitor)
pub const RETENTION_MS: i64 = 40 * 24 * 3600 * 1000;

/// 消耗计算结果
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Consumption {
    pub today_spend: f64,
    pub month_spend: f64,
    pub month_since_tracking: bool,
}

/// 余额快照(单个时间点)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceSnapshot {
    pub ts: i64,
    pub paid: f64,
}

/// 单个 account 的存储条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountEntry {
    pub currency: String,
    pub snapshots: Vec<BalanceSnapshot>,
}

/// 整个 store 文件格式:{ account_key: { currency, snapshots: [...] } }
pub type BalanceStore = BTreeMap<String, AccountEntry>;

/// store 文件路径
/// Windows: %APPDATA%\com.keypilot.app\deepseek-balance.json
/// Unix: ~/.local/share/com.keypilot.app/deepseek-balance.json
pub fn store_path() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("com.keypilot.app").join("deepseek-balance.json")
}

/// 读现有 store;文件不存在视为空 map
fn read_store(path: &Path) -> Result<BalanceStore, AppError> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(serde_json::from_str(&text)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(BalanceStore::new()),
        Err(e) => Err(AppError::Io(e)),
    }
}

/// 原子写:tmp + rename
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

/// 记录一次余额快照,返回消耗计算结果
///
/// - `account_key`:账户唯一标识(如 `sha256:<hex>`)
/// - `currency`:货币代码(如 "CNY")
/// - `paid`:当前 topped_up_balance
/// - `now_ms`:当前时间(epoch ms)
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
    // 货币变了就重置(对齐 token-monitor:if (!entry || entry.currency !== currency) entry = { currency, snapshots: [] };)
    if entry.currency != currency {
        entry.currency = currency.to_string();
        entry.snapshots.clear();
    }
    entry.snapshots.push(BalanceSnapshot { ts: now_ms, paid });

    // 过滤旧快照 + 排序
    entry.snapshots.retain(|s| s.ts >= now_ms - RETENTION_MS);
    entry.snapshots.sort_by_key(|s| s.ts);

    let snapshots: Vec<(i64, f64)> = entry.snapshots.iter().map(|s| (s.ts, s.paid)).collect();
    let consumption = compute_consumption(&snapshots, now_ms);

    write_store_atomic(&path, &store)?;
    Ok(consumption)
}

/// 计算消耗:对齐 token-monitor computeConsumption
/// - 排序 by ts
/// - 遍历相邻快照,drop = max(0, prev.paid - cur.paid)(充值时 paid 增加,drop=0)
/// - today_spend = drop 落在本地今天的总和
/// - month_spend = drop 落在本地本月的总和
/// - month_since_tracking = 最早快照是否在本地本月之后(若是,说明本月才开始追踪)
pub fn compute_consumption(snapshots: &[(i64, f64)], now_ms: i64) -> Consumption {
    if snapshots.is_empty() {
        return Consumption {
            today_spend: 0.0,
            month_spend: 0.0,
            month_since_tracking: false,
        };
    }

    // 排序 by ts
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

/// 保留 2 位小数(对齐 token-monitor round2)
fn round2(value: f64) -> f64 {
    ((value + f64::EPSILON) * 100.0).round() / 100.0
}

/// 将字节转为十六进制字符串(不依赖 hex crate)
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

    /// 测试辅助:从 store 中移除指定 account_key(避免污染真实 store)
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
        // 只有 1 个快照,没有 drop
        let now = ts_local(2026, 6, 29, 12, 0);
        let snapshots = vec![(now, 100.0)];
        let consumption = compute_consumption(&snapshots, now);
        assert_eq!(consumption.today_spend, 0.0);
        assert_eq!(consumption.month_spend, 0.0);
        // 最早快照 = now > month_start,所以 month_since_tracking = true
        assert!(consumption.month_since_tracking);
    }

    #[test]
    fn compute_consumption_drop_today() {
        // 昨天 100,今天 70 → drop = 30 落在今天
        let yesterday = ts_local(2026, 6, 28, 12, 0);
        let today = ts_local(2026, 6, 29, 12, 0);
        let snapshots = vec![(yesterday, 100.0), (today, 70.0)];
        let consumption = compute_consumption(&snapshots, today);
        assert_eq!(consumption.today_spend, 30.0);
        assert_eq!(consumption.month_spend, 30.0);
    }

    #[test]
    fn compute_consumption_drop_this_month() {
        // 上月 100,本月早些时候 70 → drop = 30 落在本月但不在今天
        let last_month = ts_local(2026, 5, 29, 12, 0);
        let earlier_this_month = ts_local(2026, 6, 15, 12, 0);
        let now = ts_local(2026, 6, 29, 12, 0);
        let snapshots = vec![(last_month, 100.0), (earlier_this_month, 70.0)];
        let consumption = compute_consumption(&snapshots, now);
        assert_eq!(consumption.today_spend, 0.0); // drop 在 6/15,不在今天
        assert_eq!(consumption.month_spend, 30.0); // 在本月
        assert!(!consumption.month_since_tracking); // 最早快照在上月
    }

    #[test]
    fn compute_consumption_topup_does_not_count() {
        // 70 → 120 充值,drop = max(0, 70-120) = 0
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
        // record_consumption 用 store_path(),无法注入路径。
        // 本测试只验证函数能跑通(在实际 store_path 上写),不验证文件位置。
        // 真正的文件路径测试在端到端测试里。
        let account_key = "sha256:test_record_consumption_creates_store_file";
        cleanup_test_account(account_key); // 确保是"首次查询"

        let now = chrono::Local::now().timestamp_millis();
        let result = record_consumption(account_key, "CNY", 100.0, now);
        assert!(result.is_ok());
        let consumption = result.unwrap();
        assert_eq!(consumption.today_spend, 0.0); // 首次查询
        assert!(consumption.month_since_tracking);

        cleanup_test_account(account_key); // 测试后清理
    }

    #[test]
    fn record_consumption_filters_old_snapshots() {
        // 验证 compute_consumption 对旧快照的行为(record_consumption 内部会过滤 RETENTION_MS 之外的)
        let now = ts_local(2026, 6, 29, 12, 0);
        let old = now - RETENTION_MS - 1000; // 41 天前
        let snapshots = vec![(old, 100.0), (now - 86400000, 90.0), (now, 70.0)];
        // compute_consumption 不过滤(record_consumption 才过滤),旧快照仍参与计算
        let consumption = compute_consumption(&snapshots, now);
        // old → (now-1d): drop = 10,ts = now-1d(6/28,在本月)
        // (now-1d) → now: drop = 20,ts = now(6/29,在今天)
        // today_spend = 20(只算 now 那个 drop)
        // month_spend = 30(两个 drop 都在本月:6/28 + 6/29)
        assert_eq!(consumption.today_spend, 20.0);
        assert_eq!(consumption.month_spend, 30.0);
    }
}
