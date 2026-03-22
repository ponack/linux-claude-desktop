<script>
  let { artifact, onClose } = $props();

  let iframeEl;
  let activeTab = $state("preview");

  function buildSrcdoc(code, language) {
    if (language === "svg" || (language === "html" && code.trim().startsWith("<svg"))) {
      return `<!DOCTYPE html>
<html><head><style>body{margin:0;display:flex;align-items:center;justify-content:center;min-height:100vh;background:#fff;}</style></head>
<body>${code}</body></html>`;
    }
    if (language === "html" && (code.includes("<!DOCTYPE") || code.includes("<html"))) {
      return code;
    }
    // Wrap partial HTML
    return `<!DOCTYPE html>
<html><head><style>body{margin:0;padding:16px;font-family:system-ui,sans-serif;background:#fff;color:#333;}</style></head>
<body>${code}</body></html>`;
  }

  let srcdoc = $derived(buildSrcdoc(artifact.code, artifact.language));
</script>

<div class="artifact-panel">
  <div class="artifact-header">
    <div class="artifact-tabs">
      <button
        class="tab-btn"
        class:active={activeTab === "preview"}
        onclick={() => (activeTab = "preview")}
      >Preview</button>
      <button
        class="tab-btn"
        class:active={activeTab === "code"}
        onclick={() => (activeTab = "code")}
      >Code</button>
    </div>
    <button class="close-artifact" onclick={onClose} title="Close preview">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <line x1="18" y1="6" x2="6" y2="18"/>
        <line x1="6" y1="6" x2="18" y2="18"/>
      </svg>
    </button>
  </div>

  <div class="artifact-body">
    {#if activeTab === "preview"}
      <iframe
        bind:this={iframeEl}
        {srcdoc}
        sandbox="allow-scripts"
        title="Artifact preview"
        class="artifact-iframe"
      ></iframe>
    {:else}
      <pre class="artifact-code"><code>{artifact.code}</code></pre>
    {/if}
  </div>
</div>

<style>
  .artifact-panel {
    width: 45%;
    min-width: 320px;
    max-width: 600px;
    display: flex;
    flex-direction: column;
    border-left: 1px solid var(--border);
    background: var(--bg-secondary);
    height: 100%;
  }

  .artifact-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    gap: 8px;
  }

  .artifact-tabs {
    display: flex;
    gap: 4px;
  }

  .tab-btn {
    padding: 4px 12px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    transition: background 0.15s, color 0.15s;
  }

  .tab-btn:hover {
    background: var(--bg-tertiary);
  }

  .tab-btn.active {
    background: var(--accent);
    color: white;
  }

  .close-artifact {
    color: var(--text-muted);
    padding: 4px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    transition: color 0.15s;
  }

  .close-artifact:hover {
    color: var(--text-primary);
  }

  .artifact-body {
    flex: 1;
    overflow: hidden;
  }

  .artifact-iframe {
    width: 100%;
    height: 100%;
    border: none;
    background: #ffffff;
  }

  .artifact-code {
    margin: 0;
    padding: 12px;
    overflow: auto;
    height: 100%;
    background: var(--code-bg);
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 13px;
    line-height: 1.5;
    color: var(--text-primary);
  }
</style>
