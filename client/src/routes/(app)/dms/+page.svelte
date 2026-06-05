<script lang="ts">
  import { onMount } from 'svelte';
  import { MessageCircle, Users } from '@lucide/svelte';
  import Button from '$components/ui/Button.svelte';
  import DirectMessageList from '$components/DirectMessageList.svelte';
  import { fetchFriends, friends } from '$stores/friends';
  import { directMessages, dmTargets, fetchDms } from '$stores/direct-messages';

  const targets = $derived(dmTargets($directMessages.values(), $friends.friends));
  const hasFriends = $derived($friends.friends.length > 0);
  const hasVisibleDms = $derived($directMessages.size > 0);

  onMount(async () => {
    await Promise.all([fetchFriends(), fetchDms()]);
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

  <section class="flex flex-1 items-center justify-center px-6">
    <div
      class="max-w-md rounded-2xl border border-line bg-surface/70 px-7 py-8 text-center shadow-soft"
    >
      <div
        class="mx-auto mb-5 flex h-14 w-14 items-center justify-center rounded-2xl bg-teal-soft text-teal-bright"
      >
        {#if hasFriends}
          <MessageCircle size={26} />
        {:else}
          <Users size={26} />
        {/if}
      </div>

      {#if hasVisibleDms}
        <h1 class="font-display text-2xl font-semibold text-linen">Pick a conversation</h1>
        <p class="mt-2 text-sm leading-6 text-linen-muted">
          Choose an existing direct message or start one with a friend from the list.
        </p>
      {:else if hasFriends}
        <h1 class="font-display text-2xl font-semibold text-linen">Start a direct message</h1>
        <p class="mt-2 text-sm leading-6 text-linen-muted">
          Select a friend from the sidebar to create your first conversation.
        </p>
      {:else}
        <h1 class="font-display text-2xl font-semibold text-linen">No friends yet</h1>
        <p class="mt-2 text-sm leading-6 text-linen-muted">
          Direct messages open once you have an accepted friend.
        </p>
        <Button href="/friends" class="mt-5">
          <Users size={16} />
          Find friends
        </Button>
      {/if}
    </div>
  </section>
</div>
