use tauri::{command, Emitter};
use std::sync::Arc;
use tokio::sync::Notify;
use sqlx::sqlite::SqlitePool;
use sqlx::Row;
use crate::mcp::{get_all_mcp_tools, call_mcp_tool, preprocess_hashtags_internal};
use crate::CancelSignal;
use tracing::{info, error, debug};

#[command]
pub async fn cancel_chat(signal: tauri::State<'_, CancelSignal>) -> Result<(), String> {
    info!("Cancellation requested");
    let notify = {
        let lock = signal.0.read().await;
        lock.clone()
    };
    notify.notify_one();
    Ok(())
}

#[command]
pub async fn get_ollama_models(
    pool: tauri::State<'_, SqlitePool>,
    client: tauri::State<'_, reqwest::Client>,
) -> Result<String, String> {
    let provider = sqlx::query("SELECT value FROM settings WHERE key = 'ai_provider'")
        .fetch_optional(&*pool).await.map_err(|e| e.to_string())?
        .map(|r| r.get::<String, _>("value")).unwrap_or_else(|| "ollama".to_string());
    let base_url = sqlx::query("SELECT value FROM settings WHERE key = 'ai_base_url'")
        .fetch_optional(&*pool).await.map_err(|e| e.to_string())?
        .map(|r| r.get::<String, _>("value")).unwrap_or_else(|| "http://localhost:11434".to_string());
    let api_key = sqlx::query("SELECT value FROM settings WHERE key = 'ai_api_key'")
        .fetch_optional(&*pool).await.map_err(|e| e.to_string())?
        .map(|r| r.get::<String, _>("value"));

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
pub async fn chat_with_model(
    app_handle: tauri::AppHandle,
    pool: tauri::State<'_, SqlitePool>,
    client: tauri::State<'_, reqwest::Client>,
    messages: Vec<serde_json::Value>,
    model: String,
    use_tools: bool,
    signal: tauri::State<'_, CancelSignal>,
) -> Result<String, String> {
    // 1. Initialize cancellation signal for this request
    let current_notify = Arc::new(Notify::new());
    {
        let mut lock = signal.0.write().await;
        *lock = current_notify.clone();
    }

    info!("Starting chat session with model: {}", model);

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

    debug!("Using provider: {}, endpoint: {}", provider, chat_endpoint);

    let mut current_messages = messages.clone();
    
    // 2. Preprocess hashtags
    if let Some(last_msg) = current_messages.iter_mut().rev().find(|m| m["role"] == "user") {
        if let Some(content) = last_msg["content"].as_str() {
             info!("Preprocessing hashtags for user message");
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

    // 3. Prepare tools
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
                if ollama_tools.is_empty() {
                    serde_json::Value::Null
                } else {
                    serde_json::json!(ollama_tools)
                }
            },
            Err(e) => {
                error!("Failed to fetch MCP tools: {}", e);
                serde_json::Value::Null
            },
        }
    } else {
        serde_json::Value::Null
    };

    // 4. Main interaction loop (for tool calls)
    for i in 0..5 {
        info!("Chat loop iteration {}", i + 1);
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

        debug!("Sending request to LLM...");
        let request = request_builder.body(payload.to_string()).send();
        let res = tokio::select! {
            result = request => result.map_err(|e: reqwest::Error| {
                error!("Request failed: {}", e);
                e.to_string()
            })?,
            _ = current_notify.notified() => {
                info!("Request cancelled during send");
                return Err("__cancelled__".to_string());
            },
        };

        if !res.status().is_success() {
            let status = res.status();
            let error_text = res.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            error!("LLM returned error status: {}, body: {}", status, error_text);
            return Err(format!("LLM Error ({}): {}", status, error_text));
        }

        let res_json: serde_json::Value = tokio::select! {
            result = res.json() => result.map_err(|e: reqwest::Error| {
                error!("Failed to parse response JSON: {}", e);
                e.to_string()
            })?,
            _ = current_notify.notified() => {
                info!("Request cancelled during JSON parsing");
                return Err("__cancelled__".to_string());
            },
        };

        // Check for provider-level error messages in successful HTTP response
        if let Some(err) = res_json.get("error") {
            let err_msg = if err.is_string() { err.as_str().unwrap().to_string() } else { err.to_string() };
            error!("LLM returned error in JSON: {}", err_msg);
            return Err(format!("LLM Error: {}", err_msg));
        }

        let (message, content, thinking) = if is_ollama {
            let msg = res_json["message"].clone();
            if msg.is_null() {
                error!("Ollama response missing 'message' field: {}", res_json);
                return Err("Ollama response missing 'message' field".to_string());
            }
            let c = msg["content"].as_str().unwrap_or_default().to_string();
            let t = msg["thinking"].as_str().map(|s| s.to_string());
            (msg, c, t)
        } else {
            let choices = res_json["choices"].as_array();
            if choices.is_none() || choices.unwrap().is_empty() {
                error!("OpenAI response missing 'choices' field or empty: {}", res_json);
                return Err("OpenAI response missing 'choices' field".to_string());
            }
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
                    "thinking": if accumulated_thinking.is_empty() { None } else { Some(accumulated_thinking.clone()) }
                }
            }).to_string()
        };
        
        if !content.is_empty() || thinking.is_some() {
            final_json_response = current_json_str;
        }

        if let Some(tool_calls) = message["tool_calls"].as_array() {
            if tool_calls.is_empty() { 
                info!("Generation complete (no tool calls)");
                return Ok(final_json_response); 
            }
            
            info!("Model requested {} tool calls", tool_calls.len());
            current_messages.push(message.clone());
            
            for call in tool_calls {
                let tool_name = call["function"]["name"].as_str().unwrap_or_default();
                let tool_args = &call["function"]["arguments"];
                let args_val: serde_json::Value = if tool_args.is_string() {
                    serde_json::from_str(tool_args.as_str().unwrap()).unwrap_or(serde_json::json!({}))
                } else {
                    tool_args.clone()
                };

                info!("Executing tool: {}", tool_name);
                let _ = app_handle.emit("tool-call", &tool_name);
                let result = match call_mcp_tool(pool.clone(), tool_name.to_string(), args_val).await {
                    Ok(r) => {
                        if let Some(parts) = r.get("content").and_then(|c| c.as_array()) {
                            parts.iter().filter_map(|p| p.get("text").and_then(|txt| txt.as_str())).collect::<Vec<_>>().join("\n")
                        } else { r.to_string() }
                    },
                    Err(e) => {
                        error!("Tool execution failed ({}): {}", tool_name, e);
                        format!("Error: {}", e)
                    },
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
            info!("Generation complete");
            return Ok(final_json_response);
        }
    }
    
    info!("Exceeded max tool call iterations (5)");
    Ok(final_json_response)
}

#[command]
pub async fn pull_ollama_model(app_handle: tauri::AppHandle, model: String) -> Result<(), String> {
    use futures_util::StreamExt;
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
