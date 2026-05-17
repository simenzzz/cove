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
    <p class="text-xs font-semibold text-gray-400 uppercase">Suggested for this room</p>
    <button
      class="text-xs px-1.5 py-0.5 rounded border border-gray-600 hover:bg-gray-700"
      onclick={load}
      disabled={loading}
      title="Refresh recommendations"
    >
      ↻
    </button>
  </div>
  {#if loading}
    <p class="text-xs text-gray-500">Loading…</p>
  {:else if error}
    <p class="text-xs text-red-400">{error}</p>
  {:else if recommendations.length === 0}
    <p class="text-xs text-gray-500">
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
    padding: 0.75rem;
    border-top: 1px solid rgb(55, 65, 81);
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: rgb(31, 41, 55);
    border-radius: 0.25rem;
    padding: 0.25rem;
  }
  .thumb {
    width: 48px;
    height: 27px;
    object-fit: cover;
    border-radius: 0.125rem;
  }
  .vid {
    flex: 1;
    font-family: ui-monospace, monospace;
    font-size: 0.75rem;
    color: rgb(229, 231, 235);
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .score {
    font-size: 0.7rem;
    color: rgb(156, 163, 175);
  }
  .add-btn {
    background: rgb(59, 130, 246);
    color: white;
    border-radius: 0.25rem;
    padding: 0 0.5rem;
    font-size: 0.85rem;
    cursor: pointer;
  }
  .add-btn:hover {
    background: rgb(37, 99, 235);
  }
</style>
