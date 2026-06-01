<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import FriendSuggestions from '$components/FriendSuggestions.svelte';
  import UserAvatar from '$components/UserAvatar.svelte';
  import {
    friends,
    fetchFriends,
    friendId,
    lookupUser,
    sendRequest,
    acceptRequest,
    declineRequest,
    removeFriend,
    type FriendUser,
  } from '$stores/friends';
  import { presence } from '$stores/presence';
  import { auth } from '$stores/auth';
  import { toasts } from '$stores/toast';
  import Button from '$components/ui/Button.svelte';
  import Input from '$components/ui/Input.svelte';
  import SectionHeading from '$components/ui/SectionHeading.svelte';
  import { UserPlus, Check, MessageCircle, X } from '@lucide/svelte';
  import { openDm, dmId } from '$stores/direct-messages';

  // ── Add friend by username ──
  let username = $state('');
  let adding = $state(false);
  let addError = $state<string | null>(null);
  let addSuccess = $state<string | null>(null);

  async function addFriend(e: Event) {
    e.preventDefault();
    addError = null;
    addSuccess = null;
    const name = username.trim();
    if (!name) return;
    if (name.toLowerCase() === $auth.user?.username?.toLowerCase()) {
      addError = "You can't add yourself.";
      return;
    }
    adding = true;
    try {
      const user = await lookupUser(name);
      await sendRequest(friendId(user));
      addSuccess = `Request sent to @${user.username}.`;
      username = '';
    } catch (err) {
      addError = err instanceof Error ? err.message : 'Could not send request.';
    } finally {
      adding = false;
    }
  }

  // ── Pending / friends lists ──
  let busyId = $state<string | null>(null);

  async function accept(user: FriendUser) {
    busyId = friendId(user);
    try {
      await acceptRequest(friendId(user));
    } catch (err) {
      toasts.error(err instanceof Error ? err.message : 'Could not accept request.');
    } finally {
      busyId = null;
    }
  }

  async function decline(user: FriendUser) {
    busyId = friendId(user);
    try {
      await declineRequest(friendId(user));
    } catch (err) {
      toasts.error(err instanceof Error ? err.message : 'Could not decline request.');
    } finally {
      busyId = null;
    }
  }

  async function remove(user: FriendUser) {
    busyId = friendId(user);
    try {
      await removeFriend(friendId(user));
    } catch (err) {
      toasts.error(err instanceof Error ? err.message : 'Could not remove friend.');
    } finally {
      busyId = null;
    }
  }

  async function messageFriend(user: FriendUser) {
    busyId = friendId(user);
    try {
      const dm = await openDm(friendId(user));
      await goto(`/dms/${dmId(dm)}`);
    } catch (err) {
      toasts.error(err instanceof Error ? err.message : 'Could not open messages.');
    } finally {
      busyId = null;
    }
  }

  function isOnline(user: FriendUser): boolean {
    return $presence.statuses.get(friendId(user)) === 'online';
  }

  const sortedFriends = $derived(
    [...$friends.friends].sort((a, b) => {
      const onlineDiff = Number(isOnline(b)) - Number(isOnline(a));
      if (onlineDiff !== 0) return onlineDiff;
      return (a.display_name || a.username).localeCompare(b.display_name || b.username);
    }),
  );

  onMount(fetchFriends);
</script>

<div class="mx-auto max-w-2xl px-4 py-8">
  <SectionHeading eyebrow="Your circle" title="Friends" />

  <!-- Add friend -->
  <form onsubmit={addFriend} class="mt-7">
    <div class="flex gap-2.5">
      <Input
        bind:value={username}
        aria-label="Add a friend by username"
        oninput={() => {
          addError = null;
          addSuccess = null;
        }}
        placeholder="Add a friend by username"
        class="flex-1"
      />
      <Button type="submit" disabled={adding || !username.trim()} class="shrink-0">
        <UserPlus size={16} />
        {adding ? 'Sending…' : 'Send request'}
      </Button>
    </div>
    {#if addError}
      <p class="mt-2 text-sm text-danger">{addError}</p>
    {:else if addSuccess}
      <p class="mt-2 text-sm text-success">{addSuccess}</p>
    {/if}
  </form>

  <!-- Pending requests -->
  {#if $friends.pending.length > 0}
    <section class="mt-9">
      <h2 class="mb-3 font-display text-lg font-semibold text-linen">
        Pending requests
        <span class="ml-1 text-sm font-normal text-linen-muted">({$friends.pending.length})</span>
      </h2>
      <ul class="space-y-2.5">
        {#each $friends.pending as user (friendId(user))}
          <li class="flex items-center gap-3 rounded-2xl border border-line bg-surface p-3">
            <UserAvatar username={user.display_name || user.username} avatarUrl={user.avatar_url} size="sm" />
            <div class="min-w-0 flex-1">
              <p class="truncate font-medium text-linen">{user.display_name || user.username}</p>
              <p class="truncate text-xs text-linen-muted">@{user.username}</p>
            </div>
            <Button
              variant="secondary"
              size="sm"
              class="shrink-0"
              onclick={() => accept(user)}
              disabled={busyId === friendId(user)}
            >
              <Check size={15} /> Accept
            </Button>
            <Button
              variant="ghost"
              size="sm"
              class="shrink-0"
              onclick={() => decline(user)}
              disabled={busyId === friendId(user)}
            >
              <X size={15} /> Decline
            </Button>
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  <!-- All friends -->
  <section class="mt-9">
    <h2 class="mb-3 font-display text-lg font-semibold text-linen">
      All friends
      <span class="ml-1 text-sm font-normal text-linen-muted">({$friends.friends.length})</span>
    </h2>
    {#if sortedFriends.length === 0}
      <p class="rounded-2xl border border-dashed border-line-strong bg-surface/40 px-5 py-10 text-center text-sm text-linen-muted">
        No friends yet. Add someone by username above.
      </p>
    {:else}
      <ul class="space-y-2.5">
        {#each sortedFriends as user (friendId(user))}
          <li class="group flex items-center gap-3 rounded-2xl border border-line bg-surface p-3">
            <div class="relative shrink-0">
              <UserAvatar username={user.display_name || user.username} avatarUrl={user.avatar_url} size="sm" />
              <span
                class="absolute -bottom-0.5 -right-0.5 h-3 w-3 rounded-full border-2 border-surface {isOnline(
                  user,
                )
                  ? 'bg-success'
                  : 'bg-linen-faint'}"
                title={isOnline(user) ? 'Online' : 'Offline'}
              ></span>
            </div>
            <div class="min-w-0 flex-1">
              <p class="truncate font-medium text-linen">{user.display_name || user.username}</p>
              <p class="truncate text-xs text-linen-muted">@{user.username}</p>
            </div>
            <Button
              variant="secondary"
              size="sm"
              class="shrink-0 opacity-0 transition-opacity group-hover:opacity-100 focus:opacity-100"
              onclick={() => messageFriend(user)}
              disabled={busyId === friendId(user)}
            >
              <MessageCircle size={15} /> Message
            </Button>
            <Button
              variant="ghost"
              size="sm"
              class="shrink-0 opacity-0 transition-opacity group-hover:opacity-100 focus:opacity-100"
              onclick={() => remove(user)}
              disabled={busyId === friendId(user)}
            >
              Remove
            </Button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <div class="mt-9">
    <FriendSuggestions />
  </div>
</div>
