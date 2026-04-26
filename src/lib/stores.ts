import { writable, derived } from 'svelte/store';
import { invoke } from "@tauri-apps/api/core";

// ── Types ──────────────────────────────────────────────────────────────────
export interface OllamaMessage {
  role: "user" | "assistant" | "system";
  content: string;
  images?: string[];
  thinking?: string;
  tool_name?: string;
}

export interface Chat {
  id: string;
  title: string;
  messages: OllamaMessage[];
}

export interface GroundingFolder {
  id: string;
  path: string;
}

export interface GroundingItem {
  content: string;
  file_path: string;
  similarity?: number;
}

export interface McpServerConfig {
  id: string;
  name: string;
  command: string;
  args: string[];
  env: [string, string][];
}

export interface McpTool {
  name: string;
  description?: string;
  input_schema: any;
}

// ── Chat Store ─────────────────────────────────────────────────────────────
export const chats = writable<Chat[]>([]);
export const activeChatId = writable<string | null>(null);

export const activeChat = derived(
  [chats, activeChatId],
  ([$chats, $activeChatId]) => $chats.find(c => c.id === $activeChatId) ?? null
);

export const messages = derived(
  activeChat,
  ($activeChat) => $activeChat?.messages ?? []
);

// ── App State Store ────────────────────────────────────────────────────────
export const isSending = writable(false);
export const ollamaConnected = writable(false);
export const systemError = writable("");

export const isIndexing = writable(false);
export const indexingStatus = writable("");
export const indexingProgress = writable(0);

export const isPulling = writable(false);
export const pullStatus = writable("");
export const pullProgress = writable(0);

// ── Config Store ───────────────────────────────────────────────────────────
export const availableModels = writable<string[]>([]);
export const selectedModel = writable("gemma4:e4b");
export const selectedEmbeddingModel = writable("nomic-embed-text");
export const isEmbeddingModelMissing = writable(false);

export const aiProvider = writable("ollama");
export const aiBaseUrl = writable("http://localhost:11434");

// ── Feature State ──────────────────────────────────────────────────────────
export const groundingFolders = writable<GroundingFolder[]>([]);
export const mcpServers = writable<McpServerConfig[]>([]);
export const mcpStatus = writable<Record<string, 'connected' | 'error' | 'checking'>>({});
export const useGrounding = writable(false);
export const useTools = writable(false);
export const mcpMaxLength = writable(2500);
export const availableTools = writable<McpTool[]>([]);
export const activeToolCall = writable<string | null>(null);

// ── Input Store ────────────────────────────────────────────────────────────
export const input = writable("");
export const attachments = writable<File[]>([]);
