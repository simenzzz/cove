<script lang="ts">
  import type { Snippet } from 'svelte';

  type Variant = 'ghost' | 'solid' | 'active' | 'primary' | 'danger';
  type Size = 'sm' | 'md' | 'lg';

  interface Props {
    variant?: Variant;
    size?: Size;
    label: string;
    type?: 'button' | 'submit';
    disabled?: boolean;
    class?: string;
    children: Snippet;
    [key: string]: unknown;
  }

  let {
    variant = 'ghost',
    size = 'md',
    label,
    type = 'button',
    disabled = false,
    class: cls = '',
    children,
    ...rest
  }: Props = $props();

  const variants: Record<Variant, string> = {
    ghost: 'text-linen-dim hover:bg-elevated hover:text-linen',
    solid: 'bg-elevated text-linen hover:bg-line-strong',
    active: 'bg-copper text-canvas shadow-glow-copper',
    primary: 'bg-copper text-canvas hover:bg-copper-bright',
    danger: 'text-danger hover:bg-danger-soft',
  };

  const sizes: Record<Size, string> = {
    sm: 'h-8 w-8',
    md: 'h-10 w-10',
    lg: 'h-11 w-11',
  };

  const classes = $derived(
    `inline-flex items-center justify-center rounded-xl transition-all duration-200 ease-out-soft focus-visible:outline-none disabled:opacity-40 disabled:pointer-events-none ${variants[variant]} ${sizes[size]} ${cls}`,
  );
</script>

<button {type} class={classes} aria-label={label} title={label} {disabled} {...rest}>
  {@render children()}
</button>
