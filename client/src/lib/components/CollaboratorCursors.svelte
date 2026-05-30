<script lang="ts">
  let { peers }: { peers: Record<string, unknown> } = $props();

  // Warm-leaning identity palette — distinct hues so collaborators stay
  // distinguishable, tuned to sit on the Cove dark canvas.
  const palette = ['#c2693c', '#4e8c84', '#d9a441', '#b06a8e', '#6aa873', '#5b8fb0'];

  function colorFor(userId: string): string {
    let hash = 0;
    for (const c of userId) hash = (hash * 31 + c.charCodeAt(0)) >>> 0;
    return palette[hash % palette.length];
  }
</script>

<div class="flex min-h-6 flex-wrap items-center gap-1.5">
  {#each Object.keys(peers) as userId (userId)}
    <span
      class="rounded-full px-2.5 py-0.5 text-2xs font-semibold text-canvas"
      style="background: {colorFor(userId)}"
      title={userId}
    >
      {userId}
    </span>
  {/each}
</div>
