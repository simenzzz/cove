<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { channels, createChannel, type Channel, type ChannelType } from '$stores/channels';
  import { servers } from '$stores/servers';
  import { auth } from '$stores/auth';
  import { recordKey } from '$lib/utils/record-id';
  import { Hash, Volume2, Feather, Brush, MonitorPlay, Plus, X } from '@lucide/svelte';
  import Button from '$components/ui/Button.svelte';
  import Input from '$components/ui/Input.svelte';
  import DirectMessageList from '$components/DirectMessageList.svelte';

  let { serverId }: { serverId: string } = $props();

  const serverChannels = $derived($channels.get(serverId) ?? []);
  const ownerKey = $derived(recordKey($servers.get(serverId)?.owner ?? null));
  const isOwner = $derived(ownerKey !== '' && ownerKey === $auth.user?.id);
  const serverName = $derived($servers.get(serverId)?.name ?? 'Server');
  const activeChannelId = $derived(page.params?.channelId ?? '');

  const CHANNEL_TYPES: { value: ChannelType; label: string }[] = [
    { value: 'text', label: '# Text' },
    { value: 'voice', label: 'Voice' },
    { value: 'collab', label: 'Collab doc' },
    { value: 'whiteboard', label: 'Whiteboard' },
    { value: 'watch', label: 'Watch together' },
  ];

  let creating = $state(false);
  let submitting = $state(false);
  let newName = $state('');
  let newType = $state<ChannelType>('text');
  let createError = $state<string | null>(null);

  const ICONS = {
    voice: Volume2,
    collab: Feather,
    whiteboard: Brush,
    watch: MonitorPlay,
    text: Hash,
  } as const;

  function channelIcon(kind: string) {
    return ICONS[kind as keyof typeof ICONS] ?? Hash;
  }

  function channelHref(channel: Channel): string {
    const id = recordKey(channel.id);
    if (channel.channel_type === 'voice') return `/servers/${serverId}/channels/${id}/voice`;
    if (channel.channel_type === 'collab') return `/servers/${serverId}/channels/${id}/collab`;
    if (channel.channel_type === 'whiteboard')
      return `/servers/${serverId}/channels/${id}/whiteboard`;
    if (channel.channel_type === 'watch') return `/servers/${serverId}/channels/${id}/watch`;
    return `/servers/${serverId}/channels/${id}`;
  }

  async function submitCreate(e: Event) {
    e.preventDefault();
    createError = null;
    const name = newName.trim();
    if (name.length < 1 || name.length > 64) {
      createError = 'Channel name must be 1–64 characters.';
      return;
    }
    submitting = true;
    try {
      const channel = await createChannel(serverId, name, newType);
      newName = '';
      newType = 'text';
      creating = false;
      goto(channelHref(channel));
    } catch (err) {
      createError = err instanceof Error ? err.message : 'Failed to create channel.';
    } finally {
      submitting = false;
    }
  }
</script>

<aside class="flex w-60 shrink-0 flex-col border-r border-line bg-surface/50">
  <div class="flex items-center gap-2 border-b border-line px-4 py-3.5">
    <span class="flex h-7 w-7 items-center justify-center rounded-lg bg-copper-soft font-display text-sm font-semibold text-copper-bright">
      {serverName.charAt(0).toUpperCase()}
    </span>
    <span class="truncate font-display font-semibold text-linen">{serverName}</span>
  </div>

  <div class="flex-1 overflow-y-auto p-2.5">
    <div class="mb-1.5 flex items-center justify-between px-1.5">
      <p class="text-2xs font-semibold uppercase tracking-[0.16em] text-linen-muted">Channels</p>
      {#if isOwner}
        <button
          type="button"
          title={creating ? 'Cancel' : 'Create channel'}
          aria-label={creating ? 'Cancel' : 'Create channel'}
          onclick={() => (creating = !creating)}
          class="rounded-md p-0.5 text-linen-muted transition-colors hover:text-copper-bright"
        >
          {#if creating}<X size={16} />{:else}<Plus size={16} />{/if}
        </button>
      {/if}
    </div>

    {#if creating}
      <form onsubmit={submitCreate} class="mb-2 space-y-2 rounded-xl border border-line bg-canvas/50 p-2.5">
        {#if createError}
          <p class="text-xs text-danger">{createError}</p>
        {/if}
        <Input
          bind:value={newName}
          aria-label="Channel name"
          placeholder="channel-name"
          maxlength={64}
          required
          class="h-9"
        />
        <select
          bind:value={newType}
          aria-label="Channel type"
          class="h-9 w-full rounded-xl border border-line-strong bg-elevated px-2.5 text-sm text-linen focus:border-copper focus:outline-none"
        >
          {#each CHANNEL_TYPES as t}
            <option value={t.value}>{t.label}</option>
          {/each}
        </select>
        <Button type="submit" size="sm" full loading={submitting}>
          {submitting ? 'Creating…' : 'Create channel'}
        </Button>
      </form>
    {/if}

    {#each serverChannels as channel (recordKey(channel.id))}
      {@const Icon = channelIcon(channel.channel_type)}
      {@const active = recordKey(channel.id) === activeChannelId}
      <a
        href={channelHref(channel)}
        class="group mb-0.5 flex items-center gap-2 rounded-lg px-2 py-1.5 text-sm transition-colors {active
          ? 'bg-copper-soft text-linen'
          : 'text-linen-dim hover:bg-elevated hover:text-linen'}"
      >
        <Icon size={16} class={active ? 'text-copper-bright' : 'text-linen-muted'} />
        <span class="truncate">{channel.name}</span>
      </a>
    {:else}
      <p class="px-2 py-3 text-sm text-linen-muted">No channels yet.</p>
    {/each}

    <DirectMessageList />
  </div>
</aside>
