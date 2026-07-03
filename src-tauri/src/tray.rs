use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{TrayIcon, TrayIconBuilder},
    AppHandle, Emitter, Manager, Runtime,
};



pub fn init_tray<R: Runtime>(app: &AppHandle<R>) -> Result<TrayIcon<R>, tauri::Error> {
    let menu = build_tray_menu(app)?;

    let tray_icon = Image::from_bytes(include_bytes!("../icons/tray.png"))
        .expect("Failed to load tray icon");

    let tray = TrayIconBuilder::new()
        .icon(tray_icon)
        .menu(&menu)
        .tooltip("KeyPilot")
        .on_menu_event(move |app, event| {
            handle_menu_event(app, event.id.as_ref());
        })
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left,
                ..
            } = event
            {
                
                if let Some(window) = tray.app_handle().get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(tray)
}

fn build_tray_menu<R: Runtime>(app: &AppHandle<R>) -> Result<Menu<R>, tauri::Error> {
    let copy_key = MenuItem::with_id(app, "copy_key", "复制 key", true, None::<&str>)?;
    let open_window = MenuItem::with_id(app, "open_window", "打开主窗口", true, None::<&str>)?;
    let pin = MenuItem::with_id(app, "pin", "钉住", true, None::<&str>)?;
    let delete = MenuItem::with_id(app, "delete", "删除", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    Menu::with_items(app, &[&copy_key, &open_window, &pin, &delete, &quit])
}

fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, id: &str) {
    match id {
        "copy_key" => {
            let _ = app.emit("tray-copy-key", ());
        }
        "open_window" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        "pin" => {
            let _ = app.emit("tray-pin", ());
        }
        "delete" => {
            let _ = app.emit("tray-delete", ());
        }
        "quit" => {
            app.exit(0);
        }
        _ => {}
    }
}
