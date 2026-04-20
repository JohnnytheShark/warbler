<script lang="ts">
  import { mcpServers, mcpStatus } from "../../lib/stores";
  import { removeMcpServer } from "../../lib/services";
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
</style>
