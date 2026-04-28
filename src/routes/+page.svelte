<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";

  // Stores & Services
  import * as s from "../lib/stores";
  import * as sv from "../lib/services";

  // Components
  import Sidebar from "../components/sidebar/Sidebar.svelte";
  import ChatView from "../components/chat/ChatView.svelte";
  import McpModal from "../components/modals/McpModal.svelte";
  import PromptModal from "../components/modals/PromptModal.svelte";

  // State
  let showMcpModal = false;
  let showPromptModal = false;
  let promptSection: any; // Binding for refresh

  // Getters for convenience in template
  const systemError = s.systemError;

  onMount(async () => {
    try {
      // 1. Initial Load
      const savedModel = await invoke<string | null>("load_model_pref").catch(
        () => null,
      );
      if (savedModel) s.selectedModel.set(savedModel);

      const chats = await invoke<s.Chat[]>("get_chats").catch(() => []);
      s.chats.set(chats);
      if (chats.length > 0) s.activeChatId.set(chats[0].id);

      await sv.initApp();

      // 2. Global Event Listeners
      await listen("indexing-progress", (event: any) => {
        s.indexingStatus.set(event.payload.status);
        s.indexingProgress.set(event.payload.progress);
        if (event.payload.status === "Done") {
          setTimeout(() => {
            s.isIndexing.set(false);
            s.indexingStatus.set("");
          }, 2000);
        }
      });

      await listen("pull-progress", (event: any) => {
        const payload = event.payload;
        s.pullStatus.set(payload.status || "Pulling...");
        if (payload.total && payload.completed) {
          s.pullProgress.set((payload.completed / payload.total) * 100);
        }
        if (payload.status === "success") {
          s.isPulling.set(false);
          s.pullStatus.set("");
          sv.fetchModels();
          sv.checkEmbeddingRequirement();
        }
      });
    } catch (e) {
      s.systemError.set("Application failed to initialize: " + e);
    }
  });
</script>

<div class="app">
  {#if $systemError}
    <div class="system-error-banner">
      ⚠️ {$systemError}
    </div>
  {/if}

  <Sidebar
    bind:promptSection
    on:add-mcp={() => (showMcpModal = true)}
    on:add-prompt={() => (showPromptModal = true)}
  />

  <ChatView />

  <McpModal bind:show={showMcpModal} />

  <PromptModal
    bind:show={showPromptModal}
    on:saved={() => promptSection.refresh()}
  />
</div>

<style>
  @import url("https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&display=swap");

  :global(*) {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
  }
  :global(body) {
    font-family: "Inter", sans-serif;
    background: #0d0d0d;
    color: #e8e8e8;
    height: 100vh;
    overflow: hidden;
  }

  .app {
    display: flex;
    height: 100vh;
    width: 100vw;
  }

  .system-error-banner {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    background: #ff4747;
    color: white;
    padding: 8px;
    text-align: center;
    font-size: 0.85rem;
    font-weight: 600;
    z-index: 9999;
  }
</style>
