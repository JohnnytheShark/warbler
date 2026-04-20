<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { addMcpServerConfig } from "../../lib/services";

  export let show = false;

  let newMcpConfig = {
    name: "",
    command: "",
    args: "",
    env: [{ key: "", value: "" }]
  };

  async function browseCommand() {
    const selected = await open({
      multiple: false,
      filters: [{ name: 'Executable', extensions: ['exe', ''] }]
    });
    if (selected && typeof selected === 'string') {
      newMcpConfig.command = selected;
    }
  }

  async function handleSave() {
    if (!newMcpConfig.name || !newMcpConfig.command) return;
    await addMcpServerConfig(
        newMcpConfig.name, 
        newMcpConfig.command, 
        newMcpConfig.args, 
        newMcpConfig.env
    );
    show = false;
  }

  function close() {
    show = false;
  }
</script>

{#if show}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div class="modal-overlay" on:click={close} role="button" tabindex="0" on:keydown={(e) => e.key === 'Escape' && close()}>
    <div class="modal" on:click|stopPropagation>
      <div class="modal-header">
        <h3>Add MCP Tool Server</h3>
        <button class="close-btn" on:click={close}>✕</button>
      </div>
      <div class="modal-body">
        <div class="field">
          <label for="mcp-name">Display Name</label>
          <input id="mcp-name" bind:value={newMcpConfig.name} placeholder="e.g. Search Engine" />
        </div>
        <div class="field">
          <label for="mcp-command">Command</label>
          <div class="command-row">
            <input id="mcp-command" bind:value={newMcpConfig.command} placeholder="e.g. /path/to/server.exe or npx" />
            <button type="button" class="browse-btn" on:click={browseCommand} title="Browse for executable">
              📂 Browse
            </button>
          </div>
        </div>
        <div class="field">
          <label for="mcp-args">Arguments (comma separated)</label>
          <input id="mcp-args" bind:value={newMcpConfig.args} placeholder="e.g. -y,@mcp/server-name" />
        </div>
        
        <div class="field">
          <label for="env-vars">Environment Variables</label>
          {#each newMcpConfig.env as env, i}
            <div class="env-row">
              <input bind:value={env.key} placeholder="KEY (e.g. API_KEY)" aria-label="Env variable key" />
              <input bind:value={env.value} type="password" placeholder="value" aria-label="Env variable value" />
              <button class="row-delete" on:click={() => newMcpConfig.env = newMcpConfig.env.filter((_, idx) => idx !== i)}>×</button>
            </div>
          {/each}
          <button class="add-row-btn" on:click={() => newMcpConfig.env = [...newMcpConfig.env, { key: "", value: "" }]}>
            + Add Variable
          </button>
        </div>
      </div>
      <div class="modal-footer">
        <button class="cancel-modal-btn" on:click={close}>Cancel</button>
        <button class="save-modal-btn" on:click={handleSave} disabled={!newMcpConfig.name || !newMcpConfig.command}>
          Save Server
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed; top: 0; left: 0; right: 0; bottom: 0;
    background: rgba(0,0,0,0.85); display: flex; align-items: center; justify-content: center;
    z-index: 2000;
  }
  .modal {
    background: #181818; border: 1px solid #333; border-radius: 12px;
    width: 100%; max-width: 480px; display: flex; flex-direction: column;
    box-shadow: 0 20px 40px rgba(0,0,0,0.5);
  }
  .modal-header {
    padding: 20px; border-bottom: 1px solid #333;
    display: flex; align-items: center; justify-content: space-between;
  }
  .modal-header h3 { font-size: 1.1rem; font-weight: 600; color: #fff; }
  .close-btn { background: none; border: none; color: #777; cursor: pointer; font-size: 1.2rem; }
  .modal-body { padding: 20px; display: flex; flex-direction: column; gap: 16px; max-height: 70vh; overflow-y: auto; }
  .field { display: flex; flex-direction: column; gap: 6px; }
  .field label { font-size: 0.8rem; font-weight: 500; color: #aaa; }
  .field input {
    background: #252525; border: 1px solid #333; color: #fff; padding: 10px; border-radius: 8px;
    font-size: 0.9rem; outline: none; focus: border-color: #555;
  }
  .command-row { display: flex; gap: 10px; }
  .command-row input { flex: 1; }
  .browse-btn {
    background: #333; border: 1px solid #444; color: #eee; padding: 0 16px; border-radius: 8px;
    cursor: pointer; font-size: 0.85rem; font-weight: 500; transition: background 0.2s;
  }
  .browse-btn:hover { background: #444; }
  .env-row { display: flex; gap: 8px; }
  .env-row input { flex: 1; }
  .row-delete { background: none; border: none; color: #555; cursor: pointer; font-size: 1.2rem; }
  .add-row-btn {
    background: none; border: 1px dashed #444; color: #888; padding: 10px; border-radius: 8px;
    cursor: pointer; font-size: 0.85rem; margin-top: 4px; transition: all 0.2s;
  }
  .add-row-btn:hover { color: #aaa; border-color: #666; background: rgba(255,255,255,0.02); }
  .modal-footer { padding: 20px; border-top: 1px solid #333; display: flex; justify-content: flex-end; gap: 12px; }
  .cancel-modal-btn { background: none; border: none; color: #aaa; cursor: pointer; font-size: 0.9rem; font-weight: 500; }
  .save-modal-btn {
    background: #fff; border: none; color: #000; padding: 10px 24px; border-radius: 8px;
    cursor: pointer; font-size: 0.9rem; font-weight: 600;
  }
  .save-modal-btn:disabled { background: #444; color: #888; cursor: not-allowed; }
</style>
