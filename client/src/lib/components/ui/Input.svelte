<script lang="ts">
  import type { HTMLInputAttributes } from 'svelte/elements';

  interface Props extends HTMLInputAttributes {
    value?: string;
    invalid?: boolean;
    class?: string;
  }

  let { value = $bindable(''), invalid = false, class: cls = '', ...rest }: Props = $props();
</script>

<input
  bind:value
  class="h-10 w-full rounded-xl border bg-elevated px-3.5 text-sm text-linen transition-colors duration-200 placeholder:text-linen-muted focus:outline-none focus-visible:outline-none {invalid
    ? 'border-danger focus:border-danger'
    : 'border-line-strong focus:border-copper'} {cls}"
  {...rest}
/>
