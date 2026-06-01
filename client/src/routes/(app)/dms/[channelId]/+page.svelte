<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { page } from '$app/state';
  import { api } from '$lib/api/client';
  import { wsClient } from '$lib/ws/client';
  import { addReceivedMessage, setActiveChannel, type ChatMessage } from '$stores/chat';
  import { directMessages, dmLabel, fetchDms } from '$stores/direct-messages';
  import { recordKey } from '$lib/utils/record-id';
  import ChatInput from '$components/ChatInput.svelte';
  import DirectMessageList from '$components/DirectMessageList.svelte';
  import MessageList from '$components/MessageList.svelte';
  import UserAvatar from '$components/UserAvatar.svelte';

  const channelId = $derived(page.params.channelId ?? '');
  const dm = $derived($directMessages.get(channelId));
  const label = $derived(dm ? dmLabel(dm) : 'Direct Message');

  async function loadHistory(id: string) {
    try {
      const data = await api.get<{ messages: unknown[] }>(
        `/api/channels/${id}/messages?limit=50`,
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
          channelId: id,
          createdAt: String(msg.created_at ?? new Date().toISOString()),
          status: 'sent',
        };
        addReceivedMessage(id, chatMsg);
      }
    } catch (err) {
      console.error('Failed to load direct message history:', err);
    }
  }

  onMount(async () => {
    if (!channelId) return;
    setActiveChannel(channelId);
    await fetchDms();
    wsClient.send({
      v: 1,
      type: 'subscribe',
      channel_id: channelId,
      level: 'active',
    });
    await loadHistory(channelId);
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

  $effect(() => {
    if (channelId) setActiveChannel(channelId);
  });
</script>

<div class="flex h-full">
  <aside class="flex w-60 shrink-0 flex-col border-r border-line bg-surface/50">
    <div class="flex items-center gap-2 border-b border-line px-4 py-3.5">
      <span
        class="flex h-7 w-7 items-center justify-center rounded-lg bg-teal-soft text-teal-bright"
      >
        @
      </span>
      <span class="truncate font-display font-semibold text-linen">Messages</span>
    </div>
    <div class="flex-1 overflow-y-auto p-2.5">
      <DirectMessageList />
    </div>
  </aside>

  <div class="flex flex-1 flex-col">
    <header class="flex items-center gap-3 border-b border-line px-5 py-3.5">
      {#if dm}
        <UserAvatar username={label} avatarUrl={dm.friend.avatar_url} size="sm" />
      {/if}
      <span class="font-display font-semibold text-linen">{label}</span>
    </header>
    <div class="flex-1 overflow-y-auto px-3 py-4">
      <MessageList />
    </div>
    <ChatInput {channelId} />
  </div>
</div>
