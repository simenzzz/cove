<script lang="ts">
  import { onMount } from 'svelte';
  import {
    fetchRecommendations,
    sendQueueAdd,
    type WatchRecommendation,
  } from '$stores/watch';

  let { channelId }: { channelId: string } = $props();

  let recommendations: WatchRecommendation[] = $state([]);
  let loading = $state(true);
  let error = $state('');

  async function load(): Promise<void> {
    loading = true;
    error = '';
    try {
      recommendations = await fetchRecommendations(channelId, 10);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  onMount(load);

  function queueIt(video_id: string): void {
    sendQueueAdd(channelId, {
      video_id,
      title: `Video ${video_id}`,
      duration_ms: 0,
      thumbnail_url: `https://i.ytimg.com/vi/${video_id}/mqdefault.jpg`,
    });
  }
</script>

<section class="recs">
  <div class="header">
    <p class="text-2xs font-semibold uppercase tracking-[0.16em] text-linen-muted">
      Suggested for this room
    </p>
    <button
      class="rounded-lg border border-line-strong px-1.5 py-0.5 text-xs text-linen-dim transition-colors hover:bg-elevated hover:text-linen"
      onclick={load}
      disabled={loading}
      title="Refresh recommendations"
    >
      ↻
    </button>
  </div>
  {#if loading}
    <p class="text-xs text-linen-muted">Loading…</p>
  {:else if error}
    <p class="text-xs text-danger">{error}</p>
  {:else if recommendations.length === 0}
    <p class="text-xs text-linen-muted">
      Nothing yet — recommendations build up as your server-mates watch things.
    </p>
  {:else}
    <ul class="list">
      {#each recommendations as r (r.video_id)}
        <li class="row">
          <img
            src="https://i.ytimg.com/vi/{r.video_id}/default.jpg"
            alt=""
            class="thumb"
            loading="lazy"
          />
          <span class="vid">{r.video_id}</span>
          <span class="score">×{r.score}</span>
          <button
            class="add-btn"
            onclick={() => queueIt(r.video_id)}
            title="Add to queue"
          >
            +
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .recs {
    padding: 0.85rem;
    border-top: 1px solid var(--color-line);
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: var(--color-surface);
    border: 1px solid var(--color-line);
    border-radius: var(--radius-lg);
    padding: 0.35rem;
  }
  .thumb {
    width: 48px;
    height: 27px;
    object-fit: cover;
    border-radius: var(--radius-sm);
  }
  .vid {
    flex: 1;
    font-family: var(--font-mono);
    font-size: 0.75rem;
    color: var(--color-linen);
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .score {
    font-size: 0.7rem;
    color: var(--color-linen-muted);
  }
  .add-btn {
    background: var(--color-copper);
    color: var(--color-canvas);
    border-radius: var(--radius-md);
    padding: 0 0.5rem;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
  }
  .add-btn:hover {
    background: var(--color-copper-bright);
  }
</style>
