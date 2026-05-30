<script lang="ts">
  import type { Snippet } from 'svelte';

  type Tone = 'copper' | 'teal' | 'neutral' | 'success' | 'warn' | 'danger';

  interface Props {
    tone?: Tone;
    class?: string;
    children: Snippet;
  }

  let { tone = 'neutral', class: cls = '', children }: Props = $props();

  const tones: Record<Tone, string> = {
    copper: 'bg-copper-soft text-copper-bright',
    teal: 'bg-teal-soft text-teal-bright',
    neutral: 'bg-elevated text-linen-dim',
    success: 'bg-teal-soft text-success',
    warn: 'bg-copper-soft text-warn',
    danger: 'bg-danger-soft text-danger',
  };
</script>

<span
  class="inline-flex items-center gap-1 rounded-full px-2.5 py-0.5 text-2xs font-semibold uppercase tracking-wide {tones[
    tone
  ]} {cls}"
>
  {@render children()}
</span>
