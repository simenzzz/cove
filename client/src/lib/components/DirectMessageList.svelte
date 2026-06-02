<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { MessageCircle } from '@lucide/svelte';
  import UserAvatar from '$components/UserAvatar.svelte';
  import { friends } from '$stores/friends';
  import { toasts } from '$stores/toast';
  import { directMessages, dmId, dmTargets, openDm } from '$stores/direct-messages';

  const activeDmId = $derived(page.params?.channelId ?? '');
  const targets = $derived(dmTargets($directMessages.values(), $friends.friends));
  let openingUserId = $state<string | null>(null);

  async function openTarget(userId: string) {
    if (!userId || openingUserId) return;
    openingUserId = userId;
    try {
      const dm = await openDm(userId);
      await goto(`/dms/${dmId(dm)}`);
    } catch (err) {
      toasts.error(err instanceof Error ? err.message : 'Could not open messages.');
    } finally {
      openingUserId = null;
    }
  }
</script>

<section class="mt-4">
  <div class="mb-1.5 flex items-center justify-between px-1.5">
    <p class="text-2xs font-semibold uppercase tracking-[0.16em] text-linen-muted">
      Direct Messages
    </p>
  </div>

  {#each targets as target (target.key)}
    {@const active = target.channelId !== null && activeDmId === target.channelId}
    {@const rowClass = `group mb-0.5 flex w-full items-center gap-2 rounded-lg px-2 py-1.5 text-left text-sm transition-colors ${
      active ? 'bg-teal-soft text-linen' : 'text-linen-dim hover:bg-elevated hover:text-linen'
    }`}
    {#if target.channelId}
      <a href="/dms/{target.channelId}" class={rowClass}>
        <UserAvatar username={target.label} avatarUrl={target.friend.avatar_url} size="sm" />
        <span class="min-w-0 flex-1 truncate">{target.label}</span>
        <MessageCircle size={14} class={active ? 'text-teal-bright' : 'text-linen-muted'} />
      </a>
    {:else}
      <button
        type="button"
        class={rowClass}
        onclick={() => openTarget(target.userId)}
        disabled={openingUserId === target.userId}
      >
        <UserAvatar username={target.label} avatarUrl={target.friend.avatar_url} size="sm" />
        <span class="min-w-0 flex-1 truncate">{target.label}</span>
        <MessageCircle size={14} class="text-linen-muted" />
      </button>
    {/if}
  {/each}
</section>
