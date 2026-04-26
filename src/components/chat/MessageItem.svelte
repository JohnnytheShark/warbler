<script lang="ts">
  import type { OllamaMessage } from "../../lib/stores";

  export let msg: OllamaMessage;
</script>

<div class="bubble-row {msg.role}">
  <div class="avatar">{msg.role === "user" ? "🐦" : "🦙"}</div>
  <div class="bubble-wrap">
    {#if msg.thinking}
      <details class="thinking-block">
        <summary class="thinking-summary">💭 Thoughts</summary>
        <p class="thinking-text">{msg.thinking}</p>
      </details>
    {/if}
    {#if msg.tool_name}
      <details class="tool-result-block">
        <summary class="tool-result-summary"
          >🔧 Tool Output: #{msg.tool_name}</summary
        >
        <pre class="tool-result-text">{msg.content}</pre>
      </details>
    {:else}
      <div class="bubble">
        {#if msg.images}
          <div class="bubble-images">
            {#each msg.images as img}
              <img
                src="data:image/png;base64,{img}"
                alt="attachment"
                class="bubble-img"
              />
            {/each}
          </div>
        {/if}
        <p class="bubble-text">{msg.content}</p>
      </div>
    {/if}
  </div>
</div>

<style>
  .bubble-row {
    display: flex;
    gap: 12px;
    margin-bottom: 24px;
    width: 100%;
  }
  .bubble-row.user {
    flex-direction: row-reverse;
  }

  .avatar {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: #222;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 1rem;
    flex-shrink: 0;
    margin-top: 4px;
  }

  .bubble-wrap {
    max-width: 80%;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .bubble-row.user .bubble-wrap {
    align-items: flex-end;
  }

  .bubble {
    padding: 12px 16px;
    border-radius: 18px;
    line-height: 1.5;
    font-size: 0.95rem;
    position: relative;
    word-wrap: break-word;
    transition: all 0.2s;
  }
  .bubble-row.user .bubble {
    background: #2563eb;
    color: #fff;
    border-bottom-right-radius: 4px;
  }
  .bubble-row.assistant .bubble {
    background: #1e1e1e;
    color: #e5e5e5;
    border-bottom-left-radius: 4px;
    border: 1px solid #2a2a2a;
  }

  .bubble-images {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 8px;
  }
  .bubble-img {
    max-width: 200px;
    max-height: 200px;
    border-radius: 8px;
    border: 1px solid #333;
  }

  .bubble-text {
    white-space: pre-wrap;
  }

  /* Thinking block styles from global styles */
  .thinking-block {
    margin-bottom: 4px;
    background: rgba(255, 255, 255, 0.03);
    border-radius: 8px;
    border-left: 3px solid #444;
    overflow: hidden;
  }
  .thinking-summary {
    padding: 6px 12px;
    font-size: 0.8rem;
    color: #666;
    cursor: pointer;
    outline: none;
    list-style: none;
    user-select: none;
    font-weight: 500;
  }
  .thinking-summary::-webkit-details-marker {
    display: none;
  }
  .thinking-text {
    padding: 4px 12px 10px;
    font-size: 0.85rem;
    color: #888;
    font-style: italic;
    line-height: 1.4;
  }

  /* Tool result block styles */
  .tool-result-block {
    margin-bottom: 4px;
    background: rgba(0, 191, 255, 0.03);
    border-radius: 8px;
    border-left: 3px solid #00bfff;
    overflow: hidden;
  }
  .tool-result-summary {
    padding: 6px 12px;
    font-size: 0.8rem;
    color: #00bfff;
    cursor: pointer;
    outline: none;
    list-style: none;
    user-select: none;
    font-weight: 500;
  }
  .tool-result-summary::-webkit-details-marker {
    display: none;
  }
  .tool-result-text {
    padding: 8px 12px 12px;
    font-size: 0.85rem;
    color: #aaa;
    line-height: 1.4;
    white-space: pre-wrap;
    font-family: "JetBrains Mono", monospace;
    max-height: 300px;
    overflow-y: auto;
  }
</style>
