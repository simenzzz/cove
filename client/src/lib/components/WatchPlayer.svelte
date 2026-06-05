<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { createYouTubePlayer, type YouTubePlayer } from '$lib/watch/youtube-player';
  import { createSyncController, type SyncController } from '$lib/watch/sync-controller';
  import { sendPlayback, sendProgress, type WatchPlayback } from '$stores/watch';

  let {
    channelId,
    playback,
    isLeader,
  }: { channelId: string; playback: WatchPlayback; isLeader: boolean } = $props();

  let mountEl: HTMLDivElement;
  let player: YouTubePlayer | null = null;
  let sync: SyncController | null = null;
  let lastApplyKey = '';
  /// Periodic progress reporter — leader only. Owns its own interval so we
  /// don't interleave it with the apply/reconcile effects.
  let progressInterval: ReturnType<typeof setInterval> | null = null;

  // Apply transitions whenever the authoritative playback changes. Use a
  // string key (action + ts + video) so we don't re-apply identical updates
  // and accidentally yank the leader's seek bar.
  $effect(() => {
    const key = `${playback.video_id ?? ''}|${playback.server_ts}|${playback.paused}|${playback.position_ms}|${playback.rate}`;
    if (!sync || key === lastApplyKey) return;
    lastApplyKey = key;
    sync.apply(playback, isLeader);
  });

  // Reconcile on every pulse — the `server_ts` updates each tick. The
  // controller decides whether to actually correct.
  $effect(() => {
    if (!sync) return;
    sync.reconcile(playback, isLeader);
  });

  onMount(async () => {
    player = createYouTubePlayer(mountEl);
    await player.ready;
    sync = createSyncController(player);
    // Initial hydrate.
    sync.apply(playback, isLeader);

    // The leader's local player events flow upstream to the server. Followers
    // never emit because their iframe is covered by the follower overlay.
    player.on((e) => {
      if (!isLeader) return;
      switch (e.kind) {
        case 'play':
          sendPlayback(channelId, 'play', player!.getPosition());
          break;
        case 'pause':
          sendPlayback(channelId, 'pause', player!.getPosition());
          break;
        case 'seek':
          sendPlayback(channelId, 'seek', e.position_ms);
          break;
        case 'rate':
          if (Math.abs(e.rate - playback.rate) > 0.001) {
            sendPlayback(channelId, 'rate', player!.getPosition(), e.rate);
          }
          break;
        case 'ended':
          // Nudge the server with a final progress at full duration so it
          // detects completion + auto-advances even when YouTube's `ended`
          // arrives before our next interval tick.
          sendProgress(channelId, player!.getDuration());
          break;
      }
    });
  });

  // Re-arm the leader progress reporter whenever leadership changes. The 5s
  // cadence matches the server's sync pulse interval — frequent enough that
  // completion detection fires within seconds of the threshold without
  // generating unnecessary traffic.
  $effect(() => {
    if (progressInterval) {
      clearInterval(progressInterval);
      progressInterval = null;
    }
    if (!isLeader) return;
    progressInterval = setInterval(() => {
      if (!player) return;
      sendProgress(channelId, player.getPosition());
    }, 5_000);
  });

  onDestroy(() => {
    if (progressInterval) {
      clearInterval(progressInterval);
      progressInterval = null;
    }
    sync?.stop();
    player?.destroy();
    player = null;
    sync = null;
  });
</script>

<div class="watch-player-shell">
  <div class="iframe-mount" bind:this={mountEl}></div>
  {#if !isLeader}
    <div class="follower-overlay" title="Only the leader can control playback"></div>
  {/if}
</div>

<style>
  .watch-player-shell {
    position: relative;
    width: 100%;
    aspect-ratio: 16 / 9;
    background: #000;
  }
  .iframe-mount {
    width: 100%;
    height: 100%;
  }
  /* For followers, swallow pointer events so they can't use the native
     iframe controls while still seeing room-synced playback. */
  .follower-overlay {
    position: absolute;
    inset: 0;
    pointer-events: auto;
    cursor: not-allowed;
    background: transparent;
  }
</style>
