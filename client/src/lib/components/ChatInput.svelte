<script lang="ts">
  import { wsClient } from '$lib/ws/client';
  import { addOptimisticMessage } from '$stores/chat';
  import { auth } from '$stores/auth';
  import { SendHorizontal } from '@lucide/svelte';
  import IconButton from '$components/ui/IconButton.svelte';

  let { channelId }: { channelId: string } = $props();
  let message = $state('');
  let lastTypingSent = 0;

  function handleSubmit(e: Event) {
    e.preventDefault();
    const content = message.trim();
    if (!content || !channelId) return;

    const nonce = crypto.randomUUID();
    const user = $auth.user;

    // Optimistic insert
    addOptimisticMessage(channelId, {
      id: nonce,
      nonce,
      content,
      authorId: user?.id ?? '',
      authorUsername: user?.username ?? '',
      authorDisplayName: user?.displayName ?? user?.username ?? '',
      channelId,
      createdAt: new Date().toISOString(),
      status: 'pending',
    });

    // Send via WebSocket
    wsClient.send({
      v: 1,
      type: 'chat_message',
      channel_id: channelId,
      content,
      nonce,
    });

    message = '';
  }

  function handleInput() {
    // Throttle typing indicator to 3 seconds
    const now = Date.now();
    if (now - lastTypingSent < 3000) return;
    lastTypingSent = now;

    wsClient.send({
      v: 1,
      type: 'typing',
      channel_id: channelId,
    });
  }
</script>

<form onsubmit={handleSubmit} class="border-t border-line p-4">
  <div
    class="flex items-center gap-2 rounded-2xl border border-line-strong bg-elevated px-3 transition-colors focus-within:border-copper"
  >
    <input
      type="text"
      bind:value={message}
      oninput={handleInput}
      aria-label="Message"
      placeholder="Type a message…"
      class="flex-1 bg-transparent py-3 text-sm text-linen outline-none placeholder:text-linen-muted"
    />
    <IconButton
      type="submit"
      variant="primary"
      size="sm"
      label="Send message"
      class="shrink-0"
      disabled={!message.trim()}
    >
      <SendHorizontal size={16} />
    </IconButton>
  </div>
</form>
