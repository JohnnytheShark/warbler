use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{command, Emitter, Manager};
use reqwest;
use tokio::sync::Notify;
use sqlx::sqlite::SqlitePool;
use sqlx::Row;

/// Shared cancellation signal – one per app instance.
type CancelSignal = Arc<Notify>;

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaMessage {
    pub role: String,
    pub content: String,
    pub images: Option<Vec<String>>,
    pub thinking: Option<String>,
    pub tool_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Chat {
    pub id: String,
    pub title: String,
    pub messages: Vec<OllamaMessage>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroundingFolder {
    pub id: String,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroundingItem {
    pub content: String,
    pub file_path: String,
    pub similarity: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Prompt {
    pub id: String,
    pub title: String,
    pub content: String,
}

// ── MCP Structures ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaToolCall {
    pub function: OllamaFunctionCall,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaFunctionCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpServerConfig {
    pub id: String,
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

#[command]
async fn list_mcp_servers(pool: tauri::State<'_, SqlitePool>) -> Result<Vec<McpServerConfig>, String> {
    let rows = sqlx::query("SELECT id, name, command, args, env FROM mcp_servers")
        .fetch_all(&*pool)
        .await
        .map_err(|e| e.to_string())?;

    let mut servers = Vec::new();
    for r in rows {
        let args_json: String = r.get("args");
        let env_json: String = r.get("env");
        servers.push(McpServerConfig {
            id: r.get("id"),
            name: r.get("name"),
            command: r.get("command"),
            args: serde_json::from_str(&args_json).unwrap_or_default(),
            env: serde_json::from_str(&env_json).unwrap_or_default(),
        });
    }
    Ok(servers)
}

#[command]
async fn add_mcp_server(
    pool: tauri::State<'_, SqlitePool>,
    config: McpServerConfig
) -> Result<(), String> {
    let args_json = serde_json::to_string(&config.args).unwrap_or_else(|_| "[]".to_string());
    let env_json = serde_json::to_string(&config.env).unwrap_or_else(|_| "[]".to_string());
    
    sqlx::query("INSERT INTO mcp_servers (id, name, command, args, env) VALUES (?, ?, ?, ?, ?)")
        .bind(&config.id)
        .bind(&config.name)
        .bind(&config.command)
        .bind(&args_json)
        .bind(&env_json)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
async fn delete_mcp_server(pool: tauri::State<'_, SqlitePool>, id: String) -> Result<(), String> {
    sqlx::query("DELETE FROM mcp_servers WHERE id = ?").bind(&id).execute(&*pool).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
async fn cancel_chat(signal: tauri::State<'_, CancelSignal>) -> Result<(), String> {
    signal.notify_one();
    Ok(())
}

#[command]
async fn get_ollama_models(pool: tauri::State<'_, SqlitePool>) -> Result<String, String> {
    let provider = sqlx::query("SELECT value FROM settings WHERE key = 'ai_provider'")
        .fetch_optional(&*pool).await.map_err(|e| e.to_string())?
        .map(|r| r.get::<String, _>("value")).unwrap_or_else(|| "ollama".to_string());
    let base_url = sqlx::query("SELECT value FROM settings WHERE key = 'ai_base_url'")
        .fetch_optional(&*pool).await.map_err(|e| e.to_string())?
        .map(|r| r.get::<String, _>("value")).unwrap_or_else(|| "http://localhost:11434".to_string());
    let api_key = sqlx::query("SELECT value FROM settings WHERE key = 'ai_api_key'")
        .fetch_optional(&*pool).await.map_err(|e| e.to_string())?
        .map(|r| r.get::<String, _>("value"));

    let client = reqwest::Client::new();
    if provider == "ollama" {
        let res = client.get(format!("{}/api/tags", base_url.trim_end_matches('/'))).send().await.map_err(|e| e.to_string())?;
        let text = res.text().await.map_err(|e| e.to_string())?;
        Ok(text)
    } else {
        let mut request = client.get(format!("{}/models", base_url.trim_end_matches('/')));
        if let Some(key) = api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }
        let res = request.send().await.map_err(|e| e.to_string())?;
        let res_json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
        
        let mut models = Vec::new();
        if let Some(data) = res_json["data"].as_array() {
            for m in data {
                if let Some(id) = m["id"].as_str() {
                    models.push(serde_json::json!({ "name": id }));
                }
            }
        }
        Ok(serde_json::json!({ "models": models }).to_string())
    }
}

#[command]
async fn chat_with_model(
    app_handle: tauri::AppHandle,
    pool: tauri::State<'_, SqlitePool>,
    messages: Vec<serde_json::Value>,
    model: String,
    use_tools: bool,
    signal: tauri::State<'_, CancelSignal>,
) -> Result<String, String> {
    let provider = sqlx::query("SELECT value FROM settings WHERE key = 'ai_provider'")
        .fetch_optional(&*pool)
        .await
        .map_err(|e| e.to_string())?
        .map(|r| r.get::<String, _>("value"))
        .unwrap_or_else(|| "ollama".to_string());

    let base_url = sqlx::query("SELECT value FROM settings WHERE key = 'ai_base_url'")
        .fetch_optional(&*pool)
        .await
        .map_err(|e| e.to_string())?
        .map(|r| r.get::<String, _>("value"))
        .unwrap_or_else(|| "http://localhost:11434".to_string());

    let api_key = sqlx::query("SELECT value FROM settings WHERE key = 'ai_api_key'")
        .fetch_optional(&*pool)
        .await
        .map_err(|e| e.to_string())?
        .map(|r| r.get::<String, _>("value"));

    let is_ollama = provider == "ollama";
    let chat_endpoint = if is_ollama {
        format!("{}/api/chat", base_url.trim_end_matches('/'))
    } else {
        format!("{}/chat/completions", base_url.trim_end_matches('/'))
    };

    let mut current_messages = messages.clone();
    
    // ── Pre-process Hashtags ──
    if let Some(last_msg) = current_messages.iter_mut().rev().find(|m| m["role"] == "user") {
        if let Some(content) = last_msg["content"].as_str() {
             if let Ok(processed) = preprocess_hashtags_internal(app_handle.clone(), pool.inner().clone(), content.to_string()).await {
                 *last_msg = serde_json::json!({
                     "role": "user",
                     "content": processed,
                     "images": last_msg.get("images")
                 });
             }
        }
    }

    let mut final_json_response = String::new();
    let mut accumulated_thinking = String::new();

    let tools_json = if use_tools {
        match get_all_mcp_tools(pool.clone()).await {
            Ok(tools) => {
                let mut ollama_tools = Vec::new();
                for t in tools {
                    ollama_tools.push(serde_json::json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description.unwrap_or_default(),
                            "parameters": t.input_schema
                        }
                    }));
                }
                serde_json::json!(ollama_tools)
            },
            Err(_) => serde_json::Value::Null,
        }
    } else {
        serde_json::Value::Null
    };

    let client = reqwest::Client::new();

    for _ in 0..5 {
        let mut payload_map = serde_json::Map::new();
        payload_map.insert("model".to_string(), serde_json::json!(model));
        payload_map.insert("messages".to_string(), serde_json::json!(current_messages));
        payload_map.insert("stream".to_string(), serde_json::json!(false));
        
        if !tools_json.is_null() {
            payload_map.insert("tools".to_string(), tools_json.clone());
        }

        let payload = serde_json::Value::Object(payload_map);
        let mut request_builder = client.post(&chat_endpoint).header("Content-Type", "application/json");

        if let Some(key) = &api_key {
            request_builder = request_builder.header("Authorization", format!("Bearer {}", key));
        }

        let request = request_builder.body(payload.to_string()).send();
        let res = tokio::select! {
            result = request => result.map_err(|e: reqwest::Error| e.to_string())?,
            _ = signal.notified() => return Err("__cancelled__".to_string()),
        };

        let res_json: serde_json::Value = tokio::select! {
            result = res.json() => result.map_err(|e: reqwest::Error| e.to_string())?,
            _ = signal.notified() => return Err("__cancelled__".to_string()),
        };

        let (message, content, thinking) = if is_ollama {
            let msg = res_json["message"].clone();
            let c = msg["content"].as_str().unwrap_or_default().to_string();
            let t = msg["thinking"].as_str().map(|s| s.to_string());
            (msg, c, t)
        } else {
            let msg = res_json["choices"][0]["message"].clone();
            let c = msg["content"].as_str().unwrap_or_default().to_string();
            (msg, c, None)
        };

        if let Some(t) = &thinking {
            if !accumulated_thinking.is_empty() {
                accumulated_thinking.push_str("\n\n");
            }
            accumulated_thinking.push_str(t);
        }

        let mut current_res_json = res_json.clone();
        if !accumulated_thinking.is_empty() {
            if is_ollama {
                current_res_json["message"]["thinking"] = serde_json::json!(accumulated_thinking);
            } else {
                // For OpenAI format, we might need a custom field or just append to content if needed,
                // but let's keep it in a 'thinking' field for our frontend to find.
                if let Some(msg) = current_res_json["choices"][0]["message"].as_object_mut() {
                    msg.insert("thinking".to_string(), serde_json::json!(accumulated_thinking));
                }
            }
        }

        let current_json_str = if is_ollama {
            current_res_json.to_string()
        } else {
            serde_json::json!({
                "message": {
                    "role": "assistant",
                    "content": content,
                    "thinking": accumulated_thinking
                }
            }).to_string()
        };
        
        if !content.is_empty() || thinking.is_some() {
            final_json_response = current_json_str;
        }

        if let Some(tool_calls) = message["tool_calls"].as_array() {
            if tool_calls.is_empty() { return Ok(final_json_response); }
            current_messages.push(message.clone());
            for call in tool_calls {
                let tool_name = call["function"]["name"].as_str().unwrap_or_default();
                let tool_args = &call["function"]["arguments"];
                let args_val: serde_json::Value = if tool_args.is_string() {
                    serde_json::from_str(tool_args.as_str().unwrap()).unwrap_or(serde_json::json!({}))
                } else {
                    tool_args.clone()
                };

                let _ = app_handle.emit("tool-call", &tool_name);
                let result = match call_mcp_tool(pool.clone(), tool_name.to_string(), args_val).await {
                    Ok(r) => {
                        if let Some(parts) = r.get("content").and_then(|c| c.as_array()) {
                            parts.iter().filter_map(|p| p.get("text").and_then(|txt| txt.as_str())).collect::<Vec<_>>().join("\n")
                        } else { r.to_string() }
                    },
                    Err(e) => format!("Error: {}", e),
                };
                let _ = app_handle.emit("tool-response", &tool_name);

                let _ = app_handle.emit("tool-result", serde_json::json!({ "name": tool_name, "content": result.clone() }));

                if is_ollama {
                    current_messages.push(serde_json::json!({ "role": "user", "content": format!("Tool response for '{}':\n{}", tool_name, result) }));
                } else {
                    current_messages.push(serde_json::json!({
                        "role": "tool",
                        "tool_call_id": call["id"],
                        "name": tool_name,
                        "content": result
                    }));
                }
            }
        } else {
            return Ok(final_json_response);
        }
    }
    Ok(final_json_response)
}

// ── Database Commands ────────────────────────────────────────────────────────

#[command]
async fn get_chats(pool: tauri::State<'_, SqlitePool>) -> Result<Vec<Chat>, String> {
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
async fn new_chat(pool: tauri::State<'_, SqlitePool>, id: String, title: String) -> Result<(), String> {
    sqlx::query("INSERT INTO chats (id, title) VALUES (?, ?)")
        .bind(id)
        .bind(title)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
async fn delete_chat(pool: tauri::State<'_, SqlitePool>, id: String) -> Result<(), String> {
    sqlx::query("DELETE FROM messages WHERE chat_id = ?").bind(&id).execute(&*pool).await.map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM chats WHERE id = ?").bind(&id).execute(&*pool).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
async fn append_message(
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
async fn update_chat_title(pool: tauri::State<'_, SqlitePool>, id: String, title: String) -> Result<(), String> {
    sqlx::query("UPDATE chats SET title = ? WHERE id = ?")
        .bind(title)
        .bind(id)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
async fn save_model_pref(pool: tauri::State<'_, SqlitePool>, model: String) -> Result<(), String> {
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
async fn load_model_pref(pool: tauri::State<'_, SqlitePool>) -> Result<Option<String>, String> {
    let row = sqlx::query("SELECT title FROM chats WHERE id = '__model_pref__'")
        .fetch_optional(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(row.map(|r| r.get("title")))
}

#[command]
async fn get_config(pool: tauri::State<'_, SqlitePool>, key: String) -> Result<Option<String>, String> {
    let row = sqlx::query("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(row.map(|r| r.get("value")))
}

#[command]
async fn set_config(pool: tauri::State<'_, SqlitePool>, key: String, value: String) -> Result<(), String> {
    sqlx::query("INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = EXCLUDED.value")
        .bind(key)
        .bind(value)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
async fn pull_ollama_model(app_handle: tauri::AppHandle, model: String) -> Result<(), String> {
    use futures_util::StreamExt;
    use tauri::Emitter;

    let client = reqwest::Client::new();
    let mut res = client
        .post("http://localhost:11434/api/pull")
        .json(&serde_json::json!({ "name": model }))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .bytes_stream();

    while let Some(chunk) = res.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&chunk) {
             let _ = app_handle.emit("pull-progress", json);
        }
    }

    Ok(())
}

// ── Grounding Commands ───────────────────────────────────────────────────────

#[command]
async fn get_grounding_folders(pool: tauri::State<'_, SqlitePool>) -> Result<Vec<GroundingFolder>, String> {
    let rows = sqlx::query("SELECT id, path FROM grounding_folders")
        .fetch_all(&*pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(rows.into_iter().map(|r| GroundingFolder {
        id: r.get("id"),
        path: r.get("path"),
    }).collect())
}

// ── MCP JSON-RPC Engine ─────────────────────────────────────────────────────

async fn process_mcp_request(
    command: &str,
    args: &[String],
    env: &[(String, String)],
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::process::Command;
    use std::process::Stdio;


    let mut child = Command::new(command)
        .args(args)
        .envs(env.iter().cloned())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    let mut stdin = child.stdin.take().ok_or("Failed to open stdin")?;
    let stdout = child.stdout.take().ok_or("Failed to open stdout")?;
    let mut reader = BufReader::new(stdout).lines();

    // 1. Initialize

    let init_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "Warbler", "version": "1.0.0" }
        }
    });
    stdin.write_all(format!("{}\n", init_req).as_bytes()).await
        .map_err(|e: std::io::Error| e.to_string())?;

    let _init_resp = reader.next_line().await
        .map_err(|e: std::io::Error| e.to_string())?;

    // 2. Send initialized notification (required by MCP spec before any requests)
    let initialized_notif = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    stdin.write_all(format!("{}\n", initialized_notif).as_bytes()).await
        .map_err(|e: std::io::Error| e.to_string())?;

    // 3. The actual request

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": method,
        "params": params
    });
    stdin.write_all(format!("{}\n", req).as_bytes()).await
        .map_err(|e: std::io::Error| e.to_string())?;

    let response = reader.next_line().await
        .map_err(|e: std::io::Error| e.to_string())?
        .ok_or_else(|| "No response from server (EOF)".to_string())?;

    // Cleanup
    let _ = child.kill().await;

    let json: serde_json::Value = serde_json::from_str(&response).map_err(|e| e.to_string())?;
    if let Some(error) = json.get("error") {
        return Err(error.to_string());
    }
    
    Ok(json["result"].clone())
}

#[command]
async fn get_all_mcp_tools(pool: tauri::State<'_, SqlitePool>) -> Result<Vec<McpTool>, String> {
    let servers = list_mcp_servers(pool).await?;
    let mut all_tools = Vec::new();

    for server in servers {
        println!("Listing tools for server: {}", server.name);
        match process_mcp_request(&server.command, &server.args, &server.env, "tools/list", serde_json::json!({})).await {
            Ok(result) => {
                if let Some(tools) = result["tools"].as_array() {
                    println!("Found {} tools for {}", tools.len(), server.name);
                    for t in tools {
                        all_tools.push(McpTool {
                            name: t["name"].as_str().unwrap_or_default().to_string(),
                            description: t["description"].as_str().map(|s| s.to_string()),
                            input_schema: t["inputSchema"].clone(),
                        });
                    }
                }
            },
            Err(e) => println!("Error listing tools for {}: {}", server.name, e),
        }
    }
    println!("Total tools found: {}", all_tools.len());
    Ok(all_tools)
}

#[command]
async fn call_mcp_tool(
    pool: tauri::State<'_, SqlitePool>,
    name: String,
    arguments: serde_json::Value
) -> Result<serde_json::Value, String> {
    let servers = list_mcp_servers(pool.clone()).await?;
    
    // Find which server has this tool (for now, just check all until we find it)
    for server in servers {
        let tools_res = process_mcp_request(&server.command, &server.args, &server.env, "tools/list", serde_json::json!({})).await;
        if let Ok(res) = tools_res {
             if res["tools"].as_array().map_or(false, |ts| ts.iter().any(|t| t["name"] == name)) {
                return process_mcp_request(
                    &server.command, 
                    &server.args, 
                    &server.env,
                    "tools/call", 
                    serde_json::json!({ "name": name, "arguments": arguments })
                ).await;
             }
        }
    }
    Err(format!("Tool {} not found in any server", name))
}

#[command]
async fn preprocess_hashtags(
    app_handle: tauri::AppHandle,
    pool: tauri::State<'_, SqlitePool>,
    text: String,
) -> Result<String, String> {
    preprocess_hashtags_internal(app_handle, pool.inner().clone(), text).await
}

async fn preprocess_hashtags_internal(
    app_handle: tauri::AppHandle,
    pool: SqlitePool,
    text: String,
) -> Result<String, String> {
    let mut augmented_text = text.clone();
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut hashtags = Vec::new();
    
    for word in words {
        if word.starts_with('#') && word.len() > 1 {
            let tag = word[1..].trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
            if !tag.is_empty() {
                hashtags.push(tag.to_string());
            }
        }
    }
    
    hashtags.sort();
    hashtags.dedup();

    for tag in hashtags {
        if text.contains(&format!("--- Tool Output (#{}) ---", tag)) {
            continue;
        }

        match call_mcp_tool_internal(pool.clone(), tag.clone(), serde_json::json!({})).await {
            Ok(result) => {
                let mut tool_output = if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
                    content.iter()
                        .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
                        .collect::<Vec<_>>()
                        .join("\n")
                } else if result.is_string() {
                    result.as_str().unwrap_or_default().to_string()
                } else {
                    serde_json::to_string_pretty(&result).unwrap_or_default()
                };

                let max_len = sqlx::query("SELECT value FROM settings WHERE key = 'mcp_max_length'")
                    .fetch_optional(&pool)
                    .await
                    .ok()
                    .flatten()
                    .and_then(|r| r.get::<String, _>("value").parse::<usize>().ok())
                    .unwrap_or(2500);

                if tool_output.len() > max_len {
                    tool_output.truncate(max_len);
                    tool_output.push_str("\n... (truncated for brevity)");
                }

                // Emit event so it shows as a separate message
                let _ = app_handle.emit("tool-result", serde_json::json!({ "name": tag, "content": tool_output.clone() }));
                
                // Still augment for the model's context, but we might want to keep it hidden from user in future
                augmented_text.push_str(&format!("\n\n--- Tool Output (#{}) ---\n{}", tag, tool_output));
            },
            Err(_) => {}
        }
    }

    Ok(augmented_text)
}


async fn call_mcp_tool_internal(
    pool: SqlitePool,
    name: String,
    arguments: serde_json::Value
) -> Result<serde_json::Value, String> {
    let rows = sqlx::query("SELECT id, name, command, args, env FROM mcp_servers")
        .fetch_all(&pool)
        .await
        .map_err(|e| e.to_string())?;

    let mut servers = Vec::new();
    for r in rows {
        let args_json: String = r.get("args");
        let env_json: String = r.get("env");
        servers.push(McpServerConfig {
            id: r.get("id"),
            name: r.get("name"),
            command: r.get("command"),
            args: serde_json::from_str(&args_json).unwrap_or_default(),
            env: serde_json::from_str(&env_json).unwrap_or_default(),
        });
    }
    
    for server in servers {
        let tools_res = process_mcp_request(&server.command, &server.args, &server.env, "tools/list", serde_json::json!({})).await;
        if let Ok(res) = tools_res {
             if res["tools"].as_array().map_or(false, |ts| ts.iter().any(|t| t["name"] == name)) {
                return process_mcp_request(
                    &server.command, 
                    &server.args, 
                    &server.env,
                    "tools/call", 
                    serde_json::json!({ "name": name, "arguments": arguments })
                ).await;
             }
        }
    }
    Err(format!("Tool {} not found in any server", name))
}

#[command]
async fn check_mcp_server(
    pool: tauri::State<'_, SqlitePool>,
    id: String
) -> Result<bool, String> {
    let servers = list_mcp_servers(pool).await?;
    let server = servers.into_iter().find(|s| s.id == id).ok_or("Server not found")?;
    
    match process_mcp_request(&server.command, &server.args, &server.env, "tools/list", serde_json::json!({})).await {
        Ok(_) => {
            Ok(true)
        },
        Err(_) => {
            Ok(false)
        },
    }
}

#[command]
async fn clear_all_knowledge(pool: tauri::State<'_, SqlitePool>) -> Result<(), String> {
    sqlx::query("DELETE FROM grounding_items").execute(&*pool).await.map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM grounding_folders").execute(&*pool).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
async fn delete_grounding_folder(pool: tauri::State<'_, SqlitePool>, id: String) -> Result<(), String> {
    sqlx::query("DELETE FROM grounding_folders WHERE id = ?").bind(&id).execute(&*pool).await.map_err(|e| e.to_string())?;
    Ok(())
}

// ── Prompts Commands ────────────────────────────────────────────────────────

#[command]
async fn get_prompts(pool: tauri::State<'_, SqlitePool>) -> Result<Vec<Prompt>, String> {
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
async fn add_prompt(pool: tauri::State<'_, SqlitePool>, title: String, content: String) -> Result<(), String> {
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
async fn delete_prompt(pool: tauri::State<'_, SqlitePool>, id: String) -> Result<(), String> {
    sqlx::query("DELETE FROM prompts WHERE id = ?").bind(&id).execute(&*pool).await.map_err(|e| e.to_string())?;
    Ok(())
}

async fn get_embedding(pool: &SqlitePool, text: &str) -> Result<Vec<f32>, String> {
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

    let res = client
        .post(endpoint)
        .json(&payload)
        .send()
        .await
        .map_err(|e: reqwest::Error| e.to_string())?;

    let json: serde_json::Value = res
        .json::<serde_json::Value>()
        .await
        .map_err(|e: reqwest::Error| e.to_string())?;
        
    let embedding_array = if provider == "ollama" {
        json["embedding"].as_array().ok_or("No embedding found".to_string())?
    } else {
        json["data"][0]["embedding"].as_array().ok_or("No embedding found".to_string())?
    };
    
    let vector: Vec<f32> = embedding_array
        .iter()
        .map(|v: &serde_json::Value| v.as_f64().unwrap_or(0.0) as f32)
        .collect();
    
    Ok(vector)
}

#[command]
async fn index_knowledge_folder(
    app_handle: tauri::AppHandle,
    pool: tauri::State<'_, SqlitePool>,
    path: String,
) -> Result<(), String> {
    let folder_id = uuid::Uuid::new_v4().to_string();
    
    // 1. Add folder record
    sqlx::query("INSERT INTO grounding_folders (id, path) VALUES (?, ?)")
        .bind(&folder_id)
        .bind(&path)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;

    // 2. Crawl files
    let mut files = Vec::new();
    let mut stack = vec![std::path::PathBuf::from(&path)];
    
    while let Some(current) = stack.pop() {
        if current.is_dir() {
            if let Ok(entries) = std::fs::read_dir(current) {
                for entry in entries.flatten() {
                    stack.push(entry.path());
                }
            }
        } else {
            let ext = current.extension().and_then(|e| e.to_str()).unwrap_or("");
            if matches!(ext, "md" | "txt" | "json") {
                files.push(current);
            }
        }
    }

    let total_files = files.len();
    for (f_idx, file_path) in files.iter().enumerate() {
        let _ = app_handle.emit("indexing-progress", serde_json::json!({
            "status": format!("Reading {}...", file_path.file_name().and_then(|n| n.to_str()).unwrap_or("file")),
            "progress": (f_idx as f32 / total_files as f32) * 100.0
        }));

        let content = match std::fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        
        let file_ext = file_path.extension().and_then(|e| e.to_str());
        let chunks = if file_ext == Some("json") {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(arr) = json.as_array() {
                    // Narrative Chunking for JSON list
                    arr.iter().map(|v| {
                        let mut narrative = format!("Entry from {}: ", file_path.file_name().unwrap().to_string_lossy());
                        if let Some(obj) = v.as_object() {
                            for (k, val) in obj {
                                narrative.push_str(&format!("{}: {}. ", k, val.to_string().replace("\"", "")));
                            }
                        } else {
                            narrative.push_str(&v.to_string());
                        }
                        narrative
                    }).collect::<Vec<_>>()
                } else {
                    vec![content]
                }
            } else {
                vec![content]
            }
        } else {
            content.split("\n\n").map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect::<Vec<_>>()
        };

        let total_chunks = chunks.len();
        for (c_idx, chunk) in chunks.iter().enumerate() {
            if chunk.len() < 5 { continue; }
            
            // Emit progress for every 10 chunks to avoid flooding
            if c_idx % 10 == 0 || c_idx == total_chunks - 1 {
                let _ = app_handle.emit("indexing-progress", serde_json::json!({
                    "status": format!("Embedding {} ({} / {})", file_path.file_name().unwrap().to_string_lossy(), c_idx + 1, total_chunks),
                    "progress": ((f_idx as f32 / total_files as f32) + (c_idx as f32 / total_chunks as f32 / total_files as f32)) * 100.0
                }));
            }

            match get_embedding(&*pool, &format!("search_query: {}", chunk)).await {
                Ok(embedding) => {
                    let embedding_bytes = match bincode::serialize(&embedding) {
                        Ok(b) => b,
                        Err(_) => continue,
                    };

                    let _ = sqlx::query("INSERT INTO grounding_items (folder_id, file_path, content, embedding) VALUES (?, ?, ?, ?)")
                        .bind(&folder_id)
                        .bind(file_path.to_string_lossy().to_string())
                        .bind(chunk)
                        .bind(embedding_bytes)
                        .execute(&*pool)
                        .await;
                },
                Err(_) => continue, // Skip if Ollama hiccups
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
async fn search_knowledge(
    pool: tauri::State<'_, SqlitePool>,
    query: String,
    top_k: i32,
) -> Result<Vec<GroundingItem>, String> {
    let query_embedding = get_embedding(&*pool, &format!("search_query: {}", query)).await?;
    
    // Select all items and calculate similarity in-memory
    // For local SQLite on a 1650, this is very fast for thousands of rows.
    let rows = sqlx::query("SELECT content, file_path, embedding FROM grounding_items")
        .fetch_all(&*pool)
        .await
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for row in rows {
        let content: String = row.get("content");
        let file_path: String = row.get("file_path");
        let embedding_bytes: Vec<u8> = row.get("embedding");
        
        let embedding: Vec<f32> = match bincode::deserialize(&embedding_bytes) {
            Ok(e) => e,
            Err(_) => continue,
        };
        
        let sim = cosine_similarity(&query_embedding, &embedding);
        
        // Only keep results with reasonable similarity (relaxed for better recall)
        if sim > 0.2 {
            results.push(GroundingItem {
                content,
                file_path,
                similarity: Some(sim),
            });
        }
    }

    results.sort_by(|a: &GroundingItem, b: &GroundingItem| {
        b.similarity.unwrap_or(0.0).partial_cmp(&a.similarity.unwrap_or(0.0)).unwrap_or(std::cmp::Ordering::Equal)
    });
    
    results.truncate(top_k as usize);
    
    Ok(results)
}

#[command]
async fn write_to_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(path, content).map_err(|e| e.to_string())
}

#[command]
async fn read_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let cancel_signal: CancelSignal = Arc::new(Notify::new());

    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle().clone();
            tauri::async_runtime::block_on(async move {
                let app_dir = app_handle.path().app_data_dir().unwrap();
                std::fs::create_dir_all(&app_dir).unwrap();
                let db_path = app_dir.join("warbler.db");
                
                use sqlx::sqlite::SqliteConnectOptions;
                let options = SqliteConnectOptions::new()
                    .filename(db_path)
                    .create_if_missing(true);
                
                let pool = SqlitePool::connect_with(options).await.unwrap();
                
                // Run migrations manually
                sqlx::query("CREATE TABLE IF NOT EXISTS chats (
                    id         TEXT PRIMARY KEY,
                    title      TEXT NOT NULL,
                    created_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
                );").execute(&pool).await.unwrap();

                 sqlx::query("CREATE TABLE IF NOT EXISTS settings (
                    key   TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );").execute(&pool).await.unwrap();

                 sqlx::query("INSERT OR IGNORE INTO settings (key, value) VALUES ('embedding_model', 'nomic-embed-text');")
                    .execute(&pool).await.unwrap();
                
                 sqlx::query("INSERT OR IGNORE INTO settings (key, value) VALUES ('chat_model', 'llama3');")
                    .execute(&pool).await.unwrap();

                 sqlx::query("INSERT OR IGNORE INTO settings (key, value) VALUES ('ai_provider', 'ollama');")
                    .execute(&pool).await.unwrap();

                 sqlx::query("INSERT OR IGNORE INTO settings (key, value) VALUES ('ai_base_url', 'http://localhost:11434');")
                    .execute(&pool).await.unwrap();

                 sqlx::query("INSERT OR IGNORE INTO settings (key, value) VALUES ('mcp_max_length', '2500');")
                    .execute(&pool).await.unwrap();
                
                sqlx::query("CREATE TABLE IF NOT EXISTS messages (
                    id       INTEGER PRIMARY KEY AUTOINCREMENT,
                    chat_id  TEXT NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
                    role     TEXT NOT NULL,
                    content  TEXT NOT NULL,
                    images   TEXT,
                    seq      INTEGER NOT NULL
                );").execute(&pool).await.unwrap();

                sqlx::query("CREATE TABLE IF NOT EXISTS grounding_folders (
                    id         TEXT PRIMARY KEY,
                    path       TEXT NOT NULL,
                    created_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
                );").execute(&pool).await.unwrap();

                sqlx::query("CREATE TABLE IF NOT EXISTS grounding_items (
                    id          INTEGER PRIMARY KEY AUTOINCREMENT,
                    folder_id   TEXT REFERENCES grounding_folders(id) ON DELETE CASCADE,
                    file_path   TEXT NOT NULL,
                    content     TEXT NOT NULL,
                    embedding   BLOB
                );").execute(&pool).await.unwrap();

                 sqlx::query("CREATE TABLE IF NOT EXISTS mcp_servers (
                    id       TEXT PRIMARY KEY,
                    name     TEXT NOT NULL,
                    command  TEXT NOT NULL,
                    args     TEXT NOT NULL,
                    env      TEXT NOT NULL,
                    created_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
                );").execute(&pool).await.unwrap();

                 sqlx::query("CREATE TABLE IF NOT EXISTS prompts (
                    id         TEXT PRIMARY KEY,
                    title      TEXT NOT NULL,
                    content    TEXT NOT NULL,
                    created_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
                );").execute(&pool).await.unwrap();
                
                // Version 2 migration
                let _ = sqlx::query("ALTER TABLE messages ADD COLUMN thinking TEXT;").execute(&pool).await;
                let _ = sqlx::query("ALTER TABLE messages ADD COLUMN tool_name TEXT;").execute(&pool).await;

                app_handle.manage(pool);
            });
            Ok(())
        })
        .manage(cancel_signal)
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_http::init())
        .invoke_handler(tauri::generate_handler![
            chat_with_model,
            get_ollama_models,
            cancel_chat,
            get_chats,
            new_chat,
            delete_chat,
            append_message,
            update_chat_title,
            save_model_pref,
            load_model_pref,
            get_grounding_folders,
            delete_grounding_folder,
            clear_all_knowledge,
            index_knowledge_folder,
            search_knowledge,
            list_mcp_servers,
            add_mcp_server,
            delete_mcp_server,
            get_all_mcp_tools,
            call_mcp_tool,
            preprocess_hashtags,
            check_mcp_server,
            get_prompts,
            add_prompt,
            delete_prompt,
            get_config,
            set_config,
            pull_ollama_model,
            write_to_file,
            read_file
        ])
        .plugin(tauri_plugin_dialog::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
