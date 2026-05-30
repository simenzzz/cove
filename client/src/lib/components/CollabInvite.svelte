<script lang="ts">
  import { inviteCollaborator } from '$lib/stores/posts';
  import Input from '$components/ui/Input.svelte';
  import Button from '$components/ui/Button.svelte';
  import { UserPlus } from '@lucide/svelte';

  let { postId }: { postId: string } = $props();

  let userId = $state('');
  let pending = $state(false);
  let error = $state<string | null>(null);
  let success = $state<string | null>(null);

  async function onSubmit(e: Event) {
    e.preventDefault();
    const trimmed = userId.trim();
    if (!trimmed) {
      error = 'User id required';
      success = null;
      return;
    }
    pending = true;
    error = null;
    success = null;
    try {
      await inviteCollaborator(postId, trimmed);
      success = `Invited ${trimmed}`;
      userId = '';
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      pending = false;
    }
  }
</script>

<form onsubmit={onSubmit} class="mb-4 rounded-2xl border border-line bg-surface p-4">
  <label for="invite-user" class="text-xs font-medium text-linen-dim">Invite collaborator</label>
  <div class="mt-2 flex gap-2.5">
    <Input
      id="invite-user"
      bind:value={userId}
      placeholder="user id"
      disabled={pending}
      class="flex-1"
    />
    <Button type="submit" class="shrink-0" disabled={pending || !userId.trim()}>
      <UserPlus size={16} />
      {pending ? 'Inviting…' : 'Invite'}
    </Button>
  </div>
  {#if error}
    <p class="mt-2 text-sm text-danger">{error}</p>
  {:else if success}
    <p class="mt-2 text-sm text-success">{success}</p>
  {/if}
</form>
