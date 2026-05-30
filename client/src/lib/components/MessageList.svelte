<script lang="ts">
  import { activeChannelMessages } from '$stores/chat';
  import UserAvatar from '$components/UserAvatar.svelte';

  let container: HTMLDivElement | undefined = $state();

  function authorLabel(message: {
    authorDisplayName?: string;
    authorUsername?: string;
    authorId: string;
  }): string {
    return message.authorDisplayName || message.authorUsername || message.authorId;
  }

  function timeLabel(iso: string | undefined): string {
    if (!iso) return '';
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return '';
    return d.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' });
  }

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

<div bind:this={container} class="space-y-0.5 overflow-y-auto">
  {#each $activeChannelMessages as message (message.id)}
    <div class="group flex gap-3 rounded-lg px-2 py-1.5 transition-colors hover:bg-surface/60">
      <div class="mt-0.5 shrink-0">
        <UserAvatar
          username={authorLabel(message)}
          avatarUrl={message.authorAvatarUrl}
          size="sm"
        />
      </div>
      <div class="min-w-0 flex-1">
        <div class="flex items-baseline gap-2">
          <span class="font-display text-sm font-semibold text-copper-bright">
            {authorLabel(message)}
          </span>
          {#if message.createdAt}
            <span class="text-2xs text-linen-muted">{timeLabel(message.createdAt)}</span>
          {/if}
          {#if message.status === 'pending'}
            <span class="text-2xs text-linen-muted">Sending…</span>
          {:else if message.status === 'failed'}
            <span class="text-2xs text-danger">Failed to send</span>
          {/if}
        </div>
        <p class="whitespace-pre-wrap break-words text-sm leading-relaxed text-linen-dim">
          {message.content}
        </p>
      </div>
    </div>
  {:else}
    <div class="flex flex-col items-center py-16 text-center">
      <p class="font-display text-lg font-semibold text-linen">No messages yet</p>
      <p class="mt-1 text-sm text-linen-muted">Say hello and start the conversation.</p>
    </div>
  {/each}
</div>
