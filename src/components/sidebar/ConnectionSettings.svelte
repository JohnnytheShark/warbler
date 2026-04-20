<script lang="ts">
  import { aiProvider, aiBaseUrl, ollamaConnected } from "../../lib/stores";
  import { saveAiProvider, saveAiBaseUrl, fetchModels } from "../../lib/services";

  let showDetails = false;

  async function handleProviderChange(e: Event) {
    const val = (e.target as HTMLSelectElement).value;
    await saveAiProvider(val);
    
    // Set default base URL for the provider if it's currently the default
    if (val === 'ollama' && $aiBaseUrl.includes(':8080')) {
      await saveAiBaseUrl('http://localhost:11434');
    } else if (val === 'openai_compatible' && $aiBaseUrl.includes(':11434')) {
      await saveAiBaseUrl('http://localhost:8080/v1');
    }
    
    await fetchModels();
  }

  async function handleUrlBlur() {
    await fetchModels();
  }
</script>

<div class="connection-section">
  <div class="section-header" on:click={() => showDetails = !showDetails} role="button" tabindex="0" on:keydown={(e) => e.key === 'Enter' && (showDetails = !showDetails)}>
    <span class="section-title">Connection</span>
    <span class="expand-icon">{showDetails ? '−' : '＋'}</span>
  </div>

  {#if showDetails}
    <div class="details-panel">
      <div class="field">
        <label for="provider-select">Provider</label>
        <select id="provider-select" value={$aiProvider} on:change={handleProviderChange}>
          <option value="ollama">Ollama</option>
          <option value="openai_compatible">Custom (OpenAI/llama.cpp)</option>
        </select>
      </div>

      <div class="field">
        <label for="base-url">Base URL</label>
        <input 
          id="base-url"
          type="text" 
          bind:value={$aiBaseUrl} 
          on:blur={handleUrlBlur}
          on:change={(e) => saveAiBaseUrl((e.target as HTMLInputElement).value)}
          placeholder="http://localhost:11434"
        />
      </div>

      <div class="field">
        <label for="api-key">API Key (Optional)</label>
        <input 
          id="api-key"
          type="password" 
          on:change={(e) => {
              const val = (e.target as HTMLInputElement).value;
              import("@tauri-apps/api/core").then(m => m.invoke("set_config", { key: "ai_api_key", value: val }));
          }}
          placeholder="••••••••"
        />
      </div>

      <div class="status-summary">
        <div class="status-dot {$ollamaConnected ? 'ok' : 'err'}"></div>
        <span>{$ollamaConnected ? 'Backend Reachable' : 'Backend Unreachable'}</span>
      </div>
    </div>
  {/if}
</div>

<style>
  .connection-section { padding: 14px; border-top: 1px solid #222; }
  .section-header { 
    display: flex; align-items: center; justify-content: space-between; 
    cursor: pointer; user-select: none;
  }
  .section-title { font-size: 0.7rem; font-weight: 600; color: #555; text-transform: uppercase; letter-spacing: 0.05em; }
  .expand-icon { font-size: 0.8rem; color: #444; }

  .details-panel { margin-top: 12px; display: flex; flex-direction: column; gap: 12px; }
  
  .field { display: flex; flex-direction: column; gap: 6px; }
  .field label { font-size: 0.7rem; color: #666; font-weight: 500; }
  
  .field select, .field input {
    background: #1a1a1a; border: 1px solid #222; color: #ccc;
    padding: 6px 10px; border-radius: 6px; font-size: 0.8rem; outline: none;
  }
  .field select:focus, .field input:focus { border-color: #444; }

  .status-summary { display: flex; align-items: center; gap: 8px; margin-top: 4px; }
  .status-dot { width: 6px; height: 6px; border-radius: 50%; }
  .status-dot.ok { background: #00ff7f; box-shadow: 0 0 6px rgba(0, 255, 127, 0.3); }
  .status-dot.err { background: #ff4747; box-shadow: 0 0 6px rgba(255, 71, 71, 0.3); }
  .status-summary span { font-size: 0.7rem; color: #666; }
</style>
