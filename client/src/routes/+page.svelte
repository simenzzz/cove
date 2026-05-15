<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { isAuthenticated, auth } from '$stores/auth';

  onMount(() => {
    // Wait for auth to finish loading
    const unsubscribe = auth.subscribe((state) => {
      if (state.loading) return;
      if (state.accessToken) {
        goto('/feed');
      } else {
        goto('/login');
      }
      unsubscribe();
    });
  });
</script>

<div class="flex items-center justify-center min-h-screen">
  <p class="text-gray-400">Redirecting...</p>
</div>
