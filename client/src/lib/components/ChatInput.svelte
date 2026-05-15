<script lang="ts">
  import { wsClient } from '$lib/ws/client';
  import { addOptimisticMessage } from '$stores/chat';
  import { auth } from '$stores/auth';

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

<form onsubmit={handleSubmit} class="p-4 border-t border-gray-700">
  <input
    type="text"
    bind:value={message}
    oninput={handleInput}
    placeholder="Type a message..."
    class="w-full p-3 bg-gray-700 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500"
  />
</form>
