use tauri::command;
use sqlx::sqlite::SqlitePool;
use sqlx::Row;
use crate::models::Prompt;

#[command]
pub async fn get_prompts(pool: tauri::State<'_, SqlitePool>) -> Result<Vec<Prompt>, String> {
    let rows = sqlx::query("SELECT id, title, content FROM prompts ORDER BY created_at DESC")
        .fetch_all(&*pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(rows.into_iter().map(|r| Prompt {
        id: r.get("id"),
        title: r.get("title"),
        content: r.get("content"),
    }).collect())
}

#[command]
pub async fn add_prompt(pool: tauri::State<'_, SqlitePool>, title: String, content: String) -> Result<(), String> {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO prompts (id, title, content) VALUES (?, ?, ?)")
        .bind(id)
        .bind(title)
        .bind(content)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub async fn delete_prompt(pool: tauri::State<'_, SqlitePool>, id: String) -> Result<(), String> {
    sqlx::query("DELETE FROM prompts WHERE id = ?").bind(&id).execute(&*pool).await.map_err(|e| e.to_string())?;
    Ok(())
}
