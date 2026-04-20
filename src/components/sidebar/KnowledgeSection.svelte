<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { 
    groundingFolders, 
    isIndexing, 
    indexingStatus, 
    indexingProgress, 
    isEmbeddingModelMissing, 
    selectedEmbeddingModel,
    isPulling,
    pullStatus,
    pullProgress,
    aiProvider
  } from "../../lib/stores";
  import { refreshGroundingFolders, addKnowledgeFolder } from "../../lib/services";

  async function removeKnowledgeFolder(id: string) {
    await invoke("delete_grounding_folder", { id });
    await refreshGroundingFolders();
  }

  async function clearAllKnowledge() {
    if (confirm("Clear ALL knowledge and indexed folders?")) {
        await invoke("clear_all_knowledge");
        groundingFolders.set([]);
    }
  }

  async function startModelPull(model: string) {
    isPulling.set(true);
    pullProgress.set(0);
    pullStatus.set("Starting pull...");
    try {
      await invoke("pull_ollama_model", { model });
    } catch (e) {
      alert("Pull failed: " + e);
      isPulling.set(false);
    }
  }
</script>

<div class="knowledge-section">
  <div class="section-header">
    <span class="section-title">Knowledge</span>
    <div class="header-actions">
        <button class="clear-kb-btn" on:click={clearAllKnowledge} title="Clear all knowledge">🗑</button>
        <button class="add-kb-btn" on:click={addKnowledgeFolder} disabled={$isIndexing || $isEmbeddingModelMissing}>＋</button>
    </div>
  </div>

  {#if $aiProvider === 'ollama'}
    {#if $isEmbeddingModelMissing}
      <div class="kb-warning">
        <p>⚠️ <strong>{$selectedEmbeddingModel}</strong> is missing. This is required for RAG.</p>
        <button class="pull-btn" on:click={() => startModelPull($selectedEmbeddingModel)} disabled={$isPulling}>
          {$isPulling ? 'Pulling...' : 'Pull Model'}
        </button>
      </div>
    {/if}

    {#if $isPulling}
      <div class="indexing-panel">
          <div class="indexing-status">{$pullStatus}</div>
          <div class="progress-bar-bg">
              <div class="progress-bar-fill" style="width: {$pullProgress}%"></div>
          </div>
      </div>
    {/if}
  {/if}

  <div class="kb-list">
    {#each $groundingFolders as folder}
        <div class="kb-item">
            <span class="folder-icon">📂</span>
            <span class="kb-path" title={folder.path}>{folder.path.split(/[\\/]/).pop()}</span>
            <button class="kb-delete" on:click={() => removeKnowledgeFolder(folder.id)}>✕</button>
        </div>
    {/each}
    {#if $groundingFolders.length === 0}
        <p class="kb-hint">No folders added.</p>
    {/if}
    {#if $isIndexing}
        <div class="indexing-panel">
            <div class="indexing-status">{$indexingStatus}</div>
            <div class="progress-bar-bg">
                <div class="progress-bar-fill" style="width: {$indexingProgress}%"></div>
            </div>
        </div>
    {/if}
  </div>
</div>

<style>
  .knowledge-section { padding: 14px; border-top: 1px solid #222; }
  .section-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 12px; }
  .section-title { font-size: 0.7rem; font-weight: 600; color: #555; text-transform: uppercase; letter-spacing: 0.05em; }
  
  .header-actions { display: flex; gap: 8px; }
  .clear-kb-btn {
    background: none; border: none; color: #555; cursor: pointer; font-size: 1rem;
    transition: color 0.15s;
  }
  .clear-kb-btn:hover { color: #ff6b6b; }

  .add-kb-btn {
    background: #252525; border: 1px solid #333; color: #eee; width: 22px; height: 22px;
    border-radius: 4px; display: flex; align-items: center; justify-content: center;
    cursor: pointer; font-size: 0.9rem; transition: background 0.2s;
  }
  .add-kb-btn:hover:not(:disabled) { background: #333; }
  .add-kb-btn:disabled { opacity: 0.3; cursor: not-allowed; }

  .kb-list { display: flex; flex-direction: column; gap: 4px; }
  .kb-item {
    display: flex; align-items: center; background: #181818; border: 1px solid #222;
    padding: 6px 10px; border-radius: 6px; gap: 8px;
  }
  .folder-icon { font-size: 0.9rem; opacity: 0.7; }
  .kb-path { flex: 1; font-size: 0.75rem; color: #bbb; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .kb-delete { background: none; border: none; color: #444; cursor: pointer; font-size: 0.75rem; }
  .kb-delete:hover { color: #ff4747; }
  .kb-hint { font-size: 0.7rem; color: #444; text-align: center; padding: 10px 0; }

  .indexing-panel { margin-top: 10px; padding: 10px; background: #1a1a1a; border-radius: 8px; border: 1px solid #222; }
  .indexing-status { font-size: 0.65rem; color: #888; margin-bottom: 6px; }
  .progress-bar-bg { background: #111; height: 4px; border-radius: 2px; overflow: hidden; }
  .progress-bar-fill { background: #00ff7f; height: 100%; transition: width 0.3s; }

  .kb-warning {
    background: #2a1a1a; border: 1px solid #4a2a2a; border-radius: 8px; padding: 10px;
    margin-bottom: 12px; font-size: 0.75rem; color: #ffbaba; display: flex; flex-direction: column; gap: 8px;
  }
  .pull-btn {
    background: #ff4747; border: none; color: white; padding: 6px; border-radius: 6px; 
    cursor: pointer; font-weight: 600; font-size: 0.7rem;
  }
  .pull-btn:disabled { background: #444; cursor: not-allowed; }
</style>
