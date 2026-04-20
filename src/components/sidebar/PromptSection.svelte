<script lang="ts">
  import { onMount, createEventDispatcher } from 'svelte';
  import { invoke } from "@tauri-apps/api/core";
  import { save, open } from "@tauri-apps/plugin-dialog";
  import { input } from "../../lib/stores";

  const dispatch = createEventDispatcher();

  interface Prompt {
    id: string;
    title: string;
    content: string;
  }

  let prompts: Prompt[] = [];
  let isLoading = false;

  export async function refresh() {
    isLoading = true;
    try {
      prompts = await invoke<Prompt[]>("get_prompts");
    } catch (e) {
      console.error("Failed to load prompts:", e);
    } finally {
      isLoading = false;
    }
  }

  async function deletePrompt(id: string, e: MouseEvent) {
    e.stopPropagation();
    if (!confirm("Are you sure you want to delete this prompt?")) return;
    try {
      await invoke("delete_prompt", { id });
      prompts = prompts.filter(p => p.id !== id);
    } catch (e) {
      alert("Failed to delete prompt: " + e);
    }
  }

  function handleSelect(content: string) {
    input.update(val => {
        if (val) {
            return val + (val.endsWith(" ") ? "" : " ") + content;
        } else {
            return content;
        }
    });
  }

  async function handleExport() {
    if (prompts.length === 0) return;
    try {
      const filePath = await save({
        filters: [{ name: 'JSON', extensions: ['json'] }],
        defaultPath: 'warbler-prompts.json'
      });
      
      if (filePath) {
        const content = JSON.stringify(prompts, null, 2);
        await invoke("write_to_file", { path: filePath, content });
        alert("Prompts exported successfully!");
      }
    } catch (e) {
      alert("Export failed: " + e);
    }
  }

  async function handleImport() {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: 'JSON', extensions: ['json'] }]
      });
      
      if (selected && typeof selected === 'string') {
        const content = await invoke<string>("read_file", { path: selected });
        const importedPrompts = JSON.parse(content);
        
        if (Array.isArray(importedPrompts)) {
          let count = 0;
          for (const p of importedPrompts) {
            if (p.title && p.content) {
              await invoke("add_prompt", { title: p.title, content: p.content });
              count++;
            }
          }
          await refresh();
          alert(`Successfully imported ${count} prompts!`);
        } else {
          alert("Invalid file format: Expected an array of prompts.");
        }
      }
    } catch (e) {
      alert("Import failed: " + e);
    }
  }

  onMount(refresh);
</script>

<div class="knowledge-section">
  <div class="section-header">
    <span class="section-title">Saved Prompts</span>
    <div class="header-actions">
        <button class="add-kb-btn" on:click={handleImport} title="Import prompts">📥</button>
        <button class="add-kb-btn" on:click={handleExport} title="Export prompts" disabled={prompts.length === 0}>💾</button>
        <button class="add-kb-btn" on:click={() => dispatch('add')} title="Create new prompt">＋</button>
    </div>
  </div>
  
  <div class="kb-list">
    {#each prompts as prompt}
      <!-- svelte-ignore a11y-click-events-have-key-events -->
      <!-- svelte-ignore a11y-no-static-element-interactions -->
      <div class="kb-item clickable" on:click={() => handleSelect(prompt.content)} role="button" tabindex="0" on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && handleSelect(prompt.content)}>
        <span class="folder-icon">📝</span>
        <span class="kb-path" title={prompt.content}>{prompt.title}</span>
        <button class="kb-delete" on:click={(e) => deletePrompt(prompt.id, e)}>✕</button>
      </div>
    {/each}
    
    {#if prompts.length === 0 && !isLoading}
      <p class="kb-hint">No saved prompts yet.</p>
    {/if}
    
    {#if isLoading}
      <p class="kb-hint">Loading...</p>
    {/if}
  </div>
</div>

<style>
  .knowledge-section { padding: 14px; border-top: 1px solid #222; }
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
  .kb-item.clickable { cursor: pointer; transition: background 0.2s; }
  .kb-item.clickable:hover { background: #222; }
  
  .folder-icon { font-size: 0.9rem; opacity: 0.7; }
  .kb-path { flex: 1; font-size: 0.75rem; color: #bbb; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .kb-delete { background: none; border: none; color: #444; cursor: pointer; font-size: 0.75rem; }
  .kb-delete:hover { color: #ff4747; }
  .kb-hint { font-size: 0.7rem; color: #444; text-align: center; padding: 10px 0; }
</style>
