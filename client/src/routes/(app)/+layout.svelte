<script lang="ts">
  import { onMount } from 'svelte';
  import { isAuthenticated } from '$stores/auth';
  import { wsClient } from '$lib/ws/client';
  import { initBridge } from '$lib/ws/bridge';
  import { fetchServers } from '$stores/servers';
  import ServerSidebar from '$components/ServerSidebar.svelte';

  let { children } = $props();
  let bridgeCleanup: (() => void) | null = null;

  onMount(async () => {
    if (!$isAuthenticated) return;

    try {
      await wsClient.connect();
      bridgeCleanup = initBridge();
      await fetchServers();
    } catch (err) {
      console.error('Failed to initialize app:', err);
    }
  });
</script>

<div class="flex h-screen">
  <ServerSidebar />
  <main class="flex-1 overflow-y-auto">
    {@render children()}
  </main>
</div>
