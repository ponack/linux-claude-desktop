<script>
  import { onMount } from "svelte";

  let { content = "" } = $props();
  let containerEl;
  let svgOutput = $state("");
  let error = $state("");

  async function renderDiagram() {
    try {
      const mermaid = (await import("mermaid")).default;
      mermaid.initialize({ startOnLoad: false, theme: "dark" });
      const id = `mermaid-${Date.now()}`;
      const { svg } = await mermaid.render(id, content);
      svgOutput = svg;
      error = "";
    } catch (e) {
      error = String(e.message || e);
      svgOutput = "";
    }
  }

  onMount(() => { renderDiagram(); });

  $effect(() => {
    content;
    renderDiagram();
  });
</script>

<div class="mermaid-renderer" bind:this={containerEl}>
  {#if error}
    <div class="mermaid-error">
      <p>Diagram error:</p>
      <pre>{error}</pre>
    </div>
  {:else}
    <div class="mermaid-svg">{@html svgOutput}</div>
  {/if}
</div>

<style>
  .mermaid-renderer {
    height: 100%;
    overflow: auto;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
    background: var(--bg-primary);
  }
  .mermaid-svg {
    max-width: 100%;
  }
  .mermaid-svg :global(svg) {
    max-width: 100%;
    height: auto;
  }
  .mermaid-error {
    color: var(--danger);
    padding: 16px;
    font-size: 13px;
  }
  .mermaid-error pre {
    background: var(--code-bg);
    padding: 8px;
    border-radius: 6px;
    margin-top: 8px;
    white-space: pre-wrap;
    word-break: break-word;
  }
</style>
