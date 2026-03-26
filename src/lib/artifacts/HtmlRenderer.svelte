<script>
  let { content = "", language = "html" } = $props();

  function buildSrcdoc(code, lang) {
    if (lang === "svg" || (lang === "html" && code.trim().startsWith("<svg"))) {
      return `<!DOCTYPE html>
<html><head><style>body{margin:0;display:flex;align-items:center;justify-content:center;min-height:100vh;background:#fff;}</style></head>
<body>${code}</body></html>`;
    }
    if (lang === "html" && (code.includes("<!DOCTYPE") || code.includes("<html"))) {
      return code;
    }
    return `<!DOCTYPE html>
<html><head><style>body{margin:0;padding:16px;font-family:system-ui,sans-serif;background:#fff;color:#333;}</style></head>
<body>${code}</body></html>`;
  }

  let srcdoc = $derived(buildSrcdoc(content, language));
</script>

<iframe
  {srcdoc}
  sandbox="allow-scripts"
  title="HTML preview"
  class="html-iframe"
></iframe>

<style>
  .html-iframe {
    width: 100%;
    height: 100%;
    border: none;
    background: #ffffff;
  }
</style>
