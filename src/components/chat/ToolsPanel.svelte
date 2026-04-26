<script lang="ts">
  import { availableTools, useTools, input } from "../../lib/stores";
  import { fade } from "svelte/transition";

  function insertTool(name: string) {
    input.update(v => {
      const trimmed = v.trim();
      if (trimmed === "") return `#${name} `;
      return `${trimmed} #${name} `;
    });
    
    // Focus the input field
    const el = document.querySelector('.chat-input') as HTMLTextAreaElement;
    if (el) el.focus();
  }
</script>

{#if $availableTools.length > 0}
  <div class="tools-panel {$useTools ? 'active' : ''}" transition:fade={{ duration: 200 }}>
    <div class="panel-header">
      <span class="icon">🔌</span>
      <span class="title">Available Tools</span>
      {#if $useTools}
        <span class="status-tag active">Active</span>
      {:else}
        <span class="status-tag">Disabled</span>
      {/if}
      <span class="count">{$availableTools.length}</span>
    </div>
    <div class="tools-list">
      {#each $availableTools as tool}
        <button class="tool-tag" 
                title={tool.description || 'No description available'}
                on:click={() => insertTool(tool.name)}>
          <span class="tool-name">#{tool.name}</span>
        </button>
      {/each}
    </div>
  </div>
{/if}

<style>
  .tools-panel {
    background: #141414;
    border-bottom: 1px solid #222;
    padding: 12px 24px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    opacity: 0.6;
    transition: all 0.3s;
  }
  .tools-panel.active {
    opacity: 1;
    background: rgba(0, 191, 255, 0.03);
    border-bottom-color: rgba(0, 191, 255, 0.2);
  }

  .panel-header {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.75rem;
    font-weight: 600;
    color: #666;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .count {
    background: #222;
    padding: 1px 6px;
    border-radius: 10px;
    font-size: 0.65rem;
    margin-left: auto;
  }

  .status-tag {
    font-size: 0.6rem;
    padding: 2px 6px;
    border-radius: 4px;
    background: #222;
    color: #888;
    margin-left: 8px;
  }

  .status-tag.active {
    background: rgba(0, 191, 255, 0.1);
    color: #00bfff;
    border: 1px solid rgba(0, 191, 255, 0.2);
  }

  .tools-list {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .tool-tag {
    background: #1e1e1e;
    border: 1px solid #333;
    padding: 4px 10px;
    border-radius: 12px;
    font-size: 0.8rem;
    color: #00bfff;
    cursor: pointer;
    transition: all 0.2s;
    font-family: inherit;
    display: flex;
    align-items: center;
  }

  .tool-tag:hover {
    background: #252525;
    border-color: #00bfff66;
    transform: translateY(-1px);
  }

  .tool-name {
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
  }
</style>
