<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { invoke } from "@tauri-apps/api/core";

  const dispatch = createEventDispatcher();

  export let show = false;

  let title = "";
  let content = "";
  let isSaving = false;

  async function save() {
    if (!title || !content) return;
    isSaving = true;
    try {
      await invoke("add_prompt", { title, content });
      title = "";
      content = "";
      dispatch('saved');
      show = false;
    } catch (e) {
      alert("Failed to save prompt: " + e);
    } finally {
      isSaving = false;
    }
  }

  function close() {
    show = false;
    dispatch('close');
  }
</script>

{#if show}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div class="modal-overlay" on:click={close} role="button" tabindex="0" on:keydown={(e) => e.key === 'Escape' && close()}>
    <div class="modal" on:click|stopPropagation>
      <div class="modal-header">
        <h3>Create System Prompt</h3>
        <button class="close-btn" on:click={close}>✕</button>
      </div>
      <div class="modal-body">
        <div class="field">
          <label for="prompt-title">Short Title</label>
          <input id="prompt-title" bind:value={title} placeholder="e.g. Code Reviewer" />
        </div>
        <div class="field">
          <label for="prompt-content">Prompt Instructions</label>
          <textarea id="prompt-content" bind:value={content} placeholder="Enter your prompt instructions here..." rows={6}></textarea>
        </div>
      </div>
      <div class="modal-footer">
        <button class="cancel-modal-btn" on:click={close}>Cancel</button>
        <button class="save-modal-btn" on:click={save} disabled={!title || !content || isSaving}>
          {isSaving ? 'Saving...' : 'Save Prompt'}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed; top: 0; left: 0; width: 100vw; height: 100vh;
    background: rgba(0, 0, 0, 0.7); backdrop-filter: blur(4px);
    display: flex; align-items: center; justify-content: center; z-index: 1000;
  }

  .modal {
    background: #121212; border: 1px solid #2a2a2a; border-radius: 16px;
    width: 480px; max-width: 90vw; display: flex; flex-direction: column;
    box-shadow: 0 20px 40px rgba(0,0,0,0.4); overflow: hidden;
  }

  .modal-header {
    padding: 16px 20px; border-bottom: 1px solid #1a1a1a;
    display: flex; align-items: center; justify-content: space-between;
  }
  .modal-header h3 { font-size: 1rem; color: #fff; font-weight: 600; }
  .close-btn { background: none; border: none; color: #555; cursor: pointer; font-size: 1.1rem; }

  .modal-body { padding: 20px; display: flex; flex-direction: column; gap: 16px; }

  .field { display: flex; flex-direction: column; gap: 6px; }
  .field label { font-size: 0.75rem; color: #666; text-transform: uppercase; letter-spacing: 0.05em; font-weight: 500; }

  .field input, .field textarea {
    background: #1a1a1a; border: 1px solid #2a2a2a; border-radius: 8px;
    color: #fff; padding: 10px 12px; font-family: inherit; font-size: 0.9rem;
    outline: none; transition: border-color 0.2s;
  }
  .field input:focus, .field textarea:focus { border-color: #3d5afe; }

  .modal-footer {
    padding: 16px 20px; border-top: 1px solid #1a1a1a;
    display: flex; justify-content: flex-end; gap: 10px;
  }

  .cancel-modal-btn {
    background: none; border: 1px solid #2a2a2a; color: #888;
    padding: 8px 16px; border-radius: 8px; cursor: pointer; font-size: 0.85rem;
    transition: background 0.2s;
  }
  .cancel-modal-btn:hover { background: #1a1a1a; color: #fff; }

  .save-modal-btn {
    background: #3d5afe; border: none; color: #fff;
    padding: 8px 20px; border-radius: 8px; cursor: pointer; font-size: 0.85rem;
    font-weight: 500; transition: transform 0.2s, background 0.2s;
  }
  .save-modal-btn:hover { background: #536dfe; transform: translateY(-1px); }
  .save-modal-btn:disabled { background: #222; color: #555; cursor: not-allowed; transform: none; }
</style>
