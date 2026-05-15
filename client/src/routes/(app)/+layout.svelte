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
    <nav class="top-nav">
      <a href="/feed">Feed</a>
      <a href="/explore">Explore</a>
      <a href="/friends">Friends</a>
      <a href="/posts/new">New post</a>
    </nav>
    {@render children()}
  </main>
</div>

<style>
  .top-nav {
    display: flex;
    gap: 1rem;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid #e5e7eb;
    font-size: 0.9rem;
  }
  .top-nav a {
    color: #2563eb;
    text-decoration: none;
  }
  .top-nav a:hover {
    text-decoration: underline;
  }
</style>
