<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import ChannelList from '$components/ChannelList.svelte';
  import DrawingTools from '$components/DrawingTools.svelte';
  import Whiteboard from '$components/Whiteboard.svelte';
  import WhiteboardLayer from '$components/WhiteboardLayer.svelte';
  import { fetchChannels } from '$stores/channels';
  import type { WhiteboardProvider } from '$lib/collab/whiteboard-provider';
  import {
    createCheckpoint,
    fetchWhiteboard,
    listCheckpoints,
    restoreCheckpoint,
    type WhiteboardCheckpoint,
    type WhiteboardSnapshot,
    checkpointIdToString,
  } from '$stores/whiteboards';

  const serverId = $derived(page.params.serverId ?? '');
  const channelId = $derived(page.params.channelId ?? '');

  let snapshot: WhiteboardSnapshot | null = $state(null);
  let checkpoints: WhiteboardCheckpoint[] = $state([]);
  let provider: WhiteboardProvider | null = $state(null);
  let error = $state('');
  let busy = $state(false);

  onMount(async () => {
    if (!serverId || !channelId) return;
    await fetchChannels(serverId);
    try {
      snapshot = await fetchWhiteboard(channelId);
      checkpoints = await listCheckpoints(channelId);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  });

  async function onSaveVersion() {
    if (!channelId) return;
    busy = true;
    try {
      const label = prompt('Label this version (optional):') ?? undefined;
      await createCheckpoint(channelId, label);
      checkpoints = await listCheckpoints(channelId);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  async function onRestore(cp: WhiteboardCheckpoint) {
    if (!channelId) return;
    if (!confirm(`Restore "${cp.label ?? 'snapshot'}"? Current state will be overwritten.`))
      return;
    busy = true;
    try {
      await restoreCheckpoint(channelId, checkpointIdToString(cp));
      // WhiteboardClosed will fire; provider re-subscribes and pulls fresh.
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="flex h-full">
  <ChannelList {serverId} />
  <div class="flex flex-1 flex-col gap-3 overflow-auto p-5">
    {#if error}
      <div class="rounded-xl border border-danger/40 bg-danger-soft p-2.5 text-sm text-danger">
        {error}
      </div>
    {/if}

    <DrawingTools />

    <div class="flex items-start gap-3">
      {#if snapshot}
        <Whiteboard
          {channelId}
          initialStateB64={snapshot.state_b64}
          onReady={(p) => (provider = p)}
        />
      {:else}
        <div class="text-sm text-linen-muted">Loading whiteboard…</div>
      {/if}

      <div class="flex flex-col gap-3">
        <WhiteboardLayer {provider} />

        <div class="w-48 rounded-xl border border-line bg-surface p-3 text-sm">
          <div class="mb-2 flex items-center justify-between">
            <span class="font-display font-semibold text-linen">Versions</span>
            <button
              type="button"
              class="rounded-lg bg-copper px-2 py-0.5 text-xs font-semibold text-canvas transition-colors hover:bg-copper-bright disabled:opacity-50"
              onclick={onSaveVersion}
              disabled={busy}
            >
              Save
            </button>
          </div>
          <ul class="flex max-h-64 flex-col gap-1 overflow-auto">
            {#each checkpoints as cp (checkpointIdToString(cp))}
              <li class="flex items-center justify-between gap-2">
                <span class="truncate text-linen-dim" title={cp.label ?? ''}>
                  {cp.label ?? 'snapshot'}
                </span>
                <button
                  type="button"
                  class="text-xs text-copper-bright transition-colors hover:text-copper disabled:opacity-50"
                  onclick={() => onRestore(cp)}
                  disabled={busy}
                >
                  restore
                </button>
              </li>
            {:else}
              <li class="text-xs text-linen-muted">No saved versions</li>
            {/each}
          </ul>
        </div>
      </div>
    </div>
  </div>
</div>

