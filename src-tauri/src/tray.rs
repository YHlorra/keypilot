use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{TrayIcon, TrayIconBuilder},
    AppHandle, Manager, Runtime,
};
use crate::store::AppState;
use crate::services::provider::PinnedProviderQuota;
use crate::types::{QuotaSnapshot, LimitStatus};
use crate::types::subscription::SubscriptionQuota;
use crate::timeutil;
use crate::catalog;


const TRAY_ID: &str = "main-tray";

pub fn init_tray<R: Runtime>(app: &AppHandle<R>) -> Result<TrayIcon<R>, tauri::Error> {
    let menu = build_quota_menu(app)?;

    let tray_icon = Image::from_bytes(include_bytes!("../icons/tray.png"))
        .expect("Failed to load tray icon");

    let tray = TrayIconBuilder::with_id(TRAY_ID)
        .icon(tray_icon)
        .tooltip("KeyPilot")
        .menu(&menu)
        .on_menu_event(move |app, event| {
            handle_menu_event(app, event.id.as_ref());
        })
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left,
                ..
            } = event
            {
                toggle_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        refresh_and_rebuild(&app_handle).await;
    });

    Ok(tray)
}

fn toggle_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let visible = window.is_visible().unwrap_or(false);
        if visible {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.unminimize();
            let _ = window.set_focus();
        }
    }
}

pub async fn refresh_and_rebuild<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<AppState>();

    // Read pinned provider meta (id, name, preset, custom_spec)
    let providers: Vec<(i64, String, Option<String>, Option<String>)> = {
        let db = state.db.clone();
        let guard = match db.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        let mut stmt = match guard.conn.prepare(
            "SELECT id, name, preset, custom_spec FROM providers WHERE pinned = 1 ORDER BY sort_index, name"
        ) {
            Ok(s) => s,
            Err(_) => return,
        };
        let rows = match stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, Option<String>>(3)?,
            ))
        }) {
            Ok(r) => r,
            Err(_) => return,
        };
        rows.filter_map(|r| r.ok()).collect()
    };

    // Fetch quota for each provider.
    // Providers with coding_plan (e.g. Minimax) use fetch_coding_plan_quota_by_state,
    // others use fetch_quota_by_state (openai/anthropic/balance/deepseek adapters).
    let mut items: Vec<String> = Vec::new();
    for (id, name, preset, custom_spec_json) in providers {
        let has_coding_plan = has_coding_plan(&preset, &custom_spec_json);
        let line = if has_coding_plan {
            let sub = crate::commands::quota::fetch_coding_plan_quota_by_state(&state, id).await.ok();
            format_coding_plan_line(&name, &sub)
        } else {
            let snapshot = crate::commands::quota::fetch_quota_by_state(&state, id).await.ok();
            let ppq = PinnedProviderQuota { id, name: name.clone(), preset, snapshot, fetched_at: Some(timeutil::now_secs()) };
            format_quota_line(&ppq)
        };
        items.push(line);
    }

    rebuild_menu_from_lines(app, &items);
}

/// Check if a provider has a coding_plan vendor by resolving its catalog entry.
pub fn has_coding_plan(preset: &Option<String>, custom_spec_json: &Option<String>) -> bool {
    let preset = match preset {
        Some(p) => p,
        None => return false,
    };
    let custom_spec = custom_spec_json
        .as_deref()
        .and_then(|s| serde_json::from_str::<catalog::CustomSpec>(s).ok());
    catalog::resolve(preset, custom_spec.as_ref())
        .ok()
        .and_then(|r| r.coding_plan)
        .is_some()
}

/// Format a coding_plan (SubscriptionQuota) result for the tray menu.
fn format_coding_plan_line(name: &str, sub: &Option<SubscriptionQuota>) -> String {
    let Some(sub) = sub else {
        return format!("· {}  未配置", name);
    };

    if !sub.success {
        let msg = sub.error.as_deref().unwrap_or("错误");
        return format!("! {}  {}", name, msg);
    }

    if sub.tiers.is_empty() {
        return format!("✓ {}", name);
    }

    // Use the first tier's remaining_percent
    let tier = &sub.tiers[0];
    let pct = tier.remaining_percent;
    let icon = match pct {
        Some(p) if p > 50.0 => "✓",
        Some(p) if p > 0.0 => "⧗",
        Some(_) => "✗",
        None => "·",
    };

    if let Some(p) = pct {
        format!("{} {}  {:.0}%", icon, name, p)
    } else {
        format!("{} {}", icon, name)
    }
}

pub fn rebuild_menu<R: Runtime>(app: &AppHandle<R>) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        match build_quota_menu(app) {
            Ok(menu) => { let _ = tray.set_menu(Some(menu)); }
            Err(e) => eprintln!("tray rebuild_menu failed: {}", e),
        }
    }
}

fn rebuild_menu_from_lines<R: Runtime>(app: &AppHandle<R>, items: &[String]) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        match build_menu_from_lines(app, items) {
            Ok(menu) => { let _ = tray.set_menu(Some(menu)); }
            Err(e) => eprintln!("tray rebuild_menu_from_lines failed: {}", e),
        }
    }
}

fn build_quota_menu<R: Runtime>(app: &AppHandle<R>) -> Result<Menu<R>, tauri::Error> {
    let items = read_pinned_quota_items_sync(app);
    build_menu_from_lines(app, &items)
}

fn build_menu_from_lines<R: Runtime>(app: &AppHandle<R>, items: &[String]) -> Result<Menu<R>, tauri::Error> {
    let menu = Menu::new(app)?;

    if items.is_empty() {
        menu.append(&MenuItem::with_id(app, "__empty", "· 未钉住凭证", false, None::<&str>)?)?;
    } else {
        for (i, text) in items.iter().enumerate() {
            let id = format!("__q{}", i);
            menu.append(&MenuItem::with_id(app, id, text, false, None::<&str>)?)?;
        }
    }

    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, "refresh_quota", "刷新额度", true, None::<&str>)?)?;
    menu.append(&MenuItem::with_id(app, "open_window", "打开主窗口", true, None::<&str>)?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?)?;

    Ok(menu)
}

fn read_pinned_quota_items_sync<R: Runtime>(app: &AppHandle<R>) -> Vec<String> {
    let state = app.state::<AppState>();
    let db = state.db.clone();
    let guard = match db.lock() {
        Ok(g) => g,
        Err(_) => return vec![],
    };
    let mut stmt = match guard.conn.prepare(
        "SELECT p.id, p.name, p.preset, p.custom_spec,
                qc.snapshot_json, qc.fetched_at,
                cp.snapshot_json, cp.fetched_at
         FROM providers p
         LEFT JOIN quota_cache qc ON p.id = qc.provider_id
         LEFT JOIN coding_plan_quota_cache cp ON p.id = cp.provider_id
         WHERE p.pinned = 1
         ORDER BY p.sort_index, p.name",
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    let rows = match stmt.query_map([], |row| {
        let id: i64 = row.get(0)?;
        let name: String = row.get(1)?;
        let preset: Option<String> = row.get(2)?;
        let custom_spec_json: Option<String> = row.get(3)?;
        let quota_json: Option<String> = row.get(4)?;
        let quota_fetched: Option<i64> = row.get(5)?;
        let coding_json: Option<String> = row.get(6)?;
        let _coding_fetched: Option<i64> = row.get(7)?;

        // If provider has coding_plan, prefer coding_plan cache
        let has_cp = has_coding_plan(&preset, &custom_spec_json);
        if has_cp {
            let sub = coding_json
                .as_deref()
                .and_then(|s| serde_json::from_str::<SubscriptionQuota>(s).ok());
            Ok(format_coding_plan_line(&name, &sub))
        } else {
            let snapshot = quota_json
                .as_deref()
                .and_then(|s| serde_json::from_str::<QuotaSnapshot>(s).ok())
                .filter(|s| !matches!(
                    s.status,
                    LimitStatus::Unavailable | LimitStatus::Error | LimitStatus::NotConfigured
                ));
            let ppq = PinnedProviderQuota { id, name, preset, snapshot, fetched_at: quota_fetched };
            Ok(format_quota_line(&ppq))
        }
    }) {
        Ok(r) => r,
        Err(_) => return vec![],
    };
    rows.filter_map(|r| r.ok()).collect()
}

fn format_quota_line(p: &PinnedProviderQuota) -> String {
    let name = &p.name;

    let Some(snap) = &p.snapshot else {
        return format!("· {}  未配置", name);
    };

    let tone_icon = quota_icon(&snap.status);

    if let Some(bal) = &snap.balance {
        let total = bal.amount;
        let sym = currency_symbol(&bal.currency);
        if let Some(rem) = snap.remaining {
            format!("{} {}  {}{:.2} / {}{:.2}", tone_icon, name, sym, rem, sym, total)
        } else {
            format!("{} {}  {}{:.2}", tone_icon, name, sym, total)
        }
    } else if snap.remaining.is_some() || snap.total.is_some() {
        let rem = snap.remaining.unwrap_or(0.0);
        let total = snap.total.unwrap_or(0.0);
        format!("{} {}  {:.0} / {:.0}", tone_icon, name, rem, total)
    } else {
        let lbl = status_label(&snap.status);
        if lbl.is_empty() {
            format!("{} {}", tone_icon, name)
        } else {
            format!("{} {}  {}", tone_icon, name, lbl)
        }
    }
}

fn quota_icon(status: &LimitStatus) -> &'static str {
    match status {
        LimitStatus::Ok => "✓",
        LimitStatus::RateLimited | LimitStatus::SourceRateLimited => "⧗",
        LimitStatus::Unauthorized => "✗",
        LimitStatus::NotConfigured => "·",
        LimitStatus::Disabled | LimitStatus::Unavailable | LimitStatus::Error => "!",
    }
}

fn status_label(status: &LimitStatus) -> &'static str {
    match status {
        LimitStatus::RateLimited | LimitStatus::SourceRateLimited => "限流中",
        LimitStatus::Unauthorized => "未授权",
        LimitStatus::NotConfigured => "未配置",
        LimitStatus::Error => "错误",
        LimitStatus::Unavailable => "不可用",
        LimitStatus::Disabled => "已禁用",
        LimitStatus::Ok => "",
    }
}

fn currency_symbol(cur: &str) -> &'static str {
    match cur {
        "CNY" | "RMB" | "cny" | "rmb" => "¥",
        "USD" | "usd" => "$",
        "EUR" | "eur" => "€",
        "GBP" | "gbp" => "£",
        "JPY" | "jpy" => "¥",
        _ => "",
    }
}

fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, id: &str) {
    match id {
        "refresh_quota" => {
            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                refresh_and_rebuild(&app_handle).await;
            });
        }
        "open_window" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }
        "quit" => {
            app.exit(0);
        }
        _ => {}
    }
}
