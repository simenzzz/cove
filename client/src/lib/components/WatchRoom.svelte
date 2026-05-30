<script lang="ts">
  import ReactionOverlay from './ReactionOverlay.svelte';
  import WatchPlayer from './WatchPlayer.svelte';
  import WatchQueue from './WatchQueue.svelte';
  import WatchRecommendations from './WatchRecommendations.svelte';
  import WatchViewers from './WatchViewers.svelte';
  import type { WatchRoomState } from '$stores/watch';

  let {
    state,
    currentUserId,
  }: { state: WatchRoomState; currentUserId: string } = $props();

  const isLeader = $derived(state.leader_id === currentUserId);
  const hasVideo = $derived(state.playback.video_id !== null);
</script>

<div class="watch-room">
  <main class="player-pane">
    <div class="player-stack">
      {#if hasVideo}
        <WatchPlayer
          channelId={state.channel_id}
          playback={state.playback}
          {isLeader}
        />
      {:else}
        <div class="empty">
          <p class="font-display text-lg font-semibold text-linen">No video playing</p>
          <p class="mt-1.5 text-sm text-linen-muted">
            Add a YouTube URL to the queue to get started.
          </p>
        </div>
      {/if}
      <ReactionOverlay
        channelId={state.channel_id}
        reactions={state.reactions}
      />
    </div>
    {#if state.error}
      <div class="error-banner">{state.error}</div>
    {/if}
  </main>
  <div class="side-pane">
    <WatchQueue
      channelId={state.channel_id}
      queue={state.queue}
      {currentUserId}
      {isLeader}
      hasCurrentVideo={hasVideo}
    />
    <WatchRecommendations channelId={state.channel_id} />
  </div>
  <WatchViewers
    channelId={state.channel_id}
    viewers={state.viewers}
    {currentUserId}
    {isLeader}
  />
</div>

<style>
  .watch-room {
    display: flex;
    flex: 1;
    min-height: 0;
  }
  .player-pane {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    padding: 1rem;
    gap: 1rem;
  }
  .player-stack {
    position: relative;
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .side-pane {
    display: flex;
    flex-direction: column;
    overflow-y: auto;
  }
  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    background: var(--color-surface);
    border: 1px solid var(--color-line);
    border-radius: var(--radius-2xl);
    text-align: center;
  }
  .error-banner {
    background: var(--color-danger-soft);
    color: var(--color-danger);
    border: 1px solid color-mix(in oklab, var(--color-danger) 40%, transparent);
    padding: 0.5rem 0.85rem;
    border-radius: var(--radius-lg);
    font-size: 0.875rem;
  }
</style>
