<script lang="ts">
  import { availableModels, selectedModel, selectedEmbeddingModel } from "../../lib/stores";
  import { fetchModels, saveModelPref, saveEmbeddingModel } from "../../lib/services";
</script>

<div class="model-section">
  <label class="model-label" for="model-select">Model</label>
  <div class="model-select-wrap">
    <select
      id="model-select"
      class="model-select"
      value={$selectedModel}
      on:change={e => saveModelPref((e.target as HTMLSelectElement).value)}
    >
      {#if $availableModels.length > 0}
        {#each $availableModels as m}
          <option value={m}>{m}</option>
        {/each}
      {:else}
        <option value={$selectedModel}>{$selectedModel}</option>
      {/if}
    </select>
  </div>

  <label class="model-label" for="embed-select" style="margin-top: 10px;">Embedding Model</label>
  <div class="model-select-wrap">
    <select
      id="embed-select"
      class="model-select"
      value={$selectedEmbeddingModel}
      on:change={e => saveEmbeddingModel((e.target as HTMLSelectElement).value)}
    >
      {#if $availableModels.length > 0}
        {#each $availableModels as m}
          <option value={m}>{m}</option>
        {/each}
      {:else}
        <option value={$selectedEmbeddingModel}>{$selectedEmbeddingModel}</option>
      {/if}
    </select>
    <button
      class="refresh-btn"
      on:click={fetchModels}
      title="Refresh models"
      aria-label="Refresh models"
    >↻</button>
  </div>
</div>

<style>
  .model-section { padding: 14px; border-bottom: 1px solid #222; }
  .model-label { display: block; font-size: 0.7rem; font-weight: 600; color: #555; text-transform: uppercase; margin-bottom: 6px; }
  .model-select-wrap { position: relative; display: flex; align-items: center; gap: 4px; }
  .model-select {
    flex: 1; background: #1a1a1a; border: 1px solid #222; color: #ccc;
    padding: 6px 10px; border-radius: 6px; font-size: 0.8rem; outline: none; transition: border-color 0.2s;
  }
  .model-select:focus { border-color: #444; }
  .refresh-btn {
    background: none; border: none; color: #444; cursor: pointer; font-size: 1rem;
    padding: 4px; transition: color 0.15s;
  }
  .refresh-btn:hover { color: #888; }
</style>
