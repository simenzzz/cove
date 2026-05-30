<script lang="ts">
  import '../app.css';
  import Toast from '$components/ui/Toast.svelte';
  import { silentRefresh, auth, isAuthenticated } from '$stores/auth';
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  let { children } = $props();

  onMount(() => {
    const timeout = setTimeout(() => {
      auth.update((s) => ({ ...s, loading: false }));
    }, 5000);

    silentRefresh()
      .catch(() => auth.set({ accessToken: null, user: null, loading: false }))
      .finally(() => clearTimeout(timeout));
  });

  $effect(() => {
    if ($auth.loading) return;
    const pathname = page.url.pathname;
    const isAuthPage = pathname.startsWith('/login') || pathname.startsWith('/register');
    if (!$isAuthenticated && !isAuthPage) {
      goto('/login');
    } else if ($isAuthenticated && isAuthPage) {
      goto('/');
    }
  });
</script>

<div class="min-h-screen bg-canvas text-linen font-sans antialiased">
  {#if $auth.loading}
    <div class="flex min-h-screen items-center justify-center">
      <div class="flex flex-col items-center gap-4">
        <div
          class="h-10 w-10 animate-pulse-glow rounded-2xl bg-gradient-to-br from-copper-bright to-copper-deep"
        ></div>
        <div class="text-sm tracking-wide text-linen-muted">Settling in…</div>
      </div>
    </div>
  {:else}
    {@render children()}
  {/if}
  <Toast />
</div>
