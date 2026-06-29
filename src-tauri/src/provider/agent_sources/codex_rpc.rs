//! Codex RPC 余额查询 source。
//!
//! Task 4 实现:发现 codex 命令 → spawn `codex -s read-only -a untrusted app-server`
//! → JSON-RPC `account/rateLimits/read` → 解析 primary(session)+ secondary(weekly)窗口
//! → 读 `~/.codex/auth.json` 解 JWT 拿 email + plan_label。
//!
//! 参考:token-monitor `limitCollector.js`(codexCommandCandidates / codexSpawnSpec /
//! spawnCodexAppServer / createJsonRpcClient / readCodexRpc)+ `codexAuth.js`。
//!
//! 实现说明:用 `std::process::Command`(同步 spawn)+ `tokio::task::spawn_blocking`
//! 包裹阻塞 IO,避免依赖 tokio 的 `process` feature(在 test 构建中 Tauri 的 dev-feature
//! 不启用它)。12s 超时通过 `std::sync::mpsc::recv_timeout` + reader 线程实现。
//! 子进程在所有路径上显式 `kill()` + `wait()`(对齐 spec 的 `kill_on_drop` 意图)。

use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::process::Command;

use crate::provider::adapter::QuotaError;
use crate::provider::agent_source::AgentBalanceSource;
use crate::types::{LimitSource, LimitStatus, LimitWindow, LimitWindowKind, QuotaSnapshot};

pub struct CodexRpcSource;

impl CodexRpcSource {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CodexRpcSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentBalanceSource for CodexRpcSource {
    fn agent_type(&self) -> &'static str {
        "codex"
    }

    fn display_name(&self) -> &'static str {
        "Codex"
    }

    fn is_available(&self) -> bool {
        // auth.json 存在 = 强信号
        if codex_auth_path().exists() {
            return true;
        }
        // 绝对路径候选存在 = 强信号;裸命令名(如 "codex")无法判断,保守返回 false
        codex_command_candidates()
            .iter()
            .any(|c| std::path::Path::new(c).is_absolute() && std::path::Path::new(c).exists())
    }

    async fn fetch_balance(&self) -> Result<QuotaSnapshot, QuotaError> {
        // 1. 找 codex 命令(同步,无 IO,直接在当前上下文执行)
        let candidates = codex_command_candidates();
        let cmd = candidates.iter().find(|c| {
            if std::path::Path::new(c).exists() {
                return true;
            }
            // 裸命令名(如 "codex"/"codex.cmd"/"codex.exe")无法直接判断,留给 spawn 处理
            !c.contains('/') && !c.contains('\\')
        });
        let cmd = match cmd {
            Some(c) => c.clone(),
            None => return Ok(not_configured_snapshot()),
        };

        // 2. spawn + RPC 是阻塞 IO,放到 spawn_blocking 里跑(避免占用 tokio runtime)
        let cmd_for_blocking = cmd.clone();
        let join_result = tokio::task::spawn_blocking(move || fetch_balance_blocking(&cmd_for_blocking)).await;
        match join_result {
            Ok(snapshot) => snapshot,
            Err(join_err) => Ok(unavailable_snapshot(format!(
                "spawn_blocking join error: {:?}",
                join_err
            ))),
        }
    }
}

/// `fetch_balance` 的阻塞实现(spawn + JSON-RPC + 解析)。
/// 在 `tokio::task::spawn_blocking` 里跑,不占用 tokio runtime 的 worker 线程。
fn fetch_balance_blocking(cmd: &str) -> Result<QuotaSnapshot, QuotaError> {
    // 1. spawn app-server
    let mut child = match spawn_codex_app_server(cmd) {
        Ok(c) => c,
        Err(e) => return Ok(unavailable_snapshot(format!("spawn failed: {}", e))),
    };

    // 拿 stdin / stdout
    let mut stdin = match child.stdin.take() {
        Some(s) => s,
        None => {
            kill_child(&mut child);
            return Ok(unavailable_snapshot("no stdin".into()));
        }
    };
    let stdout = match child.stdout.take() {
        Some(s) => s,
        None => {
            kill_child(&mut child);
            return Ok(unavailable_snapshot("no stdout".into()));
        }
    };

    // 启动 reader 线程:把 stdout 按行读出来,通过 mpsc channel 传给 json_rpc_call。
    // 这样 json_rpc_call 可以用 `recv_timeout` 实现 12s 超时,避免 `read_line` 永久阻塞。
    let (line_tx, line_rx) = std::sync::mpsc::channel::<String>();
    let reader_handle = std::thread::spawn(move || {
        let reader = std::io::BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    if line_tx.send(l).is_err() {
                        break; // 接收方丢弃,停止读取
                    }
                }
                Err(_) => break,
            }
        }
    });

    let mut next_id: u64 = 0;

    // 2. initialize
    let init_params = serde_json::json!({
        "clientInfo": {"name": "keypilot", "title": "KeyPilot", "version": "0.1"}
    });
    if json_rpc_call(&mut stdin, &line_rx, "initialize", Some(init_params), &mut next_id).is_err() {
        kill_child(&mut child);
        return Ok(unavailable_snapshot("initialize failed".into()));
    }

    // 3. initialized 通知(无 id,无响应)
    let notif = serde_json::json!({"method": "initialized", "params": {}});
    let notif_line = format!("{}\n", serde_json::to_string(&notif).unwrap_or_default());
    let _ = stdin.write_all(notif_line.as_bytes());
    let _ = stdin.flush();

    // 4. account/rateLimits/read
    let rate_result = match json_rpc_call(
        &mut stdin,
        &line_rx,
        "account/rateLimits/read",
        None,
        &mut next_id,
    ) {
        Ok(v) => v,
        Err(e) => {
            kill_child(&mut child);
            return Ok(unavailable_snapshot(format!("rateLimits failed: {:?}", e)));
        }
    };

    // 5. kill 子进程 + 等 reader 线程退出
    //    kill 后 child 的 stdout 管道关闭,reader 线程的 lines() 迭代器收到 EOF 退出
    kill_child(&mut child);
    drop(stdin);
    let _ = reader_handle.join();

    // 6. 解析 rateLimits
    let rate_limits = rate_result
        .get("rateLimits")
        .or_else(|| rate_result.get("rate_limits"))
        .or_else(|| {
            rate_result
                .get("rateLimitsByLimitId")
                .and_then(|v| v.get("codex"))
        })
        .or_else(|| {
            rate_result
                .get("rate_limits_by_limit_id")
                .and_then(|v| v.get("codex"))
        })
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    let mut windows = Vec::new();
    if let Some(primary) = rate_limits.get("primary") {
        if let Some(w) = build_window(primary, LimitWindowKind::Session, "Session") {
            windows.push(w);
        }
    }
    if let Some(secondary) = rate_limits.get("secondary") {
        if let Some(w) = build_window(secondary, LimitWindowKind::Weekly, "Weekly") {
            windows.push(w);
        }
    }

    // 7. 读 auth.json 拿 email + plan_label(失败不致命,fallback 空字符串)
    let identity = read_codex_auth_identity();
    let plan_label = codex_plan_label(&identity.plan_type);
    let source_detail = codex_command_source_detail(cmd);

    // 先算 status(避免 windows 在 struct literal 里 move 后再 borrow)
    let status = if windows.is_empty() {
        LimitStatus::NotConfigured
    } else {
        LimitStatus::Ok
    };

    Ok(QuotaSnapshot {
        total: None,
        used: 0.0,
        remaining: None,
        unit: String::new(),
        level: None,
        reset_at: None,
        windows,
        status,
        source: LimitSource::Rpc,
        source_detail: source_detail.to_string(),
        account_label: if plan_label.is_empty() {
            None
        } else {
            Some(plan_label)
        },
        account_email: if identity.email.is_empty() {
            None
        } else {
            Some(identity.email)
        },
        region: None,
        balance: None,
        used_amount: None,
        balance_usd: None,
        used_usd: None,
    })
}

/// kill 子进程并 wait 回收(对齐 spec 的 `kill_on_drop` 意图:不泄漏子进程)。
fn kill_child(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

// === 命令发现 ===

/// 返回 codex 命令候选列表(对齐 token-monitor `codexCommandCandidates`)。
/// 实际存在性由调用方(或 spawn 时的 `io::Error`)判断。
fn codex_command_candidates() -> Vec<String> {
    if let Ok(cmd) = std::env::var("TOKEN_MONITOR_CODEX_COMMAND") {
        let trimmed = cmd.trim();
        if !trimmed.is_empty() {
            return vec![cmd];
        }
    }

    let mut raw: Vec<String> = Vec::new();

    if cfg!(target_os = "macos") {
        raw.push("/Applications/Codex.app/Contents/Resources/codex".to_string());
    }

    if cfg!(windows) {
        if let Ok(lad) = std::env::var("LOCALAPPDATA") {
            raw.push(format!("{}\\Programs\\Codex\\resources\\codex.exe", lad));
        }
        if let Ok(pf) = std::env::var("PROGRAMFILES") {
            raw.push(format!("{}\\Codex\\resources\\codex.exe", pf));
        }
        if let Ok(appdata) = std::env::var("APPDATA") {
            raw.push(format!("{}\\npm\\codex.cmd", appdata));
        }
        raw.push("codex.cmd".to_string());
        raw.push("codex.exe".to_string());
    }

    if cfg!(target_os = "linux")
        || cfg!(target_os = "freebsd")
        || cfg!(target_os = "openbsd")
        || cfg!(target_os = "netbsd")
    {
        raw.push("/usr/local/bin/codex".to_string());
    }

    // 通用候选
    raw.push("codex".to_string());

    // dedup preserving order
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for c in raw {
        if seen.insert(c.clone()) {
            out.push(c);
        }
    }
    out
}

/// 分类命令来源(对齐 token-monitor `codexCommandSourceDetail`)。
/// 返回 "app"(Codex.app / Windows Store 包)/ "cli"(npm / 裸命令)/ "unknown"。
fn codex_command_source_detail(cmd: &str) -> &'static str {
    let raw = cmd.trim();
    if raw.is_empty() {
        return "unknown";
    }
    let normalized = raw.replace('\\', "/").to_lowercase();
    if normalized.contains("/codex.app/") {
        return "app";
    }
    if cfg!(windows) {
        if normalized.contains("/programs/codex/")
            || normalized.contains("/openai/codex/bin/")
            || normalized.contains("/packages/openai.codex_")
            || normalized.contains("/windowsapps/openai.codex_")
            || normalized.contains("/microsoft/windowsapps/")
        {
            return "app";
        }
        if normalized == "codex"
            || normalized == "codex.cmd"
            || normalized == "codex.exe"
            || normalized.contains("/npm/codex.cmd")
            || normalized.contains("/node_modules/@openai/codex/")
            || normalized.contains("/.bun/bin/codex.exe")
        {
            return "cli";
        }
    }
    if normalized.ends_with("/codex")
        || normalized.ends_with("/codex.cmd")
        || normalized.ends_with("/codex.exe")
        || normalized == "codex"
    {
        return "cli";
    }
    "unknown"
}

// === auth.json 读取 + JWT 解码 ===

/// `$CODEX_HOME/auth.json` 或 `~/.codex/auth.json`(对齐 token-monitor `codexAuthPath`)。
fn codex_auth_path() -> PathBuf {
    if let Ok(home) = std::env::var("CODEX_HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(home).join("auth.json");
        }
    }
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".codex").join("auth.json")
}

#[derive(Default, Debug, Clone)]
struct CodexIdentity {
    email: String,
    plan_type: String,
    // spec 要求提取 chatgpt_account_id(为 V0.2 token 统计 / account_key 预留),
    // 当前 QuotaSnapshot 没有对应字段,保留字段以表明已提取,显式抑制 dead_code 警告。
    #[allow(dead_code)]
    account_id: String,
}

/// 读 `~/.codex/auth.json`,解码 `tokens.id_token` JWT payload,
/// 提取 `email` / `chatgpt_plan_type` / `chatgpt_account_id`。
/// 文件不存在或解析失败时返回空 `CodexIdentity`,不 panic。
/// 对齐 token-monitor `codexAuth.js::codexAuthIdentity`。
fn read_codex_auth_identity() -> CodexIdentity {
    let path = codex_auth_path();
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return CodexIdentity::default(),
    };
    let v: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => return CodexIdentity::default(),
    };
    let tokens = v.get("tokens").unwrap_or(&v);
    let id_token = tokens
        .get("id_token")
        .or_else(|| v.get("id_token"))
        .and_then(|s| s.as_str())
        .unwrap_or("");
    if id_token.is_empty() {
        return CodexIdentity::default();
    }

    let payload = decode_jwt_payload(id_token);
    let nested = payload
        .get("https://api.openai.com/auth")
        .or_else(|| payload.get("https://api.openai.com/profile"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    let email = payload
        .get("email")
        .or_else(|| nested.get("email"))
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_lowercase();
    let plan_type = payload
        .get("chatgpt_plan_type")
        .or_else(|| nested.get("chatgpt_plan_type"))
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();
    let account_id = payload
        .get("chatgpt_account_id")
        .or_else(|| nested.get("chatgpt_account_id"))
        .or_else(|| payload.get("sub"))
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();

    CodexIdentity {
        email,
        plan_type,
        account_id,
    }
}

/// 解码 JWT 的 base64url payload 段(不验签)。
/// JWT = `header.payload.signature`,取第二段 base64url decode → JSON。
fn decode_jwt_payload(token: &str) -> serde_json::Value {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() < 2 || parts[1].is_empty() {
        return serde_json::Value::Null;
    }
    // base64url(JWT payload 用 base64url 无 padding;兼容带 padding 的输入)
    let input = parts[1].trim_end_matches('=');
    let decoded = match URL_SAFE_NO_PAD.decode(input) {
        Ok(d) => d,
        Err(_) => return serde_json::Value::Null,
    };
    let json_str = match std::str::from_utf8(&decoded) {
        Ok(s) => s,
        Err(_) => return serde_json::Value::Null,
    };
    serde_json::from_str(json_str).unwrap_or(serde_json::Value::Null)
}

// === plan label ===

/// 把 `chatgpt_plan_type` 映射为显示标签(对齐 token-monitor `codexPlanLabelFromParts`)。
/// - `pro` → "Pro 20x"
/// - `prolite` / `pro_lite` / `pro-lite` / `pro lite` → "Pro 5x"
/// - `free` → "Free" / `plus` → "Plus" / `max` → "Max"
/// - `team` / `teams` → "Team" / `enterprise` → "Enterprise"
fn codex_plan_label(plan_type: &str) -> String {
    let raw = plan_type.trim();
    if raw.is_empty() || raw.contains('@') {
        return String::new();
    }
    let lower = raw.to_lowercase();
    match lower.as_str() {
        "pro" => return "Pro 20x".to_string(),
        "prolite" | "pro_lite" | "pro-lite" | "pro lite" => return "Pro 5x".to_string(),
        "free" => return "Free".to_string(),
        "plus" => return "Plus".to_string(),
        "max" => return "Max".to_string(),
        "team" | "teams" => return "Team".to_string(),
        "enterprise" => return "Enterprise".to_string(),
        _ => {}
    }
    // fallback:clean prefix + capitalize
    let cleaned = lower
        .trim_start_matches("codex ")
        .trim_start_matches("chatgpt ")
        .trim_start_matches("openai ")
        .replace('_', " ")
        .replace('-', " ");
    let words: Vec<&str> = cleaned.split_whitespace().collect();
    if words.is_empty() {
        return String::new();
    }
    let capitalized: Vec<String> = words
        .iter()
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect();
    capitalized.join(" ")
}

// === spawn app-server ===

/// spawn `codex -s read-only -a untrusted app-server`(对齐 token-monitor `spawnCodexAppServer` +
/// `codexSpawnSpec`)。
///
/// Windows `.cmd` 文件走 `cmd.exe /d /s /c "<cmd> -s read-only -a untrusted app-server"` 包裹;
/// 否则直接 spawn。子进程 kill 由调用方(`fetch_balance_blocking`)在所有路径上显式调用
/// `kill_child`(对齐 spec 的 `kill_on_drop(true)` 意图)。
fn spawn_codex_app_server(cmd: &str) -> std::io::Result<std::process::Child> {
    let args = ["-s", "read-only", "-a", "untrusted", "app-server"];

    let mut command = if cfg!(windows) && cmd.to_lowercase().ends_with(".cmd") {
        let mut c = Command::new("cmd.exe");
        let joined = format!("{} {}", cmd, args.join(" "));
        c.arg("/d").arg("/s").arg("/c").arg(joined);
        c
    } else {
        let mut c = Command::new(cmd);
        for a in args {
            c.arg(a);
        }
        c
    };

    command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    // env hints(对齐 `withCodexPathHints`):把常见 bin 目录 prepend 到 PATH,
    // 让 codex 命令(无论全局安装还是 per-user)能被找到
    if cfg!(windows) {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let npm_path = format!("{}\\npm", appdata);
            let current_path = std::env::var("PATH").unwrap_or_default();
            command.env("PATH", format!("{};{}", npm_path, current_path));
        }
    } else {
        let hints = ["/opt/homebrew/bin", "/usr/local/bin"];
        let current_path = std::env::var("PATH").unwrap_or_default();
        let mut new_path = hints.join(":");
        new_path.push(':');
        new_path.push_str(&current_path);
        command.env("PATH", new_path);
    }

    command.spawn()
}

// === JSON-RPC ===

/// 行分隔 JSON-RPC:写 `{method, id, params}\n` 到 stdin → 从 `line_rx` 读行匹配 `id` → 返回 `result`。
/// 12s 超时通过 `mpsc::recv_timeout` 实现(对齐 token-monitor `createJsonRpcClient`)。
///
/// `line_rx` 由 reader 线程从子进程 stdout 按行读取后发送,避免 `read_line` 永久阻塞。
fn json_rpc_call(
    stdin: &mut impl Write,
    line_rx: &std::sync::mpsc::Receiver<String>,
    method: &str,
    params: Option<serde_json::Value>,
    next_id: &mut u64,
) -> Result<serde_json::Value, QuotaError> {
    *next_id += 1;
    let id = *next_id;
    let msg = match params {
        Some(p) => serde_json::json!({"method": method, "id": id, "params": p}),
        None => serde_json::json!({"method": method, "id": id}),
    };
    let line = format!(
        "{}\n",
        serde_json::to_string(&msg).map_err(|e| QuotaError::Parse(e.to_string()))?
    );
    stdin
        .write_all(line.as_bytes())
        .map_err(|e| QuotaError::Network(e.to_string()))?;
    stdin
        .flush()
        .map_err(|e| QuotaError::Network(e.to_string()))?;

    // 读 stdout 等待匹配 id 的响应,12s 超时
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(12);
    loop {
        let now = std::time::Instant::now();
        if now >= deadline {
            return Err(QuotaError::Network(format!("{} timed out", method)));
        }
        let remaining = deadline - now;
        match line_rx.recv_timeout(remaining) {
            Ok(raw_line) => {
                let trimmed = raw_line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let v: serde_json::Value = match serde_json::from_str(trimmed) {
                    Ok(v) => v,
                    Err(_) => continue, // 跳过非 JSON 行(如通知 / 日志)
                };
                if v.get("id").and_then(|i| i.as_u64()) == Some(id) {
                    if let Some(err) = v.get("error") {
                        return Err(QuotaError::Network(format!("rpc error: {:?}", err)));
                    }
                    return Ok(v.get("result").cloned().unwrap_or(serde_json::Value::Null));
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                return Err(QuotaError::Network(format!("{} timed out", method)));
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                return Err(QuotaError::Network(
                    "stdout reader disconnected".to_string(),
                ));
            }
        }
    }
}

// === 辅助:窗口构造 + snapshot 工厂 ===

/// 从 rateLimits.primary / secondary JSON 构造 `LimitWindow`。
/// 字段名兼容 camelCase / snake_case(对齐 token-monitor `mapCodexRateLimitsToProvider`)。
fn build_window(v: &serde_json::Value, kind: LimitWindowKind, label: &str) -> Option<LimitWindow> {
    let used_percent = v
        .get("usedPercent")
        .or_else(|| v.get("used_percent"))
        .and_then(|p| p.as_f64());
    let limit = v
        .get("limit")
        .or_else(|| v.get("limited"))
        .and_then(|p| p.as_f64());
    let used = v
        .get("used")
        .or_else(|| v.get("spent"))
        .and_then(|p| p.as_f64())
        .unwrap_or(0.0);
    let remaining = v.get("remaining").and_then(|p| p.as_f64());
    let resets_at = v
        .get("resetsAt")
        .or_else(|| v.get("resets_at"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());
    // token-monitor 用 windowDurationMins;兼容 window_minutes / windowMinutes
    let window_minutes = v
        .get("windowDurationMins")
        .or_else(|| v.get("window_duration_mins"))
        .or_else(|| v.get("windowMinutes"))
        .or_else(|| v.get("window_minutes"))
        .and_then(|p| p.as_i64());

    // 若无 usedPercent 但有 used + limit,则计算
    let used_percent = used_percent.or_else(|| {
        if let Some(limit_val) = limit {
            if limit_val > 0.0 {
                Some((used / limit_val) * 100.0)
            } else {
                None
            }
        } else {
            None
        }
    });

    Some(LimitWindow {
        kind,
        label: label.to_string(),
        used,
        limit,
        remaining,
        used_percent,
        remaining_percent: used_percent.map(|p| (100.0 - p).max(0.0)),
        resets_at,
        window_minutes,
        reset_description: String::new(),
        show_meter: true,
    })
}

fn not_configured_snapshot() -> QuotaSnapshot {
    QuotaSnapshot {
        total: None,
        used: 0.0,
        remaining: None,
        unit: String::new(),
        level: None,
        reset_at: None,
        windows: Vec::new(),
        status: LimitStatus::NotConfigured,
        source: LimitSource::Rpc,
        source_detail: "unknown".to_string(),
        account_label: None,
        account_email: None,
        region: None,
        balance: None,
        used_amount: None,
        balance_usd: None,
        used_usd: None,
    }
}

fn unavailable_snapshot(detail: String) -> QuotaSnapshot {
    QuotaSnapshot {
        total: None,
        used: 0.0,
        remaining: None,
        unit: String::new(),
        level: None,
        reset_at: None,
        windows: Vec::new(),
        status: LimitStatus::Unavailable,
        source: LimitSource::Rpc,
        source_detail: detail,
        account_label: None,
        account_email: None,
        region: None,
        balance: None,
        used_amount: None,
        balance_usd: None,
        used_usd: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // 序列化所有修改 TOKEN_MONITOR_CODEX_COMMAND 的测试,避免并行 race condition
    // (一个测试 remove_var 会让另一个测试的 fetch_balance 落到默认候选 "codex",
    //  如果机器上装了 codex 会 spawn 成功但 JSON-RPC 超时 → Unavailable 而非 NotConfigured)
    static CODEX_CMD_ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn codex_command_candidates_returns_non_empty() {
        let candidates = codex_command_candidates();
        // 至少包含 "codex" 通用候选
        assert!(candidates.iter().any(|c| {
            c == "codex" || c.ends_with("codex") || c.ends_with("codex.exe") || c.ends_with("codex.cmd")
        }));
    }

    #[test]
    fn codex_command_candidates_respects_env_override() {
        let _guard = CODEX_CMD_ENV_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!(
            "keypilot-test-codex-env-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_millis()
        ));
        std::env::set_var("TOKEN_MONITOR_CODEX_COMMAND", &tmp);
        let candidates = codex_command_candidates();
        std::env::remove_var("TOKEN_MONITOR_CODEX_COMMAND");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0], tmp.to_string_lossy().to_string());
    }

    #[test]
    fn codex_command_source_detail_classifies_app_and_cli() {
        assert_eq!(
            codex_command_source_detail("/Applications/Codex.app/Contents/Resources/codex"),
            "app"
        );
        assert_eq!(codex_command_source_detail("codex.cmd"), "cli");
        assert_eq!(codex_command_source_detail("codex"), "cli");
        assert_eq!(codex_command_source_detail("codex.exe"), "cli");
        assert_eq!(codex_command_source_detail("/usr/local/bin/codex"), "cli");
        assert_eq!(codex_command_source_detail(""), "unknown");
    }

    #[test]
    fn codex_plan_label_maps_aliases() {
        assert_eq!(codex_plan_label("pro"), "Pro 20x");
        assert_eq!(codex_plan_label("prolite"), "Pro 5x");
        assert_eq!(codex_plan_label("pro_lite"), "Pro 5x");
        assert_eq!(codex_plan_label("pro-lite"), "Pro 5x");
        assert_eq!(codex_plan_label("free"), "Free");
        assert_eq!(codex_plan_label("plus"), "Plus");
        assert_eq!(codex_plan_label("max"), "Max");
        assert_eq!(codex_plan_label("team"), "Team");
        assert_eq!(codex_plan_label("teams"), "Team");
        assert_eq!(codex_plan_label("enterprise"), "Enterprise");
        assert_eq!(codex_plan_label(""), "");
        assert_eq!(codex_plan_label("user@example.com"), "");
    }

    #[test]
    fn read_codex_auth_identity_returns_empty_when_no_file() {
        // 用 CODEX_HOME 指向不存在的目录
        let tmp = std::env::temp_dir().join(format!(
            "keypilot-test-codex-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_millis()
        ));
        std::env::set_var("CODEX_HOME", &tmp);
        let identity = read_codex_auth_identity();
        std::env::remove_var("CODEX_HOME");
        assert!(identity.email.is_empty());
        assert!(identity.plan_type.is_empty());
        assert!(identity.account_id.is_empty());
    }

    #[test]
    fn decode_jwt_payload_parses_simple_jwt() {
        // JWT: header.payload.signature
        // payload: {"email":"a@b.com","chatgpt_plan_type":"pro"} → base64url
        let payload_json = r#"{"email":"a@b.com","chatgpt_plan_type":"pro"}"#;
        let payload_b64 = URL_SAFE_NO_PAD.encode(payload_json.as_bytes());
        let jwt = format!(
            "{}.{}.signature",
            URL_SAFE_NO_PAD.encode(b"{}"),
            payload_b64
        );
        let v = decode_jwt_payload(&jwt);
        assert_eq!(v["email"].as_str().unwrap(), "a@b.com");
        assert_eq!(v["chatgpt_plan_type"].as_str().unwrap(), "pro");
    }

    #[test]
    fn decode_jwt_payload_returns_null_for_invalid() {
        // "not.a.jwt":parts[1]="a",base64url decode 1 char 无效 → Null
        assert!(decode_jwt_payload("not.a.jwt").is_null());
        assert!(decode_jwt_payload("").is_null());
        assert!(decode_jwt_payload("onlyonepart").is_null());
    }

    #[test]
    fn fetch_balance_returns_not_configured_when_no_codex_command() {
        let _guard = CODEX_CMD_ENV_LOCK.lock().unwrap();
        // 设置 TOKEN_MONITOR_CODEX_COMMAND 指向不存在的路径(包含分隔符 → 不走 bare-name 分支)
        let tmp = std::env::temp_dir().join(format!(
            "keypilot-test-no-codex-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_millis()
        ));
        std::env::set_var("TOKEN_MONITOR_CODEX_COMMAND", &tmp);
        let source = CodexRpcSource::new();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = rt.block_on(source.fetch_balance());
        std::env::remove_var("TOKEN_MONITOR_CODEX_COMMAND");
        assert!(result.is_ok());
        let snap = result.unwrap();
        assert_eq!(snap.status, LimitStatus::NotConfigured);
    }
}
