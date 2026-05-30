<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { page } from '$app/state';
  import MessageList from '$components/MessageList.svelte';
  import ChatInput from '$components/ChatInput.svelte';
  import ChannelList from '$components/ChannelList.svelte';
  import { wsClient } from '$lib/ws/client';
  import { setActiveChannel } from '$stores/chat';
  import { fetchChannels } from '$stores/channels';
  import { api } from '$lib/api/client';
  import { addReceivedMessage, type ChatMessage } from '$stores/chat';
  import { channels } from '$stores/channels';
  import { recordKey } from '$lib/utils/record-id';
  import { Hash } from '@lucide/svelte';

  const serverId = $derived(page.params.serverId ?? '');
  const channelId = $derived(page.params.channelId ?? '');
  const channelName = $derived(
    ($channels.get(serverId) ?? []).find((c) => recordKey(c.id) === channelId)?.name ?? '',
  );

  onMount(async () => {
    if (!serverId || !channelId) return;

    setActiveChannel(channelId);
    await fetchChannels(serverId);

    // Subscribe to channel via WS
    wsClient.send({
      v: 1,
      type: 'subscribe',
      channel_id: channelId,
      level: 'active',
    });

    // Fetch message history via REST
    try {
      const data = await api.get<{ messages: unknown[] }>(
        `/api/channels/${channelId}/messages?limit=50`,
      );
      for (const raw of data.messages) {
        const msg = raw as Record<string, unknown>;
        const author = msg.author as Record<string, unknown> | undefined;
        const chatMsg: ChatMessage = {
          id: recordKey(msg.id),
          content: String(msg.content ?? ''),
          authorId: author ? recordKey(author.id) : '',
          authorUsername: author ? String(author.username ?? '') : '',
          authorDisplayName: author ? String(author.display_name ?? '') : '',
          authorAvatarUrl: author?.avatar_url == null ? null : String(author.avatar_url),
          channelId,
          createdAt: String(msg.created_at ?? new Date().toISOString()),
          status: 'sent',
        };
        addReceivedMessage(channelId, chatMsg);
      }
    } catch (err) {
      console.error('Failed to load message history:', err);
    }
  });

  onDestroy(() => {
    if (!channelId) return;
    wsClient.send({
      v: 1,
      type: 'unsubscribe',
      channel_id: channelId,
    });
    setActiveChannel(null);
  });

  // Re-fetch channels when server changes
  $effect(() => {
    if (serverId && channelId) {
      setActiveChannel(channelId);
      fetchChannels(serverId);
    }
  });
</script>

<div class="flex h-full">
  <ChannelList {serverId} />
  <div class="flex flex-1 flex-col">
    <header class="flex items-center gap-2 border-b border-line px-5 py-3.5">
      <Hash size={18} class="text-linen-muted" />
      <span class="font-display font-semibold text-linen">{channelName || 'Channel'}</span>
    </header>
    <div class="flex-1 overflow-y-auto px-3 py-4">
      <MessageList />
    </div>
    <ChatInput {channelId} />
  </div>
</div>
