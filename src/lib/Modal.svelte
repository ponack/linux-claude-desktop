<script>
  import Icon from "./Icon.svelte";

  let {
    open = false,
    onClose,
    title = "",
    size = "md",                  // sm | md | lg
    placement = "center",         // center | top
    closeOnBackdrop = true,
    closeOnEsc = true,
    showClose = true,
    labelledby = "",              // optional id of label element inside body
    children,
  } = $props();

  let dialogEl;
  let previousFocus = null;
  let wasOpen = false;

  function handleKey(e) {
    if (e.key === "Escape" && closeOnEsc) {
      e.preventDefault();
      onClose?.();
    }
  }

  function handleBackdropClick(e) {
    if (e.target === e.currentTarget && closeOnBackdrop) {
      onClose?.();
    }
  }

  $effect(() => {
    if (open && !wasOpen) {
      previousFocus = document.activeElement;
      requestAnimationFrame(() => {
        const focusable = dialogEl?.querySelector(
          'input, textarea, button, select, [tabindex]:not([tabindex="-1"])'
        );
        focusable?.focus();
      });
    } else if (!open && wasOpen && previousFocus) {
      previousFocus.focus?.();
      previousFocus = null;
    }
    wasOpen = open;
  });
</script>

<svelte:window onkeydown={open ? handleKey : null} />

{#if open}
  <div
    class="modal-backdrop"
    class:top={placement === "top"}
    onclick={handleBackdropClick}
    role="presentation"
  >
    <div
      bind:this={dialogEl}
      class="modal-dialog modal-{size}"
      role="dialog"
      aria-modal="true"
      aria-label={labelledby ? undefined : title}
      aria-labelledby={labelledby || undefined}
    >
      {#if title || showClose}
        <div class="modal-header">
          <h3 class="modal-title">{title}</h3>
          {#if showClose}
            <button class="modal-close" onclick={() => onClose?.()} aria-label="Close dialog">
              <Icon name="close" size={18} />
            </button>
          {/if}
        </div>
      {/if}
      <div class="modal-body">
        {@render children?.()}
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: var(--overlay-bg);
    display: flex;
    justify-content: center;
    align-items: center;
    padding: var(--space-4);
    z-index: var(--z-modal);
  }

  .modal-backdrop.top {
    align-items: flex-start;
    padding-top: 15vh;
  }

  .modal-dialog {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-4);
    box-shadow: var(--shadow-3);
    max-height: calc(100vh - var(--space-6) * 2);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    width: 100%;
  }

  .modal-sm { max-width: 400px; }
  .modal-md { max-width: 560px; }
  .modal-lg { max-width: 800px; }

  .modal-header {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-4);
    padding-bottom: var(--space-3);
    flex-shrink: 0;
  }

  .modal-title {
    flex: 1;
    font-size: 15px;
    font-weight: 600;
    margin: 0;
  }

  .modal-close {
    flex-shrink: 0;
    padding: var(--space-1);
    color: var(--text-muted);
    border-radius: var(--radius-1);
    display: flex;
    transition: color 0.15s, background 0.15s;
  }

  .modal-close:hover {
    color: var(--text-primary);
    background: var(--bg-tertiary);
  }

  .modal-body {
    padding: 0 var(--space-4) var(--space-4);
    overflow-y: auto;
    flex: 1;
  }
</style>
