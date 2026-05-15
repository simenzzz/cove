<script lang="ts">
  import '../app.css';
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

<div class="min-h-screen bg-gray-900 text-gray-100">
  {#if $auth.loading}
    <div class="flex items-center justify-center min-h-screen">
      <div class="text-gray-400">Loading...</div>
    </div>
  {:else}
    {@render children()}
  {/if}
</div>
