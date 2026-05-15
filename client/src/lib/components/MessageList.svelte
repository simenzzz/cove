<script lang="ts">
  import { onMount } from 'svelte';
  import { activeChannelMessages, chat } from '$stores/chat';

  let container: HTMLDivElement | undefined = $state();

  // Auto-scroll to bottom when new messages arrive
  $effect(() => {
    // Read the messages to create a reactive dependency
    const msgs = $activeChannelMessages;
    if (container) {
      // Use requestAnimationFrame to ensure DOM has updated
      requestAnimationFrame(() => {
        if (container) {
          container.scrollTop = container.scrollHeight;
        }
      });
    }
  });
</script>

<div bind:this={container} class="space-y-2 overflow-y-auto">
  {#each $activeChannelMessages as message (message.id)}
    <div class="flex gap-3 p-2 hover:bg-gray-800 rounded">
      <div class="font-semibold text-sm text-indigo-400">{message.authorId}</div>
      <div class="text-sm flex-1">{message.content}</div>
      {#if message.status === 'pending'}
        <span class="text-xs text-gray-500">Sending...</span>
      {:else if message.status === 'failed'}
        <span class="text-xs text-red-400">Failed</span>
      {/if}
    </div>
  {:else}
    <p class="text-gray-500 text-center py-8">No messages yet. Start the conversation!</p>
  {/each}
</div>
