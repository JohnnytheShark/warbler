use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Notify;
use sqlx::sqlite::SqlitePool;
use tracing_subscriber::EnvFilter;

pub mod models;
pub mod db;
pub mod mcp;
pub mod ai;
pub mod grounding;
pub mod prompts;
pub mod utils;

use ai::CancelSignal;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize tracing logger
    init_logger();
    
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
                
                // Version migrations
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
            ai::chat_with_model,
            ai::get_ollama_models,
            ai::cancel_chat,
            ai::pull_ollama_model,
            db::get_chats,
            db::new_chat,
            db::delete_chat,
            db::append_message,
            db::update_chat_title,
            db::save_model_pref,
            db::load_model_pref,
            grounding::get_grounding_folders,
            grounding::delete_grounding_folder,
            grounding::clear_all_knowledge,
            grounding::index_knowledge_folder,
            grounding::search_knowledge,
            mcp::list_mcp_servers,
            mcp::add_mcp_server,
            mcp::delete_mcp_server,
            mcp::get_all_mcp_tools,
            mcp::call_mcp_tool,
            mcp::preprocess_hashtags,
            mcp::check_mcp_server,
            prompts::get_prompts,
            prompts::add_prompt,
            prompts::delete_prompt,
            utils::get_config,
            utils::set_config,
            utils::write_to_file,
            utils::read_file
        ])
        .plugin(tauri_plugin_dialog::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn init_logger() {
    use tracing_subscriber::fmt;
    
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    
    // Try to create logs directory in a standard location
    let logs_dir = if let Ok(app_data) = std::env::var("APPDATA") {
        let dir = std::path::PathBuf::from(&app_data)
            .join("Warbler")
            .join("logs");
        let _ = std::fs::create_dir_all(&dir);
        Some(dir)
    } else {
        None
    };
    
    let builder = fmt()
        .with_env_filter(env_filter)
        .with_thread_ids(true)
        .with_target(true)
        .with_level(true);
    
    if let Some(logs_dir) = logs_dir {
        let file_appender = tracing_appender::rolling::daily(&logs_dir, "warbler.log");
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
        
        let _ = builder
            .with_writer(non_blocking)
            .with_ansi(false)
            .try_init();
    } else {
        let _ = builder.try_init();
    }
}
