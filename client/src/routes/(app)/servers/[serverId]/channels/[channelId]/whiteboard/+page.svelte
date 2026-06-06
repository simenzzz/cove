<script lang="ts">
  import { page } from '$app/state';
  import ChannelList from '$components/ChannelList.svelte';
  import DrawingTools from '$components/DrawingTools.svelte';
  import Whiteboard from '$components/Whiteboard.svelte';
  import WhiteboardLayer from '$components/WhiteboardLayer.svelte';
  import ConfirmDialog from '$components/ui/ConfirmDialog.svelte';
  import TextInputDialog from '$components/ui/TextInputDialog.svelte';
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
  let saveVersionOpen = $state(false);
  let restoreOpen = $state(false);
  let checkpointToRestore: WhiteboardCheckpoint | null = $state(null);
  let loadToken = 0;

  $effect(() => {
    const sid = serverId;
    const cid = channelId;
    if (!sid || !cid) return;

    const token = ++loadToken;
    snapshot = null;
    checkpoints = [];
    provider = null;
    error = '';
    checkpointToRestore = null;
    saveVersionOpen = false;
    restoreOpen = false;

    void (async () => {
      await fetchChannels(sid);
      try {
        const [nextSnapshot, nextCheckpoints] = await Promise.all([
          fetchWhiteboard(cid),
          listCheckpoints(cid),
        ]);
        if (token !== loadToken) return;
        snapshot = nextSnapshot;
        checkpoints = nextCheckpoints;
      } catch (e) {
        if (token !== loadToken) return;
        error = e instanceof Error ? e.message : String(e);
      }
    })();
  });

  function onSaveVersion() {
    saveVersionOpen = true;
  }

  async function saveVersion(labelInput: string) {
    if (!channelId) return;
    busy = true;
    try {
      const label = labelInput.trim() || undefined;
      await createCheckpoint(channelId, label);
      checkpoints = await listCheckpoints(channelId);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  function onRestore(cp: WhiteboardCheckpoint) {
    if (!channelId) return;
    checkpointToRestore = cp;
    restoreOpen = true;
  }

  async function restoreSelectedCheckpoint() {
    if (!channelId || !checkpointToRestore) return;
    const cp = checkpointToRestore;
    busy = true;
    try {
      await restoreCheckpoint(channelId, checkpointIdToString(cp));
      // WhiteboardClosed will fire; provider re-subscribes and pulls fresh.
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
      checkpointToRestore = null;
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
        {#key channelId}
          <Whiteboard
            {channelId}
            initialStateB64={snapshot.state_b64}
            onReady={(p) => (provider = p)}
          />
        {/key}
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

<TextInputDialog
  bind:open={saveVersionOpen}
  title="Save whiteboard version"
  label="Version label"
  placeholder="Optional label"
  submitLabel="Save version"
  onsubmit={saveVersion}
/>

<ConfirmDialog
  bind:open={restoreOpen}
  title="Restore version"
  confirmLabel="Restore"
  danger
  onconfirm={restoreSelectedCheckpoint}
  oncancel={() => (checkpointToRestore = null)}
>
  <p>
    Restore "{checkpointToRestore?.label ?? 'snapshot'}"? Current whiteboard state will be
    overwritten.
  </p>
</ConfirmDialog>
