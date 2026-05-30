<script lang="ts">
  import type { Snippet } from 'svelte';

  type Variant = 'primary' | 'secondary' | 'outline' | 'ghost' | 'danger';
  type Size = 'sm' | 'md' | 'lg';

  interface Props {
    variant?: Variant;
    size?: Size;
    href?: string;
    type?: 'button' | 'submit' | 'reset';
    loading?: boolean;
    disabled?: boolean;
    full?: boolean;
    class?: string;
    children: Snippet;
    [key: string]: unknown;
  }

  let {
    variant = 'primary',
    size = 'md',
    href,
    type = 'button',
    loading = false,
    disabled = false,
    full = false,
    class: cls = '',
    children,
    ...rest
  }: Props = $props();

  const base =
    'relative inline-flex items-center justify-center gap-2 rounded-xl font-medium tracking-[0.01em] transition-all duration-200 ease-out-soft select-none focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50';

  const variants: Record<Variant, string> = {
    primary:
      'bg-copper text-canvas hover:bg-copper-bright active:bg-copper-deep shadow-soft hover:shadow-glow-copper',
    secondary: 'bg-teal text-canvas hover:bg-teal-bright shadow-soft hover:shadow-glow-teal',
    outline:
      'border border-line-strong bg-transparent text-linen hover:border-copper hover:text-copper-bright',
    ghost: 'bg-transparent text-linen-dim hover:bg-elevated hover:text-linen',
    danger: 'bg-danger text-canvas hover:brightness-110 active:brightness-95 shadow-soft',
  };

  const sizes: Record<Size, string> = {
    sm: 'h-8 px-3 text-sm',
    md: 'h-10 px-4 text-sm',
    lg: 'h-12 px-6 text-base',
  };

  const isDisabled = $derived(disabled || loading);

  const classes = $derived(
    `${base} ${variants[variant]} ${sizes[size]} ${full ? 'w-full' : ''} ${cls}`,
  );
</script>

{#snippet inner()}
  {#if loading}
    <span
      class="h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"
      aria-hidden="true"
    ></span>
  {/if}
  {@render children()}
{/snippet}

{#if href}
  <!-- When disabled/loading, drop href so the link is genuinely non-navigable. -->
  <a
    href={isDisabled ? undefined : href}
    class={classes}
    aria-disabled={isDisabled}
    tabindex={isDisabled ? -1 : undefined}
    {...rest}
  >
    {@render inner()}
  </a>
{:else}
  <button {type} class={classes} disabled={isDisabled} {...rest}>
    {@render inner()}
  </button>
{/if}
