<script lang="ts">
  import { activeChat } from "../../lib/stores";
  import ChatHeader from "./ChatHeader.svelte";
  import ToolsPanel from "./ToolsPanel.svelte";
  import MessageList from "./MessageList.svelte";
  import ChatInput from "./ChatInput.svelte";
  import icon from "../../assets/icon.png";
</script>

<main class="main">
  {#if !$activeChat}
    <div class="empty-state">
      <div class="empty-icon">
        <img src={icon} alt="Warbler Logo" />
      </div>
      <h2>Start a new chat</h2>
      <p>Click <strong>＋</strong> in the sidebar to begin.</p>
    </div>
  {:else}
    <div class="chat-container">
      <ChatHeader />
      <ToolsPanel />
      <MessageList />
      <ChatInput />
    </div>
  {/if}
</main>

<style>
  .main {
    flex: 1;
    display: flex;
    flex-direction: column;
    background: #0d0d0d;
    position: relative;
    overflow: hidden;
  }
  .chat-container {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: #333;
    text-align: center;
    padding-bottom: 10vh;
  }
  .empty-icon {
    font-size: 5rem;
    margin-bottom: 20px;
    opacity: 0.15;
    filter: grayscale(1);
  }
  .empty-state h2 {
    font-size: 1.5rem;
    color: #444;
    margin-bottom: 8px;
    font-weight: 600;
  }
  .empty-state p {
    font-size: 0.95rem;
    color: #555;
  }
</style>
