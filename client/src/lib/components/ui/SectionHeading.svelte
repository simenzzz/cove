<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    title: string;
    eyebrow?: string;
    subtitle?: string;
    class?: string;
    actions?: Snippet;
  }

  let { title, eyebrow, subtitle, class: cls = '', actions }: Props = $props();
</script>

<div class="flex items-end justify-between gap-4 {cls}">
  <div>
    {#if eyebrow}
      <div class="mb-1 text-2xs font-semibold uppercase tracking-[0.18em] text-copper">
        {eyebrow}
      </div>
    {/if}
    <h2 class="font-display text-2xl font-semibold text-linen">{title}</h2>
    {#if subtitle}
      <p class="mt-1 text-sm text-linen-muted">{subtitle}</p>
    {/if}
  </div>
  {#if actions}
    <div class="shrink-0">{@render actions()}</div>
  {/if}
</div>
