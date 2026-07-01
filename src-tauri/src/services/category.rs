use crate::error::AppError;
use crate::store::AppState;
use crate::timeutil;
use crate::types::Category;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AddCategoryRequest {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteCategoryRequest {
    pub id: i64,
    pub migrate_to: i64,
}

fn row_to_category(row: &rusqlite::Row) -> Result<Category, rusqlite::Error> {
    Ok(Category {
        id: row.get(0)?,
        name: row.get(1)?,
        is_default: row.get::<_, i64>(2)? != 0,
        sort_index: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

pub async fn list_categories(state: tauri::State<'_, AppState>) -> Result<Vec<Category>, AppError> {
    list_categories_by_state(&state).await
}

pub async fn list_categories_by_state(state: &AppState) -> Result<Vec<Category>, AppError> {
    let db = state.db.clone();
    let categories = tauri::async_runtime::spawn_blocking(move || {
        let guard = db.lock().unwrap();
        let mut stmt = guard.conn.prepare(
            "SELECT id, name, is_default, sort_index, created_at, updated_at
             FROM categories ORDER BY sort_index"
        )?;
        let rows = stmt.query_map([], row_to_category)?;
        let categories: Vec<Category> = rows.filter_map(|r| r.ok()).collect();
        Ok::<_, AppError>(categories)
    }).await.map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))??;
    Ok(categories)
}

pub async fn add_category(
    state: tauri::State<'_, AppState>,
    req: AddCategoryRequest,
) -> Result<Category, AppError> {
    add_category_by_state(&state, req).await
}

pub async fn add_category_by_state(state: &AppState, req: AddCategoryRequest) -> Result<Category, AppError> {
    let db = state.db.clone();
    let now = timeutil::now_secs();

    let category = tauri::async_runtime::spawn_blocking(move || {
        let guard = db.lock().unwrap();

        guard.conn.execute(
            "INSERT INTO categories (name, is_default, sort_index, created_at, updated_at)
             VALUES (?1, 0, (SELECT COALESCE(MAX(sort_index), 0) + 1 FROM categories), ?2, ?2)",
            rusqlite::params![req.name, now],
        )?;

        let id: i64 = guard.conn.last_insert_rowid();

        let mut stmt = guard.conn.prepare(
            "SELECT id, name, is_default, sort_index, created_at, updated_at
             FROM categories WHERE id = ?1"
        )?;
        let category = stmt.query_row([id], row_to_category)?;
        Ok::<_, AppError>(category)
    }).await.map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))??;

    Ok(category)
}

pub async fn delete_category(
    state: tauri::State<'_, AppState>,
    req: DeleteCategoryRequest,
) -> Result<(), AppError> {
    delete_category_by_state(&state, req).await
}

pub async fn delete_category_by_state(state: &AppState, req: DeleteCategoryRequest) -> Result<(), AppError> {
    let db = state.db.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let guard = db.lock().unwrap();

        // Check if category is default
        let is_default: i64 = guard.conn.query_row(
            "SELECT is_default FROM categories WHERE id = ?1",
            [req.id],
            |row| row.get(0),
        ).map_err(|_| AppError::CategoryNotFound(req.id))?;

        if is_default == 1 {
            return Err(AppError::CategoryIsDefault(req.id));
        }

        // Migrate providers to target category
        guard.conn.execute(
            "UPDATE providers SET category_id = ?1 WHERE category_id = ?2",
            rusqlite::params![req.migrate_to, req.id],
        )?;

        // Delete category
        guard.conn.execute("DELETE FROM categories WHERE id = ?1", [req.id])?;

        Ok::<_, AppError>(())
    }).await.map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))??;

    Ok(())
}
