<script lang="ts">
  import { activeChat, selectedModel } from "../../lib/stores";
</script>

{#if $activeChat}
  <header class="chat-header">
    <span class="chat-title">{$activeChat.title}</span>
    <span class="header-model">🤖 {$selectedModel}</span>
  </header>
{/if}

<style>
  .chat-header {
    height: 60px; min-height: 60px;
    display: flex; align-items: center; justify-content: space-between;
    padding: 0 24px; border-bottom: 1px solid #222;
    background: rgba(13, 13, 13, 0.8); backdrop-filter: blur(10px);
    z-index: 10;
  }
  .chat-title { font-weight: 600; font-size: 0.95rem; color: #fff; }
  .header-model { font-size: 0.75rem; color: #555; font-weight: 500; background: #1a1a1a; padding: 4px 10px; border-radius: 100px; border: 1px solid #222; }
</style>
