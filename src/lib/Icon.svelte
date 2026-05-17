<script>
  import { ICONS } from "./icons.js";

  let { name, size = 16, stroke = 2, class: className = "" } = $props();
  const path = $derived(ICONS[name]);
</script>

{#if path}
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width={stroke}
    stroke-linecap="round"
    stroke-linejoin="round"
    class={className}
    aria-hidden="true"
  >
    {@html path}
  </svg>
{/if}
