<script>
  // Hosts a plugin-registered artifact renderer inside a managed DOM container.
  //
  // The plugin's render() receives:
  //   container — an empty <div> we own; the plugin can do anything inside it
  //   content   — the artifact's text content
  //   ctx       — { id, language, title, conversationId, artifactType }
  //
  // It may return a cleanup function which we'll call on destroy or when the
  // content changes (so plugins can dispose listeners, intervals, etc.).

  import { onDestroy, tick } from "svelte";

  let { render, content, ctx } = $props();

  let container;
  let cleanup = null;

  async function mount() {
    if (!container) return;
    // Tear down any previous instance before re-rendering with new content.
    if (cleanup) {
      try { cleanup(); } catch (e) { console.error("plugin renderer cleanup threw:", e); }
      cleanup = null;
    }
    container.innerHTML = "";
    try {
      const result = await render(container, content, ctx);
      if (typeof result === "function") cleanup = result;
    } catch (e) {
      console.error("plugin renderer threw:", e);
      container.innerHTML = `<pre style="color: var(--danger); padding: 16px; font-size: 12px; white-space: pre-wrap;">Plugin renderer error: ${String(e?.message || e)}</pre>`;
    }
  }

  // Re-mount whenever render, content, or ctx changes.
  $effect(() => {
    render; content; ctx;
    tick().then(mount);
  });

  onDestroy(() => {
    if (cleanup) {
      try { cleanup(); } catch (e) { console.error("plugin renderer cleanup threw:", e); }
    }
  });
</script>

<div class="plugin-renderer" bind:this={container}></div>

<style>
  .plugin-renderer {
    width: 100%;
    height: 100%;
    overflow: auto;
  }
</style>
