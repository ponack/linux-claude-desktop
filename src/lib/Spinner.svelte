<script>
  let { size = "md", label = "" } = $props();
  // size: sm (14px) | md (20px) | lg (32px)
</script>

<div class="spinner-wrap">
  <div class="spinner spinner-{size}" role="status" aria-label={label || "Loading"}></div>
  {#if label}
    <span class="spinner-label">{label}</span>
  {/if}
</div>

<style>
  .spinner-wrap {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }

  .spinner {
    border-radius: 50%;
    border: 2px solid currentColor;
    border-top-color: transparent;
    opacity: 0.7;
    animation: spin 0.8s linear infinite;
    flex-shrink: 0;
  }

  .spinner-sm { width: 14px; height: 14px; border-width: 2px; }
  .spinner-md { width: 20px; height: 20px; border-width: 2px; }
  .spinner-lg { width: 32px; height: 32px; border-width: 3px; }

  .spinner-label {
    font-size: 13px;
    color: var(--text-muted);
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
