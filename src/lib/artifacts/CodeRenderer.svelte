<script>
  import { onMount, onDestroy } from "svelte";
  import { EditorView, basicSetup } from "codemirror";
  import { EditorState } from "@codemirror/state";
  import { oneDark } from "@codemirror/theme-one-dark";

  let { content = "", language = "", editable = false, onChange } = $props();
  let containerEl;
  let view;

  async function getLanguageExtension(lang) {
    try {
      switch (lang) {
        case "javascript": case "js": case "jsx": case "react":
          return (await import("@codemirror/lang-javascript")).javascript({ jsx: true });
        case "typescript": case "ts": case "tsx":
          return (await import("@codemirror/lang-javascript")).javascript({ jsx: true, typescript: true });
        case "html": case "svg":
          return (await import("@codemirror/lang-html")).html();
        case "css":
          return (await import("@codemirror/lang-css")).css();
        case "markdown": case "md":
          return (await import("@codemirror/lang-markdown")).markdown();
        case "python": case "py":
          return (await import("@codemirror/lang-python")).python();
        case "rust": case "rs":
          return (await import("@codemirror/lang-rust")).rust();
        case "json":
          return (await import("@codemirror/lang-json")).json();
        default:
          return null;
      }
    } catch {
      return null;
    }
  }

  onMount(async () => {
    const extensions = [basicSetup, oneDark, EditorView.lineWrapping];

    const langExt = await getLanguageExtension(language);
    if (langExt) extensions.push(langExt);

    if (!editable) {
      extensions.push(EditorState.readOnly.of(true));
    }

    if (editable && onChange) {
      let debounceTimer;
      extensions.push(EditorView.updateListener.of((update) => {
        if (update.docChanged) {
          clearTimeout(debounceTimer);
          debounceTimer = setTimeout(() => {
            onChange(update.state.doc.toString());
          }, 500);
        }
      }));
    }

    view = new EditorView({
      state: EditorState.create({ doc: content, extensions }),
      parent: containerEl,
    });
  });

  onDestroy(() => {
    if (view) view.destroy();
  });

  // Update content when it changes externally
  $effect(() => {
    if (view && content !== view.state.doc.toString()) {
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: content },
      });
    }
  });
</script>

<div class="code-renderer" bind:this={containerEl}></div>

<style>
  .code-renderer {
    height: 100%;
    overflow: auto;
  }
  .code-renderer :global(.cm-editor) {
    height: 100%;
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 13px;
  }
  .code-renderer :global(.cm-scroller) {
    overflow: auto;
  }
</style>
