use tauri::{command, Emitter};
use sqlx::sqlite::SqlitePool;
use sqlx::Row;
use serde_json;
use crate::models::{McpServerConfig, McpTool};
use tracing::{info, error, debug, warn};

#[command]
pub async fn list_mcp_servers(pool: tauri::State<'_, SqlitePool>) -> Result<Vec<McpServerConfig>, String> {
    info!("Fetching all MCP servers from database");
    let rows = sqlx::query("SELECT id, name, command, args, env FROM mcp_servers")
        .fetch_all(&*pool)
        .await
        .map_err(|e| {
            error!("Failed to fetch MCP servers: {}", e);
            e.to_string()
        })?;

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
    info!("Successfully loaded {} MCP servers", servers.len());
    Ok(servers)
}

#[command]
pub async fn add_mcp_server(
    pool: tauri::State<'_, SqlitePool>,
    config: McpServerConfig
) -> Result<(), String> {
    info!("Adding MCP server: {} (command: {})", config.name, config.command);
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
        .map_err(|e| {
            error!("Failed to add MCP server {}: {}", config.name, e);
            e.to_string()
        })?;
    info!("Successfully added MCP server: {}", config.name);
    Ok(())
}

#[command]
pub async fn delete_mcp_server(pool: tauri::State<'_, SqlitePool>, id: String) -> Result<(), String> {
    info!("Deleting MCP server with id: {}", id);
    sqlx::query("DELETE FROM mcp_servers WHERE id = ?")
        .bind(&id)
        .execute(&*pool)
        .await
        .map_err(|e| {
            error!("Failed to delete MCP server {}: {}", id, e);
            e.to_string()
        })?;
    info!("Successfully deleted MCP server: {}", id);
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
    use tokio::time::{timeout, Duration};
    use std::process::Stdio;

    debug!("Starting MCP request - command: {}, method: {}", command, method);

    let operation = async {
        debug!("Spawning MCP process: {} with args: {:?}", command, args);
        let mut child = Command::new(command)
            .args(args)
            .envs(env.iter().cloned())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                error!("Failed to spawn MCP process '{}': {}", command, e);
                format!("Failed to spawn '{}': {}", command, e)
            })?;

        debug!("MCP process spawned successfully");
        let mut stdin = child.stdin.take().ok_or_else(|| {
            error!("Failed to open stdin for MCP process");
            "Failed to open stdin"
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            error!("Failed to open stdout for MCP process");
            "Failed to open stdout"
        })?;
        let mut stderr = child.stderr.take().map(BufReader::new);
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
        debug!("Sending MCP initialization request");
        stdin.write_all(format!("{}\n", init_req).as_bytes()).await
            .map_err(|e| {
                error!("Failed to write init request: {}", e);
                format!("Failed to write init: {}", e)
            })?;

        let _init_resp = reader.next_line().await
            .map_err(|e| {
                error!("Failed to read init response: {}", e);
                format!("Failed to read init response: {}", e)
            })?;
        debug!("Received MCP initialization response");

        let initialized_notif = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });
        debug!("Sending initialized notification");
        stdin.write_all(format!("{}\n", initialized_notif).as_bytes()).await
            .map_err(|e| {
                error!("Failed to write initialized notification: {}", e);
                format!("Failed to write initialized notification: {}", e)
            })?;

        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": method,
            "params": params
        });
        debug!("Sending MCP request for method: {}", method);
        stdin.write_all(format!("{}\n", req).as_bytes()).await
            .map_err(|e| {
                error!("Failed to write request: {}", e);
                format!("Failed to write request: {}", e)
            })?;

        let response = reader.next_line().await
            .map_err(|e| {
                error!("Failed to read response: {}", e);
                format!("Failed to read response: {}", e)
            })?
            .ok_or_else(|| {
                error!("No response from MCP server (EOF)");
                "No response from server (EOF)".to_string()
            })?;

        let _ = child.kill().await;

        debug!("Parsing MCP response");
        let json: serde_json::Value = serde_json::from_str(&response)
            .map_err(|e| {
                error!("Failed to parse MCP response: {}", e);
                format!("Failed to parse response: {}", e)
            })?;

        if let Some(error) = json.get("error") {
            let err_msg = error.to_string();
            error!("MCP server returned error: {}", err_msg);
            return Err(err_msg);
        }

        info!("Successfully received MCP response for method: {}", method);
        Ok(json["result"].clone())
    };

    timeout(Duration::from_secs(15), operation)
        .await
        .map_err(|_| {
            error!("MCP server timed out after 15 seconds for method: {}", method);
            "MCP server timed out after 15 seconds".to_string()
        })?
}

#[command]
pub async fn get_all_mcp_tools(pool: tauri::State<'_, SqlitePool>) -> Result<Vec<McpTool>, String> {
    info!("Fetching all MCP tools from all servers");
    let servers = list_mcp_servers(pool).await?;
    let mut all_tools = Vec::new();

    for server in servers {
        debug!("Fetching tools from server: {}", server.name);
        match process_mcp_request(&server.command, &server.args, &server.env, "tools/list", serde_json::json!({})).await {
            Ok(result) => {
                if let Some(tools) = result["tools"].as_array() {
                    info!("Found {} tools in server: {}", tools.len(), server.name);
                    for t in tools {
                        all_tools.push(McpTool {
                            name: t["name"].as_str().unwrap_or_default().to_string(),
                            description: t["description"].as_str().map(|s| s.to_string()),
                            input_schema: t["inputSchema"].clone(),
                        });
                    }
                }
            },
            Err(e) => {
                warn!("Error listing tools for {}: {}", server.name, e);
            }
        }
    }
    info!("Successfully fetched {} total tools from all servers", all_tools.len());
    Ok(all_tools)
}

#[command]
pub async fn call_mcp_tool(
    pool: tauri::State<'_, SqlitePool>,
    name: String,
    arguments: serde_json::Value
) -> Result<serde_json::Value, String> {
    info!("Calling MCP tool: {} with arguments: {}", name, arguments);
    let servers = list_mcp_servers(pool.clone()).await?;
    
    for server in servers {
        debug!("Checking server {} for tool {}", server.name, name);
        let tools_res = process_mcp_request(&server.command, &server.args, &server.env, "tools/list", serde_json::json!({})).await;
        if let Ok(res) = tools_res {
             if res["tools"].as_array().map_or(false, |ts| ts.iter().any(|t| t["name"] == name)) {
                info!("Found tool {} in server {}, executing...", name, server.name);
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
    let err = format!("Tool {} not found in any server", name);
    error!("{}", err);
    Err(err)
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
    info!("Checking MCP server connection for id: {}", id);
    let servers = list_mcp_servers(pool).await?;
    let server = servers.into_iter().find(|s| s.id == id).ok_or_else(|| {
        error!("MCP server with id {} not found", id);
        "Server not found"
    })?;
    
    debug!("Attempting to connect to server: {}", server.name);
    match process_mcp_request(&server.command, &server.args, &server.env, "tools/list", serde_json::json!({})).await {
        Ok(_) => {
            info!("Successfully connected to MCP server: {}", server.name);
            Ok(true)
        },
        Err(e) => {
            warn!("Failed to connect to MCP server {} ({}): {}", server.name, id, e);
            Ok(false)
        }
    }
}
