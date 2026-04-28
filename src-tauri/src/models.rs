use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OllamaMessage {
    pub role: String,
    pub content: String,
    pub images: Option<Vec<String>>,
    pub thinking: Option<String>,
    pub tool_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Chat {
    pub id: String,
    pub title: String,
    pub messages: Vec<OllamaMessage>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroundingFolder {
    pub id: String,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroundingItem {
    pub content: String,
    pub file_path: String,
    pub similarity: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Prompt {
    pub id: String,
    pub title: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OllamaToolCall {
    pub function: OllamaFunctionCall,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OllamaFunctionCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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
