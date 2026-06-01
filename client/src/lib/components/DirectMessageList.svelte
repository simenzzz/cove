<script lang="ts">
  import { page } from '$app/state';
  import { MessageCircle } from '@lucide/svelte';
  import UserAvatar from '$components/UserAvatar.svelte';
  import { directMessages, dmId, dmLabel } from '$stores/direct-messages';

  const activeDmId = $derived(page.params?.channelId ?? '');
  const dms = $derived(
    Array.from($directMessages.values()).sort((a, b) =>
      dmLabel(a).localeCompare(dmLabel(b)),
    ),
  );
</script>

<section class="mt-4">
  <div class="mb-1.5 flex items-center justify-between px-1.5">
    <p class="text-2xs font-semibold uppercase tracking-[0.16em] text-linen-muted">
      Direct Messages
    </p>
  </div>

  {#each dms as dm (dmId(dm))}
    {@const id = dmId(dm)}
    {@const active = activeDmId === id}
    <a
      href="/dms/{id}"
      class="group mb-0.5 flex items-center gap-2 rounded-lg px-2 py-1.5 text-sm transition-colors {active
        ? 'bg-teal-soft text-linen'
        : 'text-linen-dim hover:bg-elevated hover:text-linen'}"
    >
      <UserAvatar username={dmLabel(dm)} avatarUrl={dm.friend.avatar_url} size="sm" />
      <span class="min-w-0 flex-1 truncate">{dmLabel(dm)}</span>
      <MessageCircle size={14} class={active ? 'text-teal-bright' : 'text-linen-muted'} />
    </a>
  {/each}
</section>
