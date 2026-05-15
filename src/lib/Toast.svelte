<script>
  import { toasts, dismissToast } from "./toast.js";
</script>

<div class="toast-stack" role="region" aria-label="Notifications">
  {#each $toasts as t (t.id)}
    <div class="toast toast-{t.level}" role="status" aria-live="polite">
      <span class="toast-msg">{t.message}</span>
      <button class="toast-close" onclick={() => dismissToast(t.id)} aria-label="Dismiss">×</button>
    </div>
  {/each}
</div>

<style>
  .toast-stack {
    position: fixed;
    bottom: 20px;
    right: 20px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: 9999;
    pointer-events: none;
  }

  .toast {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    border-radius: 8px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.25);
    color: var(--text-primary);
    font-size: 13px;
    min-width: 220px;
    max-width: 400px;
    pointer-events: auto;
    animation: toast-in 0.18s ease-out;
  }

  .toast-info    { border-left: 3px solid var(--accent, #7aa2f7); }
  .toast-success { border-left: 3px solid var(--success, #4ecca3); }
  .toast-warning { border-left: 3px solid var(--warning, #e0af68); }
  .toast-error   { border-left: 3px solid var(--danger,  #e94560); }

  .toast-msg {
    flex: 1;
    word-break: break-word;
  }

  .toast-close {
    font-size: 18px;
    color: var(--text-muted);
    padding: 0 4px;
    line-height: 1;
    background: transparent;
    border: 0;
    cursor: pointer;
  }
  .toast-close:hover { color: var(--text-primary); }

  @keyframes toast-in {
    from { transform: translateY(8px); opacity: 0; }
    to   { transform: translateY(0); opacity: 1; }
  }
</style>
