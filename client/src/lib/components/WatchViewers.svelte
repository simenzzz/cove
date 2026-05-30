<script lang="ts">
  import { sendTransferLeader, type WatchViewer } from '$stores/watch';

  let {
    channelId,
    viewers,
    currentUserId,
    isLeader,
  }: {
    channelId: string;
    viewers: WatchViewer[];
    currentUserId: string;
    isLeader: boolean;
  } = $props();

  function handleTransfer(toUserId: string): void {
    if (!isLeader || toUserId === currentUserId) return;
    sendTransferLeader(channelId, toUserId);
  }
</script>

<aside class="viewer-list">
  <p class="mb-2 text-2xs font-semibold uppercase tracking-[0.16em] text-linen-muted">
    Viewers ({viewers.length})
  </p>
  <ul class="space-y-1">
    {#each viewers as v (v.user_id)}
      <li class="flex items-center gap-2 rounded-lg px-2 py-1.5 transition-colors hover:bg-elevated">
        <span class="flex-1 truncate text-sm text-linen-dim">
          {v.username}
          {#if v.user_id === currentUserId}
            <span class="text-xs text-linen-muted">(you)</span>
          {/if}
        </span>
        {#if v.is_leader}
          <span
            class="rounded-full bg-copper-soft px-1.5 py-0.5 text-xs text-copper-bright"
            title="Room leader"
          >
            ★
          </span>
        {:else if isLeader}
          <button
            class="rounded-lg border border-line-strong px-1.5 py-0.5 text-xs text-linen-dim transition-colors hover:bg-elevated hover:text-linen"
            onclick={() => handleTransfer(v.user_id)}
            title="Transfer leadership"
          >
            Promote
          </button>
        {/if}
      </li>
    {/each}
  </ul>
</aside>

<style>
  .viewer-list {
    width: 200px;
    border-left: 1px solid var(--color-line);
    padding: 0.6rem;
    overflow-y: auto;
  }
</style>
