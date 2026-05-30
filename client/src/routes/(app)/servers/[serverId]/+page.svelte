<script lang="ts">
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { channels, fetchChannels, type Channel } from '$stores/channels';
  import { recordKey } from '$lib/utils/record-id';

  const serverId = $derived(page.params.serverId ?? '');

  let loading = $state(true);
  let empty = $state(false);

  // Show the most useful channel first when entering a server.
  const TYPE_PRIORITY: Record<string, number> = {
    text: 0,
    collab: 1,
    whiteboard: 2,
    watch: 3,
    voice: 4,
  };

  function channelHref(channel: Channel): string {
    const id = recordKey(channel.id);
    if (channel.channel_type === 'voice') {
      return `/servers/${serverId}/channels/${id}/voice`;
    }
    if (channel.channel_type === 'collab') {
      return `/servers/${serverId}/channels/${id}/collab`;
    }
    if (channel.channel_type === 'whiteboard') {
      return `/servers/${serverId}/channels/${id}/whiteboard`;
    }
    if (channel.channel_type === 'watch') {
      return `/servers/${serverId}/channels/${id}/watch`;
    }
    return `/servers/${serverId}/channels/${id}`;
  }

  function pickFirst(list: Channel[]): Channel | null {
    if (list.length === 0) return null;
    return [...list].sort(
      (a, b) =>
        (TYPE_PRIORITY[a.channel_type] ?? 99) - (TYPE_PRIORITY[b.channel_type] ?? 99),
    )[0];
  }

  onMount(async () => {
    if (!serverId) {
      empty = true;
      loading = false;
      return;
    }
    await fetchChannels(serverId);
    const list = get(channels).get(serverId) ?? [];
    const first = pickFirst(list);
    if (first) {
      goto(channelHref(first), { replaceState: true });
    } else {
      empty = true;
      loading = false;
    }
  });
</script>

<div class="flex h-full items-center justify-center px-6 text-center">
  {#if loading && !empty}
    <div class="flex flex-col items-center gap-3">
      <div class="h-9 w-9 animate-pulse-glow rounded-2xl bg-gradient-to-br from-copper-bright to-copper-deep"></div>
      <p class="text-sm text-linen-muted">Opening server…</p>
    </div>
  {:else if empty}
    <div>
      <h1 class="mb-2 font-display text-xl font-semibold text-linen">No channels yet</h1>
      <p class="text-sm text-linen-muted">This server doesn't have any channels to open.</p>
    </div>
  {/if}
</div>
