<script>
  import { marked } from "marked";
  import hljs from "../highlight.js";
  import katex from "katex";

  let { content = "" } = $props();

  marked.setOptions({
    highlight(code, lang) {
      if (lang && hljs.getLanguage(lang)) {
        return hljs.highlight(code, { language: lang }).value;
      }
      return hljs.highlightAuto(code).value;
    },
  });

  function renderLatex(text) {
    if (!text) return "";
    text = text.replace(/\$\$([\s\S]*?)\$\$/g, (_, math) => {
      try { return katex.renderToString(math.trim(), { displayMode: true, throwOnError: false }); } catch { return _; }
    });
    text = text.replace(/\\\[([\s\S]*?)\\\]/g, (_, math) => {
      try { return katex.renderToString(math.trim(), { displayMode: true, throwOnError: false }); } catch { return _; }
    });
    text = text.replace(/(?<!\$)\$(?!\$)((?:[^$\\]|\\.)+?)\$/g, (_, math) => {
      try { return katex.renderToString(math.trim(), { displayMode: false, throwOnError: false }); } catch { return _; }
    });
    text = text.replace(/\\\(([\s\S]*?)\\\)/g, (_, math) => {
      try { return katex.renderToString(math.trim(), { displayMode: false, throwOnError: false }); } catch { return _; }
    });
    return text;
  }

  let renderedHtml = $derived(marked.parse(renderLatex(content || "")));
</script>

<div class="markdown-renderer">
  {@html renderedHtml}
</div>

<style>
  .markdown-renderer {
    padding: 16px;
    overflow: auto;
    height: 100%;
    color: var(--text-primary);
    line-height: 1.6;
    font-size: 14px;
  }
  .markdown-renderer :global(pre) {
    background: var(--code-bg);
    padding: 12px;
    border-radius: 8px;
    overflow-x: auto;
    margin: 8px 0;
  }
  .markdown-renderer :global(code) {
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 13px;
  }
  .markdown-renderer :global(p code) {
    background: var(--code-inline-bg);
    padding: 2px 6px;
    border-radius: 4px;
  }
  .markdown-renderer :global(h1), .markdown-renderer :global(h2), .markdown-renderer :global(h3) {
    margin: 16px 0 8px;
  }
  .markdown-renderer :global(p) { margin-bottom: 8px; }
  .markdown-renderer :global(ul), .markdown-renderer :global(ol) { padding-left: 20px; margin: 8px 0; }
  .markdown-renderer :global(a) { color: var(--accent); text-decoration: none; }
  .markdown-renderer :global(a:hover) { text-decoration: underline; }
</style>
