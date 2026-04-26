import { get } from 'svelte/store';
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import * as s from './stores';

// ── Helpers ────────────────────────────────────────────────────────────────
export async function toBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload  = () => resolve((reader.result as string).split(",")[1]);
    reader.onerror = reject;
    reader.readAsDataURL(file);
  });
}

// ── Models ─────────────────────────────────────────────────────────────────
export async function fetchModels() {
  try {
    const raw = await invoke<string>("get_ollama_models");
    const parsed = JSON.parse(raw);
    const models = (parsed.models ?? []).map((m: { name: string }) => m.name);
    s.availableModels.set(models);
    
    const currentModel = get(s.selectedModel);
    if (models.length > 0 && !models.includes(currentModel)) {
      await saveModelPref(models[0]);
    }
    s.ollamaConnected.set(true);
  } catch {
    s.availableModels.set([]);
    s.ollamaConnected.set(false);
  }
}

export async function saveModelPref(m: string) {
  s.selectedModel.set(m);
  await invoke("save_model_pref", { model: m });
}

export async function saveAiProvider(p: string) {
  s.aiProvider.set(p);
  await invoke("set_config", { key: "ai_provider", value: p });
}

export async function saveAiBaseUrl(url: string) {
  s.aiBaseUrl.set(url);
  await invoke("set_config", { key: "ai_base_url", value: url });
}

export async function checkEmbeddingRequirement() {
  const models = get(s.availableModels);
  const embedModel = get(s.selectedEmbeddingModel);
  s.isEmbeddingModelMissing.set(!models.includes(embedModel));
}

export async function saveEmbeddingModel(m: string) {
  const current = get(s.selectedEmbeddingModel);
  if (m === current) return;

  const folders = get(s.groundingFolders);
  if (folders.length > 0) {
    const confirmChange = confirm("Warning: Changing the embedding model will make existing indexed folders incompatible with searches. You will need to re-index them. Proceed?");
    if (!confirmChange) return false;
  }

  s.selectedEmbeddingModel.set(m);
  await invoke("set_config", { key: "embedding_model", value: m });
  await checkEmbeddingRequirement();
  return true;
}

// ── Chat ───────────────────────────────────────────────────────────────────
export async function createNewChat() {
  const id = crypto.randomUUID();
  const title = "New Chat";
  await invoke("new_chat", { id, title });
  s.chats.update(c => [{ id, title, messages: [] }, ...c]);
  s.activeChatId.set(id);
  return id;
}

export async function deleteChat(id: string) {
  await invoke("delete_chat", { id });
  s.chats.update(list => list.filter(c => c.id !== id));
  if (get(s.activeChatId) === id) {
    const remaining = get(s.chats);
    s.activeChatId.set(remaining[0]?.id ?? null);
  }
}

export async function sendMessage() {
  const currentInput = get(s.input).trim();
  const currentFiles = get(s.attachments);
  const activeId = get(s.activeChatId);
  const isSending = get(s.isSending);

  if (isSending || (!currentInput && currentFiles.length === 0)) return;

  let targetId = activeId;
  if (!targetId) {
    targetId = await createNewChat();
  }

  s.isSending.set(true);

  const images: string[] = [];
  for (const f of currentFiles) {
    if (f.type.startsWith("image/")) images.push(await toBase64(f));
  }

  const userMsg: s.OllamaMessage = { role: "user", content: currentInput };
  if (images.length > 0) userMsg.images = images;

  // ── Hashtag Tool Preprocessing ──
  try {
    const processedText = await invoke<string>("preprocess_hashtags", { text: currentInput });
    userMsg.content = processedText;
  } catch (e) {
    console.warn("Hashtag preprocessing failed:", e);
  }

  const chatSnapshot = get(s.chats).find(c => c.id === targetId)!;
  const isFirst = chatSnapshot.messages.length === 0;
  const title = isFirst ? (currentInput.slice(0, 40) || "New Chat") : chatSnapshot.title;

  if (isFirst) {
    await invoke("update_chat_title", { id: targetId, title });
  }

  const seq = chatSnapshot.messages.length;
  await invoke("append_message", { chatId: targetId, msg: userMsg, seq });

  s.chats.update(list => list.map(c => 
    c.id === targetId ? { ...c, title, messages: [...c.messages, userMsg] } : c
  ));

  s.input.set("");
  s.attachments.set([]);

  // ── RAG Context ──
  let groundingContext = "";
  if (get(s.useGrounding)) {
    try {
      const results = await invoke<s.GroundingItem[]>("search_knowledge", {
        query: userMsg.content,
        topK: 20
      });
      if (results.length > 0) {
        groundingContext = "\n\nRelevant Context from your files:\n" + 
          results.map(r => `--- From ${r.file_path} ---\n${r.content}`).join("\n\n");
      }
    } catch (e) {
      console.error("Grounding error:", e);
    }
  }

  try {
    const updatedVisibleChat = get(s.chats).find(c => c.id === targetId)!;
    const messagesWithContext = [...updatedVisibleChat.messages];
    if (groundingContext) {
      const last = messagesWithContext[messagesWithContext.length - 1];
      messagesWithContext[messagesWithContext.length - 1] = {
        ...last,
        content: last.content + groundingContext
      };
    }

    const response = await invoke<string>("chat_with_model", {
      messages: messagesWithContext,
      model: get(s.selectedModel),
      useTools: get(s.useTools)
    });

    let text = "";
    let thinking: string | undefined;
    try {
      const parsed = typeof response === "string" ? JSON.parse(response) : response;
      let raw: string = parsed?.message?.content ?? JSON.stringify(parsed, null, 2);

      if (parsed?.message?.thinking) {
        thinking = parsed.message.thinking;
      }

      if (!thinking) {
        const thinkMatch = raw.match(/^<think>([\s\S]*?)<\/think>\s*/i);
        if (thinkMatch) {
          thinking = thinkMatch[1].trim();
          raw = raw.slice(thinkMatch[0].length);
        }
      }
      text = raw;
    } catch { text = String(response); }

    const assistantMsg: s.OllamaMessage = { role: "assistant", content: text, ...(thinking ? { thinking } : {}) };
    const nextSeq = get(s.chats).find(c => c.id === targetId)!.messages.length;
    await invoke("append_message", { chatId: targetId, msg: assistantMsg, seq: nextSeq });

    s.chats.update(list => list.map(c => 
      c.id === targetId ? { ...c, messages: [...c.messages, assistantMsg] } : c
    ));
  } catch (err) {
    const errStr = String(err);
    if (!errStr.includes("__cancelled__")) {
      const errMsg: s.OllamaMessage = { role: "assistant", content: `[Error: ${errStr}]` };
      const nextSeq = get(s.chats).find(c => c.id === targetId)!.messages.length;
      await invoke("append_message", { chatId: targetId, msg: errMsg, seq: nextSeq });
      s.chats.update(list => list.map(c => 
        c.id === targetId ? { ...c, messages: [...c.messages, errMsg] } : c
      ));
    }
  }

  s.isSending.set(false);
}

// ── Grounding ──────────────────────────────────────────────────────────────
export async function refreshGroundingFolders() {
  const folders = await invoke<s.GroundingFolder[]>("get_grounding_folders");
  s.groundingFolders.set(folders);
}

export async function addKnowledgeFolder() {
  const selected = await open({ directory: true, multiple: false });
  if (selected) {
    s.isIndexing.set(true);
    try {
      await invoke("index_knowledge_folder", { path: selected });
      await refreshGroundingFolders();
    } catch (e) {
      alert("Error indexing folder: " + e);
    } finally {
      s.isIndexing.set(false);
    }
  }
}

// ── MCP ────────────────────────────────────────────────────────────────────
export async function refreshMcpServers() {
  const servers = await invoke<s.McpServerConfig[]>("list_mcp_servers");
  s.mcpServers.set(servers);
}

export async function checkAllMcpServers() {
  const servers = get(s.mcpServers);
  for (const server of servers) {
    s.mcpStatus.update(m => ({ ...m, [server.id]: 'checking' }));
    invoke<boolean>("check_mcp_server", { id: server.id })
      .then(ok => s.mcpStatus.update(m => ({ ...m, [server.id]: ok ? 'connected' : 'error' })))
      .catch(() => s.mcpStatus.update(m => ({ ...m, [server.id]: 'error' })));
  }
}

export async function removeMcpServer(id: string) {
  await invoke("delete_mcp_server", { id });
  await refreshMcpServers();
}

export async function addMcpServerConfig(name: string, command: string, args: string, env: {key: string, value: string}[]) {
  const config: s.McpServerConfig = {
    id: crypto.randomUUID(),
    name,
    command,
    args: args.split(",").map(a => a.trim()).filter(Boolean),
    env: env.filter(e => e.key).map(e => [e.key, e.value])
  };

  await invoke("add_mcp_server", { config });
  await refreshMcpServers();
  await checkAllMcpServers();
}

export async function saveMcpMaxLength(limit: number) {
  s.mcpMaxLength.set(limit);
  await invoke("set_config", { key: "mcp_max_length", value: limit.toString() });
}
