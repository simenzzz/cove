<script lang="ts">
  import type { HTMLTextareaAttributes } from 'svelte/elements';

  interface Props extends HTMLTextareaAttributes {
    value?: string;
    invalid?: boolean;
    class?: string;
  }

  let { value = $bindable(''), invalid = false, class: cls = '', ...rest }: Props = $props();
</script>

<textarea
  bind:value
  class="w-full rounded-xl border bg-elevated px-3.5 py-2.5 text-sm leading-relaxed text-linen transition-colors duration-200 placeholder:text-linen-muted focus:outline-none focus-visible:outline-none {invalid
    ? 'border-danger focus:border-danger'
    : 'border-line-strong focus:border-copper'} {cls}"
  {...rest}
></textarea>
