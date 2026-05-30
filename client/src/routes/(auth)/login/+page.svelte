<script lang="ts">
  import { login } from '$stores/auth';
  import { goto } from '$app/navigation';
  import Button from '$components/ui/Button.svelte';
  import Input from '$components/ui/Input.svelte';
  import { AlertCircle } from '@lucide/svelte';

  let username = $state('');
  let password = $state('');
  let error = $state('');
  let loading = $state(false);

  async function handleSubmit(e: Event) {
    e.preventDefault();
    error = '';
    loading = true;
    try {
      await login(username, password);
      goto('/');
    } catch (err) {
      error = err instanceof Error ? err.message : 'Login failed';
    } finally {
      loading = false;
    }
  }
</script>

<div>
  <p class="text-2xs font-semibold uppercase tracking-[0.18em] text-copper">Welcome back</p>
  <h1 class="mt-1.5 font-display text-3xl font-semibold text-linen">Good to see you again</h1>
  <p class="mt-2 text-sm text-linen-muted">Sign in to drop back into your coves.</p>

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
      <Input id="username" type="text" bind:value={username} placeholder="your-handle" required />
    </div>
    <div class="space-y-1.5">
      <label for="password" class="text-xs font-medium text-linen-dim">Password</label>
      <Input id="password" type="password" bind:value={password} placeholder="••••••••" required />
    </div>
    <Button type="submit" full size="lg" loading={loading}>
      {loading ? 'Signing in…' : 'Sign in'}
    </Button>
  </form>

  <p class="mt-6 text-sm text-linen-muted">
    New to Cove?
    <a href="/register" class="font-medium text-copper-bright transition-colors hover:text-copper"
      >Create an account</a
    >
  </p>
</div>
