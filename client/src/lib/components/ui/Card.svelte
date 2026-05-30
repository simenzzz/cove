<script lang="ts">
  import type { Snippet } from 'svelte';

  type Variant = 'surface' | 'elevated' | 'outline';

  interface Props {
    variant?: Variant;
    interactive?: boolean;
    class?: string;
    children: Snippet;
    [key: string]: unknown;
  }

  let {
    variant = 'surface',
    interactive = false,
    class: cls = '',
    children,
    ...rest
  }: Props = $props();

  const variants: Record<Variant, string> = {
    surface: 'bg-surface border border-line',
    elevated: 'bg-elevated border border-line-strong shadow-soft',
    outline: 'bg-transparent border border-line-strong',
  };

  const classes = $derived(
    `rounded-2xl ${variants[variant]} ${interactive ? 'transition-all duration-200 ease-out-soft hover:border-copper/60 hover:shadow-lift' : ''} ${cls}`,
  );
</script>

<div class={classes} {...rest}>
  {@render children()}
</div>
