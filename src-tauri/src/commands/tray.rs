use crate::error::AppError;
use crate::store::AppState;


#[tauri::command]
pub async fn pin_provider(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    provider_id: i64,
) -> Result<(), AppError> {
    let db = state.db.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let guard = db.lock().unwrap();
        guard.conn.execute(
            "UPDATE providers SET pinned = 1 WHERE id = ?1",
            [provider_id],
        )?;
        Ok::<_, AppError>(())
    })
    .await
    .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))??;

    crate::tray::rebuild_menu(&app);
    Ok(())
}


#[tauri::command]
pub async fn unpin_provider(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    provider_id: i64,
) -> Result<(), AppError> {
    let db = state.db.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let guard = db.lock().unwrap();
        guard.conn.execute(
            "UPDATE providers SET pinned = 0 WHERE id = ?1",
            [provider_id],
        )?;
        Ok::<_, AppError>(())
    })
    .await
    .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))??;

    crate::tray::rebuild_menu(&app);
    Ok(())
}


#[tauri::command]
pub fn quit_app() {
    std::process::exit(0);
}
