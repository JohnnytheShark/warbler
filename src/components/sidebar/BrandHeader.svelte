<script lang="ts">
  import { ollamaConnected } from "../../lib/stores";
  import { createNewChat } from "../../lib/services";
</script>

<div class="sidebar-header">
  <div class="app-brand">
    <span class="app-name">🐦 Warbler</span>
    <div class="status-indicator {$ollamaConnected ? 'connected' : 'disconnected'}" 
         title={$ollamaConnected ? 'Connected to Ollama' : 'Disconnected from Ollama'}></div>
  </div>
  <button class="new-chat-btn" on:click={createNewChat} title="New chat">＋</button>
</div>

<style>
  .sidebar-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 16px 14px;
    border-bottom: 1px solid #222;
  }
  .app-brand { display: flex; align-items: center; gap: 8px; }
  .app-name { font-weight: 600; font-size: 1rem; letter-spacing: 0.02em; color: #fff; }
  .status-indicator {
    width: 8px; height: 8px; border-radius: 50%;
    background: #444; position: relative;
    transition: background 0.3s, box-shadow 0.3s;
  }
  .status-indicator.connected {
    background: #00ff7f;
    box-shadow: 0 0 8px rgba(0, 255, 127, 0.4);
  }
  .status-indicator.disconnected {
    background: #ff4747;
    box-shadow: 0 0 8px rgba(255, 71, 71, 0.4);
  }
  .new-chat-btn {
    background: transparent; border: none; color: #aaa; cursor: pointer;
    font-size: 1.2rem; display: flex; align-items: center; justify-content: center;
    width: 28px; height: 28px; border-radius: 6px; transition: all 0.2s;
  }
  .new-chat-btn:hover { background: #222; color: #fff; }
</style>
