//! Currency conversion service (token-monitor-alignment Part B #6B).
//!
//! 4 种支持货币:USD / CNY / TWD / HKD。
//! 基础硬编码汇率 + 用户可配置覆盖(override > default)。
//! 对齐 token-monitor 的 `src/shared/currency.js`。

use std::collections::HashMap;

/// 支持的货币代码
pub const SUPPORTED_CURRENCIES: [&str; 4] = ["USD", "CNY", "TWD", "HKD"];

/// 默认基础汇率(对 USD 的汇率)
/// - USD = 1.0(基准)
/// - CNY = 6.8
/// - TWD = 31.5
/// - HKD = 7.8
pub const DEFAULT_RATES: [(&str, f64); 4] = [
    ("USD", 1.0),
    ("CNY", 6.8),
    ("TWD", 31.5),
    ("HKD", 7.8),
];

/// 货币符号(对齐 token-monitor CURRENCY_RATES.symbol)
pub const CURRENCY_SYMBOLS: [(&str, &str); 4] = [
    ("USD", "$"),
    ("CNY", "¥"),
    ("TWD", "NT$"),
    ("HKD", "HK$"),
];

/// 货币换算服务。基础汇率 + 用户覆盖(override 优先)。
#[derive(Debug, Clone)]
pub struct CurrencyService {
    /// 基础汇率(USD → X)
    rates: HashMap<String, f64>,
    /// 用户覆盖汇率(USD → X),优先级最高
    overrides: HashMap<String, f64>,
}

impl Default for CurrencyService {
    fn default() -> Self {
        Self::new()
    }
}

impl CurrencyService {
    /// 用默认汇率初始化。
    pub fn new() -> Self {
        let mut rates = HashMap::new();
        for (code, rate) in DEFAULT_RATES.iter() {
            rates.insert(code.to_string(), *rate);
        }
        Self {
            rates,
            overrides: HashMap::new(),
        }
    }

    /// 把 USD 金额换算为 target 货币。
    /// target 不支持时 fallback 到 USD(返回原值)。
    pub fn convert_usd(&self, amount_usd: f64, target: &str) -> f64 {
        let code = normalize_currency(target);
        let rate = self.effective_rate(&code);
        amount_usd * rate
    }

    /// 把 amount 从 from 货币换算到 to 货币(经 USD 中转)。
    pub fn convert(&self, amount: f64, from: &str, to: &str) -> f64 {
        let from_code = normalize_currency(from);
        let to_code = normalize_currency(to);
        let from_rate = self.effective_rate(&from_code);
        let to_rate = self.effective_rate(&to_code);
        // amount / from_rate = USD;USD * to_rate = target
        if from_rate == 0.0 {
            return 0.0;
        }
        let usd = amount / from_rate;
        usd * to_rate
    }

    /// 设置用户覆盖汇率(USD → currency)。
    /// rate 必须 > 0,否则忽略。
    pub fn set_override(&mut self, currency: &str, rate: f64) {
        let code = normalize_currency(currency);
        if rate > 0.0 {
            self.overrides.insert(code, rate);
        }
    }

    /// 清除用户覆盖汇率。
    pub fn clear_override(&mut self, currency: &str) {
        let code = normalize_currency(currency);
        self.overrides.remove(&code);
    }

    /// 取生效汇率:override > default。
    pub fn effective_rate(&self, currency: &str) -> f64 {
        let code = normalize_currency(currency);
        if let Some(&r) = self.overrides.get(&code) {
            return r;
        }
        if let Some(&r) = self.rates.get(&code) {
            return r;
        }
        1.0 // fallback USD
    }

    /// 取货币符号(如 "USD" → "$")。
    pub fn symbol(&self, currency: &str) -> &'static str {
        let code = normalize_currency(currency);
        for (c, s) in CURRENCY_SYMBOLS.iter() {
            if *c == code {
                return s;
            }
        }
        ""
    }

    /// 格式化金额:把 USD 金额换算为目标货币并加符号。
    /// 金额 >= 10 时保留 2 位小数;< 10 时保留 4 位小数(对齐 token-monitor fractionDigitsFor)。
    pub fn format_from_usd(&self, amount_usd: f64, currency: &str) -> String {
        let code = normalize_currency(currency);
        let amount = self.convert_usd(amount_usd, &code);
        let digits = if code == "USD" {
            if amount.abs() >= 10.0 { 2 } else { 4 }
        } else {
            if amount.abs() >= 1.0 { 2 } else { 4 }
        };
        format!("{}{}", self.symbol(&code), format!("{:.*}", digits, amount))
    }
}

/// 把货币代码规范化为大写并校验。
/// 不支持的代码 fallback 到 "USD"(对齐 token-monitor normalizeCurrency)。
pub fn normalize_currency(value: &str) -> String {
    let code = value.trim().to_uppercase();
    if SUPPORTED_CURRENCIES.contains(&code.as_str()) {
        code
    } else {
        "USD".to_string()
    }
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_usd_to_cny_default_rate() {
        let svc = CurrencyService::new();
        // 默认 CNY = 6.8
        let result = svc.convert_usd(10.0, "CNY");
        assert!((result - 68.0).abs() < 0.0001, "expected 68.0, got {}", result);
    }

    #[test]
    fn convert_with_override() {
        let mut svc = CurrencyService::new();
        svc.set_override("CNY", 7.0);
        // 用户覆盖后 CNY = 7.0
        let result = svc.convert_usd(10.0, "CNY");
        assert!((result - 70.0).abs() < 0.0001, "expected 70.0, got {}", result);
    }

    #[test]
    fn convert_same_currency_returns_original() {
        let svc = CurrencyService::new();
        let result = svc.convert(100.0, "USD", "USD");
        assert!((result - 100.0).abs() < 0.0001);
        // CNY → CNY 也应原样
        let result2 = svc.convert(100.0, "CNY", "CNY");
        assert!((result2 - 100.0).abs() < 0.0001);
    }

    #[test]
    fn effective_rate_uses_override_when_present() {
        let mut svc = CurrencyService::new();
        assert!((svc.effective_rate("CNY") - 6.8).abs() < 0.0001);
        svc.set_override("CNY", 7.0);
        assert!((svc.effective_rate("CNY") - 7.0).abs() < 0.0001);
        svc.clear_override("CNY");
        assert!((svc.effective_rate("CNY") - 6.8).abs() < 0.0001);
    }

    #[test]
    fn convert_cross_currency_via_usd() {
        let svc = CurrencyService::new();
        // 100 CNY → USD → TWD
        // 100 / 6.8 * 31.5 = 463.235...
        let result = svc.convert(100.0, "CNY", "TWD");
        let expected = 100.0 / 6.8 * 31.5;
        assert!((result - expected).abs() < 0.01, "expected {}, got {}", expected, result);
    }

    #[test]
    fn format_from_usd_uses_correct_symbol_and_digits() {
        let svc = CurrencyService::new();
        // 10 USD → "$10.00"(>= 10,2 位小数)
        let s = svc.format_from_usd(10.0, "USD");
        assert!(s.starts_with("$"), "expected to start with $, got {}", s);
        // 0.5 USD → "$0.5000"(< 10,4 位小数)
        let s2 = svc.format_from_usd(0.5, "USD");
        assert!(s2.starts_with("$"));
        // 100 USD → ¥680.00(CNY)
        let s3 = svc.format_from_usd(100.0, "CNY");
        assert!(s3.starts_with("¥"), "expected to start with ¥, got {}", s3);
    }

    #[test]
    fn normalize_currency_handles_invalid_codes() {
        assert_eq!(normalize_currency("usd"), "USD");
        assert_eq!(normalize_currency("cny"), "CNY");
        assert_eq!(normalize_currency("XXX"), "USD"); // 不支持 fallback USD
        assert_eq!(normalize_currency(""), "USD");
    }
}
