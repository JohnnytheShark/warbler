<script lang="ts">
  import type { OllamaMessage } from "../../lib/stores";
  import { parseMarkdown } from "../../lib/markdown";

  export let msg: OllamaMessage;

  $: renderedContent = msg.role === "assistant" ? parseMarkdown(msg.content) : msg.content;
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
        {#if msg.role === "assistant"}
          <div class="markdown-body">
            {@html renderedContent}
          </div>
        {:else}
          <p class="bubble-text">{msg.content}</p>
        {/if}
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
    max-width: 85%;
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
    width: 100%;
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

  /* --- Markdown Body Styles --- */
  .markdown-body {
    font-size: 0.95rem;
    line-height: 1.6;
    color: #e0e0e0;
  }
  
  :global(.markdown-body p) { margin-bottom: 12px; }
  :global(.markdown-body p:last-child) { margin-bottom: 0; }
  
  :global(.markdown-body h1, .markdown-body h2, .markdown-body h3) {
    margin: 20px 0 10px;
    font-weight: 600;
    line-height: 1.25;
    color: #fff;
  }
  :global(.markdown-body h1) { font-size: 1.4rem; border-bottom: 1px solid #333; padding-bottom: 0.3em; }
  :global(.markdown-body h2) { font-size: 1.2rem; }
  :global(.markdown-body h3) { font-size: 1.1rem; }
  
  :global(.markdown-body ul, .markdown-body ol) {
    margin-bottom: 12px;
    padding-left: 20px;
  }
  :global(.markdown-body li) { margin-bottom: 4px; }
  
  :global(.markdown-body code) {
    padding: 0.2em 0.4em;
    margin: 0;
    font-size: 85%;
    background-color: rgba(255,255,255,0.1);
    border-radius: 6px;
    font-family: "JetBrains Mono", "Fira Code", monospace;
  }
  
  :global(.markdown-body pre) {
    padding: 16px;
    overflow: auto;
    font-size: 85%;
    line-height: 1.45;
    background-color: #0d0d0d;
    border-radius: 12px;
    border: 1px solid #2a2a2a;
    margin-bottom: 16px;
  }
  
  :global(.markdown-body pre code) {
    padding: 0;
    margin: 0;
    background-color: transparent;
    border: 0;
    font-size: 100%;
    color: #d1d5db;
    display: block;
    white-space: pre;
  }

  :global(.code-block-container) {
    margin-bottom: 16px;
  }
  :global(.code-block-header) {
    background: #2a2a2a;
    color: #888;
    padding: 4px 12px;
    font-size: 0.75rem;
    font-family: inherit;
    border-top-left-radius: 12px;
    border-top-right-radius: 12px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    border: 1px solid #2a2a2a;
    border-bottom: none;
  }
  :global(.code-block-header + pre) {
    border-top-left-radius: 0;
    border-top-right-radius: 0;
  }

  :global(.markdown-body blockquote) {
    padding: 0 1em;
    color: #8b949e;
    border-left: 0.25em solid #30363d;
    margin: 0 0 16px 0;
  }

  /* --- Other Chat Blocks --- */
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
