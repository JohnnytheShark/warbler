<script lang="ts">
  import BrandHeader from "./BrandHeader.svelte";
  import ModelSettings from "./ModelSettings.svelte";
  import ChatList from "./ChatList.svelte";
  import KnowledgeSection from "./KnowledgeSection.svelte";
  import McpSection from "./McpSection.svelte";
  import PromptSection from "./PromptSection.svelte";
  import ConnectionSettings from "./ConnectionSettings.svelte";
  import { createEventDispatcher } from "svelte";

  const dispatch = createEventDispatcher();

  export let promptSection: PromptSection;
</script>

<aside class="sidebar">
  <BrandHeader />
  <ConnectionSettings />
  <ModelSettings />
  <ChatList />
  <KnowledgeSection />
  <McpSection on:add={() => dispatch("add-mcp")} />
  <PromptSection 
    bind:this={promptSection} 
    on:add={() => dispatch("add-prompt")} 
  />
</aside>

<style>
  .sidebar {
    width: 240px; min-width: 240px;
    background: #111;
    border-right: 1px solid #222;
    display: flex; flex-direction: column;
    height: 100vh;
  }
</style>
