<script lang="ts">
  interface Props {
    class?: string;
    rounded?: string;
  }

  let { class: cls = 'h-4 w-full', rounded = 'rounded-lg' }: Props = $props();
</script>

<div
  class="animate-pulse bg-gradient-to-r from-elevated via-line-strong to-elevated bg-[length:200%_100%] {rounded} {cls}"
></div>
