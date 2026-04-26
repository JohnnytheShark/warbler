use tauri::{command, Emitter};
use sqlx::sqlite::SqlitePool;
use sqlx::Row;
use crate::models::{GroundingFolder, GroundingItem};

#[command]
pub async fn get_grounding_folders(pool: tauri::State<'_, SqlitePool>) -> Result<Vec<GroundingFolder>, String> {
    let rows = sqlx::query("SELECT id, path FROM grounding_folders")
        .fetch_all(&*pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(rows.into_iter().map(|r| GroundingFolder {
        id: r.get("id"),
        path: r.get("path"),
    }).collect())
}

#[command]
pub async fn delete_grounding_folder(pool: tauri::State<'_, SqlitePool>, id: String) -> Result<(), String> {
    sqlx::query("DELETE FROM grounding_folders WHERE id = ?").bind(&id).execute(&*pool).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub async fn clear_all_knowledge(pool: tauri::State<'_, SqlitePool>) -> Result<(), String> {
    sqlx::query("DELETE FROM grounding_items").execute(&*pool).await.map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM grounding_folders").execute(&*pool).await.map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn get_embedding(pool: &SqlitePool, text: &str) -> Result<Vec<f32>, String> {
    let model = sqlx::query("SELECT value FROM settings WHERE key = 'embedding_model'")
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?
        .map(|r| r.get::<String, _>("value"))
        .unwrap_or_else(|| "nomic-embed-text".to_string());

    let provider = sqlx::query("SELECT value FROM settings WHERE key = 'ai_provider'")
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?
        .map(|r| r.get::<String, _>("value"))
        .unwrap_or_else(|| "ollama".to_string());

    let base_url = sqlx::query("SELECT value FROM settings WHERE key = 'ai_base_url'")
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?
        .map(|r| r.get::<String, _>("value"))
        .unwrap_or_else(|| "http://localhost:11434".to_string());

    let endpoint = if provider == "ollama" {
        format!("{}/api/embeddings", base_url.trim_end_matches('/'))
    } else {
        format!("{}/embeddings", base_url.trim_end_matches('/'))
    };

    let client = reqwest::Client::new();
    let payload = if provider == "ollama" {
        serde_json::json!({ "model": model, "prompt": text })
    } else {
        serde_json::json!({ "model": model, "input": text })
    };

    let res = client.post(endpoint).json(&payload).send().await.map_err(|e: reqwest::Error| e.to_string())?;
    let json: serde_json::Value = res.json::<serde_json::Value>().await.map_err(|e: reqwest::Error| e.to_string())?;
        
    let embedding_array = if provider == "ollama" {
        json["embedding"].as_array().ok_or("No embedding found".to_string())?
    } else {
        json["data"][0]["embedding"].as_array().ok_or("No embedding found".to_string())?
    };
    
    let vector: Vec<f32> = embedding_array.iter().map(|v| v.as_f64().unwrap_or(0.0) as f32).collect();
    Ok(vector)
}

#[command]
pub async fn index_knowledge_folder(
    app_handle: tauri::AppHandle,
    pool: tauri::State<'_, SqlitePool>,
    path: String,
) -> Result<(), String> {
    let folder_id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO grounding_folders (id, path) VALUES (?, ?)")
        .bind(&folder_id).bind(&path).execute(&*pool).await.map_err(|e| e.to_string())?;

    let mut files = Vec::new();
    let mut stack = vec![std::path::PathBuf::from(&path)];
    while let Some(current) = stack.pop() {
        if current.is_dir() {
            if let Ok(entries) = std::fs::read_dir(current) {
                for entry in entries.flatten() { stack.push(entry.path()); }
            }
        } else {
            let ext = current.extension().and_then(|e| e.to_str()).unwrap_or("");
            if matches!(ext, "md" | "txt" | "json") { files.push(current); }
        }
    }

    let total_files = files.len();
    for (f_idx, file_path) in files.iter().enumerate() {
        let _ = app_handle.emit("indexing-progress", serde_json::json!({
            "status": format!("Reading {}...", file_path.file_name().and_then(|n| n.to_str()).unwrap_or("file")),
            "progress": (f_idx as f32 / total_files as f32) * 100.0
        }));

        let content = std::fs::read_to_string(&file_path).unwrap_or_default();
        let file_ext = file_path.extension().and_then(|e| e.to_str());
        let chunks = if file_ext == Some("json") {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(arr) = json.as_array() {
                    arr.iter().map(|v| {
                        let mut narrative = format!("Entry from {}: ", file_path.file_name().unwrap().to_string_lossy());
                        if let Some(obj) = v.as_object() {
                            for (k, val) in obj { narrative.push_str(&format!("{}: {}. ", k, val.to_string().replace("\"", ""))); }
                        } else { narrative.push_str(&v.to_string()); }
                        narrative
                    }).collect::<Vec<_>>()
                } else { vec![content] }
            } else { vec![content] }
        } else {
            content.split("\n\n").map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect::<Vec<_>>()
        };

        let total_chunks = chunks.len();
        for (c_idx, chunk) in chunks.iter().enumerate() {
            if chunk.len() < 5 { continue; }
            if c_idx % 10 == 0 || c_idx == total_chunks - 1 {
                let _ = app_handle.emit("indexing-progress", serde_json::json!({
                    "status": format!("Embedding {} ({} / {})", file_path.file_name().unwrap().to_string_lossy(), c_idx + 1, total_chunks),
                    "progress": ((f_idx as f32 / total_files as f32) + (c_idx as f32 / total_chunks as f32 / total_files as f32)) * 100.0
                }));
            }

            if let Ok(embedding) = get_embedding(&*pool, &format!("search_query: {}", chunk)).await {
                if let Ok(embedding_bytes) = bincode::serialize(&embedding) {
                    let _ = sqlx::query("INSERT INTO grounding_items (folder_id, file_path, content, embedding) VALUES (?, ?, ?, ?)")
                        .bind(&folder_id).bind(file_path.to_string_lossy().to_string()).bind(chunk).bind(embedding_bytes).execute(&*pool).await;
                }
            }
        }
    }
    let _ = app_handle.emit("indexing-progress", serde_json::json!({ "status": "Done", "progress": 100.0 }));
    Ok(())
}

fn cosine_similarity(v1: &[f32], v2: &[f32]) -> f32 {
    let dot: f32 = v1.iter().zip(v2.iter()).map(|(a, b)| a * b).sum();
    let mag1: f32 = v1.iter().map(|a| a * a).sum::<f32>().sqrt();
    let mag2: f32 = v2.iter().map(|a| a * a).sum::<f32>().sqrt();
    if mag1 == 0.0 || mag2 == 0.0 { 0.0 } else { dot / (mag1 * mag2) }
}

#[command]
pub async fn search_knowledge(
    pool: tauri::State<'_, SqlitePool>,
    query: String,
    top_k: i32,
) -> Result<Vec<GroundingItem>, String> {
    let query_embedding = get_embedding(&*pool, &format!("search_query: {}", query)).await?;
    let rows = sqlx::query("SELECT content, file_path, embedding FROM grounding_items").fetch_all(&*pool).await.map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for row in rows {
        let content: String = row.get("content");
        let file_path: String = row.get("file_path");
        let embedding_bytes: Vec<u8> = row.get("embedding");
        if let Ok(embedding) = bincode::deserialize::<Vec<f32>>(&embedding_bytes) {
            let sim = cosine_similarity(&query_embedding, &embedding);
            if sim > 0.2 {
                results.push(GroundingItem { content, file_path, similarity: Some(sim) });
            }
        }
    }
    results.sort_by(|a, b| b.similarity.unwrap_or(0.0).partial_cmp(&a.similarity.unwrap_or(0.0)).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(top_k as usize);
    Ok(results)
}
