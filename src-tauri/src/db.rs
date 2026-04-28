use tauri::command;
use sqlx::sqlite::SqlitePool;
use sqlx::Row;
use crate::models::{Chat, OllamaMessage};

#[command]
pub async fn get_chats(pool: tauri::State<'_, SqlitePool>) -> Result<Vec<Chat>, String> {
    let chat_rows = sqlx::query("SELECT id, title FROM chats WHERE id != '__model_pref__' ORDER BY created_at DESC")
        .fetch_all(&*pool)
        .await
        .map_err(|e| e.to_string())?;

    let mut chats = Vec::new();
    for row in chat_rows {
        let id: String = row.get("id");
        let title: String = row.get("title");

        let msg_rows = sqlx::query("SELECT role, content, images, thinking, tool_name FROM messages WHERE chat_id = ? ORDER BY seq ASC")
            .bind(&id)
            .fetch_all(&*pool)
            .await
            .map_err(|e| e.to_string())?;

        let messages = msg_rows.into_iter().map(|m| {
            let images_raw: Option<String> = m.get("images");
            OllamaMessage {
                role: m.get("role"),
                content: m.get("content"),
                images: images_raw.and_then(|i| serde_json::from_str::<Vec<String>>(&i).ok()),
                thinking: m.get("thinking"),
                tool_name: m.get("tool_name"),
            }
        }).collect();

        chats.push(Chat {
            id,
            title,
            messages,
        });
    }
    Ok(chats)
}

#[command]
pub async fn new_chat(pool: tauri::State<'_, SqlitePool>, id: String, title: String) -> Result<(), String> {
    sqlx::query("INSERT INTO chats (id, title) VALUES (?, ?)")
        .bind(id)
        .bind(title)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub async fn delete_chat(pool: tauri::State<'_, SqlitePool>, id: String) -> Result<(), String> {
    sqlx::query("DELETE FROM messages WHERE chat_id = ?").bind(&id).execute(&*pool).await.map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM chats WHERE id = ?").bind(&id).execute(&*pool).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub async fn append_message(
    pool: tauri::State<'_, SqlitePool>,
    chat_id: String,
    msg: OllamaMessage,
    seq: i64
) -> Result<(), String> {
    let images_json = msg.images.as_ref().and_then(|i| serde_json::to_string(i).ok());
    sqlx::query("INSERT INTO messages (chat_id, role, content, images, thinking, tool_name, seq) VALUES (?, ?, ?, ?, ?, ?, ?)")
        .bind(chat_id)
        .bind(msg.role)
        .bind(msg.content)
        .bind(images_json)
        .bind(msg.thinking)
        .bind(msg.tool_name)
        .bind(seq)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub async fn update_chat_title(pool: tauri::State<'_, SqlitePool>, id: String, title: String) -> Result<(), String> {
    sqlx::query("UPDATE chats SET title = ? WHERE id = ?")
        .bind(title)
        .bind(id)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub async fn save_model_pref(pool: tauri::State<'_, SqlitePool>, model: String) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO chats (id, title) VALUES ('__model_pref__', ?) ON CONFLICT(id) DO UPDATE SET title=excluded.title"
    )
    .bind(model)
    .execute(&*pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub async fn load_model_pref(pool: tauri::State<'_, SqlitePool>) -> Result<Option<String>, String> {
    let row = sqlx::query("SELECT title FROM chats WHERE id = '__model_pref__'")
        .fetch_optional(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(row.map(|r| r.get("title")))
}
