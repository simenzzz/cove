<script lang="ts">
  let { peers }: { peers: Record<string, unknown> } = $props();

  const palette = ['#e11d48', '#2563eb', '#059669', '#d97706', '#7c3aed', '#0891b2'];

  function colorFor(userId: string): string {
    let hash = 0;
    for (const c of userId) hash = (hash * 31 + c.charCodeAt(0)) >>> 0;
    return palette[hash % palette.length];
  }
</script>

<div class="cursors">
  {#each Object.keys(peers) as userId (userId)}
    <span class="chip" style="background: {colorFor(userId)}">{userId}</span>
  {/each}
</div>

<style>
  .cursors {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    min-height: 1.5rem;
  }

  .chip {
    color: #fff;
    font-size: 0.75rem;
    padding: 0.15rem 0.5rem;
    border-radius: 9999px;
  }
</style>
