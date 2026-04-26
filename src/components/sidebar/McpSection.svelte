<script lang="ts">
  import { mcpServers, mcpStatus, mcpMaxLength } from "../../lib/stores";
  import { removeMcpServer, saveMcpMaxLength } from "../../lib/services";
  import { createEventDispatcher } from "svelte";

  const dispatch = createEventDispatcher();

  function onAdd() {
    dispatch("add");
  }
</script>

<div class="mcp-section">
  <div class="section-header">
    <span class="section-title">Tools (MCP)</span>
    <button class="add-kb-btn" on:click={onAdd}>＋</button>
  </div>
  <div class="kb-list">
    {#each $mcpServers as server}
        <div class="kb-item">
            <div class="status-indicator-mini {$mcpStatus[server.id] || 'checking'}" 
                 title="Status: {$mcpStatus[server.id] || 'checking'}"></div>
            <span class="kb-path" title="{server.command} {server.args.join(' ')}">{server.name}</span>
            <button class="kb-delete" on:click={() => removeMcpServer(server.id)}>✕</button>
        </div>
    {/each}
    {#if $mcpServers.length === 0}
        <p class="kb-hint">No tools added.</p>
    {/if}
  </div>

  <div class="mcp-config-row">
    <div class="config-label-row">
      <span>Truncation Limit</span>
      <span class="value-badge">{$mcpMaxLength} chars</span>
    </div>
    <input type="range" min="100" max="100000" step="100" 
           bind:value={$mcpMaxLength} 
           on:change={(e) => saveMcpMaxLength(parseInt(e.currentTarget.value))} />
  </div>
</div>

<style>
  .mcp-section { padding: 14px; border-top: 1px solid #222; margin-top: 0; padding-top: 20px; }
  .section-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 12px; }
  .section-title { font-size: 0.7rem; font-weight: 600; color: #555; text-transform: uppercase; letter-spacing: 0.05em; }

  .add-kb-btn {
    background: #252525; border: 1px solid #333; color: #eee; width: 22px; height: 22px;
    border-radius: 4px; display: flex; align-items: center; justify-content: center;
    cursor: pointer; font-size: 0.9rem; transition: background 0.2s;
  }
  .add-kb-btn:hover { background: #333; }

  .kb-list { display: flex; flex-direction: column; gap: 4px; }
  .kb-item {
    display: flex; align-items: center; background: #181818; border: 1px solid #222;
    padding: 6px 10px; border-radius: 6px; gap: 8px;
  }
  .kb-path { flex: 1; font-size: 0.75rem; color: #bbb; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .kb-delete { background: none; border: none; color: #444; cursor: pointer; font-size: 0.75rem; }
  .kb-delete:hover { color: #ff4747; }
  .kb-hint { font-size: 0.7rem; color: #444; text-align: center; padding: 10px 0; }

  .status-indicator-mini {
    width: 6px; height: 6px; border-radius: 50%;
    background: #444; transition: background 0.3s;
  }
  .status-indicator-mini.connected { background: #00ff7f; box-shadow: 0 0 6px rgba(0, 255, 127, 0.4); }
  .status-indicator-mini.error { background: #ff4747; box-shadow: 0 0 6px rgba(255, 71, 71, 0.4); }
  .status-indicator-mini.checking { background: #ffaa00; box-shadow: 0 0 6px rgba(255, 170, 0, 0.4); }

  .mcp-config-row { margin-top: 16px; padding-top: 16px; border-top: 1px solid #1a1a1a; display: flex; flex-direction: column; gap: 8px; }
  .config-label-row { display: flex; justify-content: space-between; align-items: center; font-size: 0.7rem; color: #555; font-weight: 600; text-transform: uppercase; }
  .value-badge { background: #222; color: #aaa; padding: 2px 6px; border-radius: 4px; font-family: monospace; text-transform: none; }

  input[type="range"] {
    -webkit-appearance: none; width: 100%; height: 4px; background: #222; border-radius: 2px; outline: none; margin: 8px 0;
  }
  input[type="range"]::-webkit-slider-thumb {
    -webkit-appearance: none; width: 12px; height: 12px; background: #555; border-radius: 50%; cursor: pointer; transition: background 0.2s;
  }
  input[type="range"]::-webkit-slider-thumb:hover { background: #888; }
</style>
