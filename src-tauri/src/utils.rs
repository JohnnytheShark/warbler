use tauri::command;
use sqlx::sqlite::SqlitePool;
use sqlx::Row;

#[command]
pub async fn get_config(pool: tauri::State<'_, SqlitePool>, key: String) -> Result<Option<String>, String> {
    let row = sqlx::query("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(row.map(|r| r.get("value")))
}

#[command]
pub async fn set_config(pool: tauri::State<'_, SqlitePool>, key: String, value: String) -> Result<(), String> {
    sqlx::query("INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = EXCLUDED.value")
        .bind(key)
        .bind(value)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub async fn write_to_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(path, content).map_err(|e| e.to_string())
}

#[command]
pub async fn read_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| e.to_string())
}
