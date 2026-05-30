<script lang="ts">
  import { friends, sendRequest, friendId, type FriendUser } from '$stores/friends';
  import { toasts } from '$stores/toast';
  import UserAvatar from '$components/UserAvatar.svelte';
  import Button from '$components/ui/Button.svelte';
  import { UserPlus } from '@lucide/svelte';

  const suggestions = $derived($friends.suggestions);
  let pendingId = $state<string | null>(null);

  async function add(user: FriendUser) {
    const id = friendId(user);
    pendingId = id;
    try {
      await sendRequest(id);
      // Drop from suggestions so the list stays consistent with the request
      // we just sent (the server would otherwise keep suggesting them).
      friends.update((s) => ({
        ...s,
        suggestions: s.suggestions.filter((u) => friendId(u) !== id),
      }));
    } catch (err) {
      toasts.error(err instanceof Error ? err.message : 'Could not send friend request.');
    } finally {
      pendingId = null;
    }
  }
</script>

<section>
  <h2 class="mb-3 font-display text-lg font-semibold text-linen">Suggested friends</h2>
  {#if suggestions.length === 0}
    <p class="rounded-2xl border border-dashed border-line-strong bg-surface/40 px-5 py-10 text-center text-sm text-linen-muted">
      No suggestions yet. They appear once you have friends in common with others.
    </p>
  {:else}
    <ul class="space-y-2.5">
      {#each suggestions as user (friendId(user))}
        {@const id = friendId(user)}
        <li class="flex items-center gap-3 rounded-2xl border border-line bg-surface p-3">
          <UserAvatar username={user.display_name || user.username} avatarUrl={user.avatar_url} size="sm" />
          <div class="min-w-0 flex-1">
            <p class="truncate font-medium text-linen">{user.display_name || user.username}</p>
            <p class="truncate text-xs text-linen-muted">@{user.username}</p>
          </div>
          <Button
            size="sm"
            class="shrink-0"
            onclick={() => add(user)}
            disabled={pendingId === id}
          >
            <UserPlus size={15} />
            {pendingId === id ? 'Sending…' : 'Add'}
          </Button>
        </li>
      {/each}
    </ul>
  {/if}
</section>
