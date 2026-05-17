<script>
  import Icon from "./Icon.svelte";

  let {
    icon = "",          // optional icon name from icons.js
    title,              // required heading
    description = "",   // optional secondary line
    children,           // optional action snippet (buttons, links)
  } = $props();
</script>

<div class="empty-state" role="status">
  {#if icon}
    <div class="empty-icon">
      <Icon name={icon} size={32} stroke={1.5} />
    </div>
  {/if}
  <h3 class="empty-title">{title}</h3>
  {#if description}
    <p class="empty-description">{description}</p>
  {/if}
  {#if children}
    <div class="empty-actions">{@render children()}</div>
  {/if}
</div>

<style>
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: var(--space-6) var(--space-4);
    color: var(--text-muted);
    gap: var(--space-2);
  }

  .empty-icon {
    color: var(--text-muted);
    opacity: 0.5;
    margin-bottom: var(--space-1);
  }

  .empty-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-secondary);
    margin: 0;
  }

  .empty-description {
    font-size: 12px;
    color: var(--text-muted);
    margin: 0;
    max-width: 280px;
    line-height: 1.5;
  }

  .empty-actions {
    margin-top: var(--space-3);
  }
</style>
