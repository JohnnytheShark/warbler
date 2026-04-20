<script lang="ts">
  import { input, attachments, isSending, useGrounding, useTools, isEmbeddingModelMissing } from "../../lib/stores";
  import { sendMessage } from "../../lib/services";

  async function handleCancel() {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("cancel_chat");
    } catch {}
  }

  function handleFiles(e: Event) {
    const files = Array.from((e.target as HTMLInputElement).files ?? []);
    attachments.set(files);
  }

  function handleKey(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  }

  function handleSubmit(e: Event) {
    e.preventDefault();
    sendMessage();
  }
</script>

<form class="input-row" on:submit={handleSubmit}>
  <label class="file-label" title="Attach image">
    📎
    <input id="file-input" type="file" multiple accept="image/*" on:change={handleFiles} hidden />
  </label>
  {#if $attachments.length > 0}
    <span class="file-badge">{$attachments.length} file{$attachments.length > 1 ? 's' : ''}</span>
  {/if}
  <textarea
    class="chat-input"
    placeholder="Message…"
    bind:value={$input}
    on:keydown={handleKey}
    rows={1}
    disabled={$isSending}
  ></textarea>
  
  <div class="input-actions-col">
      <button class="grounding-toggle {$useGrounding ? 'active' : ''}" type="button" 
              on:click={() => useGrounding.update(v => !v)}
              title="Toggle Grounding (RAG)"
              disabled={$isEmbeddingModelMissing}>
          🎯
      </button>
      <button class="tools-toggle {$useTools ? 'active' : ''}" type="button" 
              on:click={() => useTools.update(v => !v)}
              title="Toggle Tools (MCP)">
          🔌
      </button>
      {#if $isSending}
          <button class="cancel-btn" type="button" on:click={handleCancel} title="Cancel">
              ✕
          </button>
      {:else}
          <button class="send-btn" type="submit" disabled={!$input.trim() && $attachments.length === 0}>
              ↑
          </button>
      {/if}
  </div>
</form>

<style>
  .input-row {
    margin: 16px 24px 24px;
    background: #1a1a1a;
    border: 1px solid #2a2a2a;
    border-radius: 16px;
    display: flex; align-items: flex-end;
    padding: 8px 12px; gap: 10px;
    box-shadow: 0 10px 30px rgba(0,0,0,0.3);
    transition: border-color 0.2s;
  }
  .input-row:focus-within { border-color: #444; }

  .file-label {
    width: 36px; height: 36px; display: flex; align-items: center; justify-content: center;
    color: #666; cursor: pointer; border-radius: 10px; transition: all 0.2s;
    font-size: 1.2rem;
  }
  .file-label:hover { background: #252525; color: #aaa; }

  .chat-input {
    flex: 1; background: none; border: none; color: #eee;
    padding: 10px 0; font-family: inherit; font-size: 1rem;
    outline: none; resize: none; min-height: 24px; max-height: 200px;
    line-height: 1.5;
  }

  .input-actions-col { display: flex; align-items: center; gap: 8px; }

  .grounding-toggle, .tools-toggle {
    width: 32px; height: 32px; border-radius: 8px; border: 1px solid transparent;
    background: none; cursor: pointer; font-size: 1rem; display: flex; 
    align-items: center; justify-content: center; opacity: 0.4; transition: all 0.2s;
  }
  .grounding-toggle:hover:not(:disabled), .tools-toggle:hover { opacity: 0.8; background: #252525; }
  .grounding-toggle.active { opacity: 1; color: #00ff7f; background: rgba(0,255,127,0.1); border-color: rgba(0,255,127,0.2); }
  .tools-toggle.active { opacity: 1; color: #00bfff; background: rgba(0,191,255,0.1); border-color: rgba(0,191,255,0.2); }
  .grounding-toggle:disabled { opacity: 0.1; cursor: not-allowed; }

  .send-btn {
    width: 32px; height: 32px; border-radius: 50%; background: #fff;
    border: none; color: #000; cursor: pointer; font-weight: bold;
    display: flex; align-items: center; justify-content: center;
    transition: all 0.2s; font-size: 1.2rem;
  }
  .send-btn:hover:not(:disabled) { transform: scale(1.05); }
  .send-btn:disabled { background: #333; color: #555; cursor: not-allowed; }

  .cancel-btn {
    width: 32px; height: 32px; border-radius: 50%; background: #ff4747;
    border: none; color: #fff; cursor: pointer; font-weight: bold;
    display: flex; align-items: center; justify-content: center;
    transition: all 0.2s; font-size: 0.8rem;
  }
  .cancel-btn:hover { background: #ff6b6b; }

  .file-badge {
    position: absolute; top: -10px; left: 10px; background: #2563eb;
    color: white; font-size: 0.65rem; padding: 2px 6px; border-radius: 10px;
    border: 2px solid #0d0d0d; font-weight: 600;
  }
</style>
