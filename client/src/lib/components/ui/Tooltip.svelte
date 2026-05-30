<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    text: string;
    side?: 'top' | 'bottom' | 'right' | 'left';
    class?: string;
    children: Snippet;
  }

  let { text, side = 'top', class: cls = '', children }: Props = $props();

  const positions: Record<string, string> = {
    top: 'bottom-full left-1/2 -translate-x-1/2 mb-2',
    bottom: 'top-full left-1/2 -translate-x-1/2 mt-2',
    right: 'left-full top-1/2 -translate-y-1/2 ml-2',
    left: 'right-full top-1/2 -translate-y-1/2 mr-2',
  };
</script>

<span class="group relative inline-flex {cls}">
  {@render children()}
  <span
    role="tooltip"
    class="pointer-events-none absolute z-50 hidden whitespace-nowrap rounded-lg border border-line-strong bg-overlay px-2.5 py-1.5 text-xs font-medium text-linen opacity-0 shadow-lift transition-opacity duration-150 group-hover:block group-hover:opacity-100 {positions[
      side
    ]}"
  >
    {text}
  </span>
</span>
