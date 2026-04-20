<script lang="ts">
  import { chats, activeChatId } from "../../lib/stores";
  import { deleteChat } from "../../lib/services";

  function selectChat(id: string) {
    activeChatId.set(id);
  }

  function handleDelete(id: string, e: MouseEvent) {
    e.stopPropagation();
    deleteChat(id);
  }
</script>

<div class="chat-list">
  {#each $chats as chat (chat.id)}
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <div
      class="chat-item {chat.id === $activeChatId ? 'active' : ''}"
      on:click={() => selectChat(chat.id)}
      on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && selectChat(chat.id)}
      role="button"
      tabindex="0"
    >
      <span class="chat-item-title">{chat.title}</span>
      <button class="delete-btn" on:click={e => handleDelete(chat.id, e)} title="Delete">✕</button>
    </div>
  {/each}
  {#if $chats.length === 0}
    <p class="empty-hint">No chats yet.<br/>Click ＋ to start.</p>
  {/if}
</div>

<style>
  .chat-list { flex: 1; overflow-y: auto; padding: 10px 0; }
  .chat-item {
    padding: 10px 14px; margin: 2px 8px; border-radius: 8px;
    cursor: pointer; display: flex; align-items: center; justify-content: space-between;
    font-size: 0.85rem; color: #999; transition: all 0.2s; position: relative;
    outline: none;
  }
  .chat-item:hover { background: #1a1a1a; color: #eee; }
  .chat-item.active { background: #222; color: #fff; font-weight: 500; }
  .chat-item-title { flex: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; margin-right: 8px; }
  .delete-btn {
    background: none; border: none; color: #444; cursor: pointer; font-size: 0.8rem;
    padding: 4px; opacity: 0; transition: opacity 0.2s, color 0.2s;
  }
  .chat-item:hover .delete-btn { opacity: 1; }
  .delete-btn:hover { color: #ff4747; }
  .empty-hint { padding: 40px 20px; text-align: center; font-size: 0.8rem; color: #444; line-height: 1.5; }
</style>
