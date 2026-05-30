<script lang="ts">
  import { register } from '$stores/auth';
  import { goto } from '$app/navigation';
  import Button from '$components/ui/Button.svelte';
  import Input from '$components/ui/Input.svelte';
  import { AlertCircle } from '@lucide/svelte';

  let username = $state('');
  let displayName = $state('');
  let password = $state('');
  let confirmPassword = $state('');
  let error = $state('');
  let loading = $state(false);

  async function handleSubmit(e: Event) {
    e.preventDefault();
    error = '';

    if (password !== confirmPassword) {
      error = 'Passwords do not match';
      return;
    }
    if (password.length < 8) {
      error = 'Password must be at least 8 characters';
      return;
    }
    if (username.length < 3) {
      error = 'Username must be at least 3 characters';
      return;
    }

    loading = true;
    try {
      await register(username, displayName || username, password);
      goto('/');
    } catch (err) {
      error = err instanceof Error ? err.message : 'Registration failed';
    } finally {
      loading = false;
    }
  }
</script>

<div>
  <p class="text-2xs font-semibold uppercase tracking-[0.18em] text-copper">Join Cove</p>
  <h1 class="mt-1.5 font-display text-3xl font-semibold text-linen">Make yourself at home</h1>
  <p class="mt-2 text-sm text-linen-muted">A handle, a password, and you're in.</p>

  {#if error}
    <div
      class="mt-6 flex items-center gap-2.5 rounded-xl border border-danger/40 bg-danger-soft px-3.5 py-3 text-sm text-danger"
    >
      <AlertCircle size={16} class="shrink-0" />
      <span>{error}</span>
    </div>
  {/if}

  <form onsubmit={handleSubmit} class="mt-6 space-y-4">
    <div class="space-y-1.5">
      <label for="username" class="text-xs font-medium text-linen-dim">Username</label>
      <Input
        id="username"
        type="text"
        bind:value={username}
        placeholder="3–32 chars · letters, numbers, _"
        required
      />
    </div>
    <div class="space-y-1.5">
      <label for="displayName" class="text-xs font-medium text-linen-dim">Display name</label>
      <Input
        id="displayName"
        type="text"
        bind:value={displayName}
        placeholder="What should we call you?"
      />
    </div>
    <div class="space-y-1.5">
      <label for="password" class="text-xs font-medium text-linen-dim">Password</label>
      <Input
        id="password"
        type="password"
        bind:value={password}
        placeholder="At least 8 characters"
        required
      />
    </div>
    <div class="space-y-1.5">
      <label for="confirm" class="text-xs font-medium text-linen-dim">Confirm password</label>
      <Input
        id="confirm"
        type="password"
        bind:value={confirmPassword}
        placeholder="Type it again"
        required
      />
    </div>
    <Button type="submit" full size="lg" loading={loading}>
      {loading ? 'Creating account…' : 'Create account'}
    </Button>
  </form>

  <p class="mt-6 text-sm text-linen-muted">
    Already have an account?
    <a href="/login" class="font-medium text-copper-bright transition-colors hover:text-copper"
      >Sign in</a
    >
  </p>
</div>
