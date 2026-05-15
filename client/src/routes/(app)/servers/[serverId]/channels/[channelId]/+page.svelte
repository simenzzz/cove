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

  const serverId = $derived(page.params.serverId ?? '');
  const channelId = $derived(page.params.channelId ?? '');

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
          id: String(msg.id ?? ''),
          content: String(msg.content ?? ''),
          authorId: author ? String(author.id ?? '') : '',
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
  <div class="flex flex-col flex-1">
    <div class="flex-1 overflow-y-auto p-4">
      <MessageList />
    </div>
    <ChatInput {channelId} />
  </div>
</div>
