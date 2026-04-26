use tauri::{command, Emitter};
use sqlx::sqlite::SqlitePool;
use sqlx::Row;
use serde_json;
use crate::models::{McpServerConfig, McpTool};

#[command]
pub async fn list_mcp_servers(pool: tauri::State<'_, SqlitePool>) -> Result<Vec<McpServerConfig>, String> {
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
pub async fn add_mcp_server(
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
pub async fn delete_mcp_server(pool: tauri::State<'_, SqlitePool>, id: String) -> Result<(), String> {
    sqlx::query("DELETE FROM mcp_servers WHERE id = ?").bind(&id).execute(&*pool).await.map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn process_mcp_request(
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

    let initialized_notif = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    stdin.write_all(format!("{}\n", initialized_notif).as_bytes()).await
        .map_err(|e: std::io::Error| e.to_string())?;

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

    let _ = child.kill().await;

    let json: serde_json::Value = serde_json::from_str(&response).map_err(|e| e.to_string())?;
    if let Some(error) = json.get("error") {
        return Err(error.to_string());
    }
    
    Ok(json["result"].clone())
}

#[command]
pub async fn get_all_mcp_tools(pool: tauri::State<'_, SqlitePool>) -> Result<Vec<McpTool>, String> {
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
pub async fn call_mcp_tool(
    pool: tauri::State<'_, SqlitePool>,
    name: String,
    arguments: serde_json::Value
) -> Result<serde_json::Value, String> {
    let servers = list_mcp_servers(pool.clone()).await?;
    
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

pub async fn call_mcp_tool_internal(
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
pub async fn preprocess_hashtags(
    app_handle: tauri::AppHandle,
    pool: tauri::State<'_, SqlitePool>,
    text: String,
) -> Result<String, String> {
    preprocess_hashtags_internal(app_handle, pool.inner().clone(), text).await
}

pub async fn preprocess_hashtags_internal(
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

                let _ = app_handle.emit("tool-result", serde_json::json!({ "name": tag, "content": tool_output.clone() }));
                augmented_text.push_str(&format!("\n\n--- Tool Output (#{}) ---\n{}", tag, tool_output));
            },
            Err(_) => {}
        }
    }

    Ok(augmented_text)
}

#[command]
pub async fn check_mcp_server(
    pool: tauri::State<'_, SqlitePool>,
    id: String
) -> Result<bool, String> {
    let servers = list_mcp_servers(pool).await?;
    let server = servers.into_iter().find(|s| s.id == id).ok_or("Server not found")?;
    
    match process_mcp_request(&server.command, &server.args, &server.env, "tools/list", serde_json::json!({})).await {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}
