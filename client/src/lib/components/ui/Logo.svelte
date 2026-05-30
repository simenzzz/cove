<script module lang="ts">
  // Unique gradient ids per instance so multiple logos can coexist on a page
  // without colliding on duplicate SVG <defs> ids.
  let counter = 0;
</script>

<script lang="ts">
  interface Props {
    size?: number;
    /** Show the "Cove" wordmark beside the mark. */
    wordmark?: boolean;
    class?: string;
  }

  let { size = 32, wordmark = false, class: cls = '' }: Props = $props();
  const uid = `cove-${++counter}`;
</script>

<span class="inline-flex items-center gap-2.5 {cls}">
  <svg
    width={size}
    height={size}
    viewBox="0 0 32 32"
    fill="none"
    aria-hidden="true"
    class="shrink-0"
  >
    <!-- The cove: a sheltering arc cradling a resting sun -->
    <path
      d="M23.6 7A11 11 0 1 0 23.6 25"
      stroke="url(#{uid}-arc)"
      stroke-width="4.3"
      stroke-linecap="round"
    />
    <circle cx="18.4" cy="16" r="4.2" fill="url(#{uid}-dot)" />
    <defs>
      <linearGradient id="{uid}-arc" x1="4" y1="5" x2="22" y2="27" gradientUnits="userSpaceOnUse">
        <stop stop-color="#D67E4E" />
        <stop offset="1" stop-color="#C2693C" />
      </linearGradient>
      <linearGradient id="{uid}-dot" x1="14" y1="12" x2="22" y2="20" gradientUnits="userSpaceOnUse">
        <stop stop-color="#63A59C" />
        <stop offset="1" stop-color="#4E8C84" />
      </linearGradient>
    </defs>
  </svg>
  {#if wordmark}
    <span class="font-display text-xl font-semibold tracking-tight text-linen">cove</span>
  {/if}
</span>
