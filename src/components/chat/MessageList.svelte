<script lang="ts">
  import { messages, isSending } from "../../lib/stores";
  import MessageItem from "./MessageItem.svelte";
  import { onMount, afterUpdate } from "svelte";

  let chatAreaEl: HTMLDivElement;

  function scrollToBottom() {
    if (chatAreaEl) {
      chatAreaEl.scrollTop = chatAreaEl.scrollHeight;
    }
  }

  // Auto-scroll when messages change or sending state changes
  $: if ($messages || $isSending) {
    setTimeout(scrollToBottom, 30);
  }

  onMount(scrollToBottom);
</script>

<div class="chat-area" bind:this={chatAreaEl}>
  {#each $messages as msg}
    <MessageItem {msg} />
  {/each}
  {#if $isSending}
    <div class="bubble-row assistant">
      <div class="avatar">🦙</div>
      <div class="bubble typing"><span></span><span></span><span></span></div>
    </div>
  {/if}
</div>

<style>
  .chat-area {
    flex: 1; overflow-y: auto; padding: 24px;
    display: flex; flex-direction: column;
    scroll-behavior: smooth;
  }

  /* Shared styles for typing indicator and avatar from page.svelte */
  .bubble-row { display: flex; gap: 12px; margin-bottom: 24px; width: 100%; }
  .avatar {
    width: 32px; height: 32px; border-radius: 50%; background: #222;
    display: flex; align-items: center; justify-content: center; font-size: 1rem;
    flex-shrink: 0; margin-top: 4px;
  }
  .bubble {
    padding: 12px 16px; border-radius: 18px; line-height: 1.5; font-size: 0.95rem;
    position: relative; word-wrap: break-word;
  }
  .bubble-row.assistant .bubble { background: #1e1e1e; color: #e5e5e5; border-bottom-left-radius: 4px; border: 1px solid #2a2a2a; }

  .bubble.typing { display: flex; gap: 4px; padding: 14px 18px; }
  .bubble.typing span { width: 6px; height: 6px; background: #555; border-radius: 50%; animation: typing 1.4s infinite; }
  .bubble.typing span:nth-child(2) { animation-delay: 0.2s; }
  .bubble.typing span:nth-child(3) { animation-delay: 0.4s; }

  @keyframes typing {
    0%, 100% { transform: translateY(0); opacity: 0.4; }
    50% { transform: translateY(-4px); opacity: 1; }
  }
</style>
