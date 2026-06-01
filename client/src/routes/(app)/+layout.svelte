<script lang="ts">
  import { onDestroy } from 'svelte';
  import { goto } from '$app/navigation';
  import { auth, isAuthenticated } from '$stores/auth';
  import { wsClient } from '$lib/ws/client';
  import { initBridge } from '$lib/ws/bridge';
  import { fetchServers } from '$stores/servers';
  import { fetchFriends, friends } from '$stores/friends';
  import { fetchDms } from '$stores/direct-messages';
  import { notifications } from '$stores/notifications';
  import ServerSidebar from '$components/ServerSidebar.svelte';
  import { Newspaper, Compass, Users, PenLine } from '@lucide/svelte';
  import { get } from 'svelte/store';

  import { page } from '$app/state';

  let { children } = $props();
  let bridgeCleanup: (() => void) | null = null;
  // Guards the one-time WS connect + data bootstrap so the $effect below
  // doesn't re-run it on every reactive tick.
  let initialized = false;

  const navLinks = [
    { href: '/feed', label: 'Feed', icon: Newspaper },
    { href: '/explore', label: 'Explore', icon: Compass },
    { href: '/friends', label: 'Friends', icon: Users },
    { href: '/posts/new', label: 'New post', icon: PenLine },
  ];

  function isActive(href: string): boolean {
    return page.url.pathname === href || page.url.pathname.startsWith(`${href}/`);
  }

  async function initialize() {
    try {
      await wsClient.connect();
      bridgeCleanup = initBridge();
      await fetchServers();
      await fetchFriends();
      await fetchDms();
      const pendingCount = get(friends).pending.length;
      if (pendingCount > 0) {
        notifications.push({
          key: 'friends:pending-login',
          kind: 'friend_request',
          title: `${pendingCount} pending friend request${pendingCount === 1 ? '' : 's'}`,
          body: 'Review incoming requests from your friends section.',
          href: '/friends',
        });
      }
    } catch (err) {
      console.error('Failed to initialize app:', err);
    }
  }

  // Auth gate: bounce unauthenticated users to /login BEFORE child pages
  // (feed/friends/explore) can mount and fire token-less API calls — those
  // were the source of the 401 → silent-refresh retry → 429 cascade. The root
  // +layout.svelte also redirects unauthenticated users; that's intentional
  // defense-in-depth, but the `{#if $isAuthenticated}` render gate below is
  // what actually prevents the token-less fetches.
  $effect(() => {
    if ($auth.loading) return;
    if (!$isAuthenticated) {
      // Tear down session state so a re-login within this same layout
      // lifetime (logout → login without an unmount) re-initializes cleanly.
      if (initialized) {
        bridgeCleanup?.();
        bridgeCleanup = null;
        initialized = false;
      }
      goto('/login');
      return;
    }
    if (!initialized) {
      initialized = true;
      initialize();
    }
  });

  onDestroy(() => {
    bridgeCleanup?.();
  });
</script>

<div class="flex h-screen">
  {#if $isAuthenticated}
    <ServerSidebar />
    <main class="flex flex-1 flex-col overflow-y-auto">
      <nav
        class="glass sticky top-0 z-30 flex shrink-0 items-center gap-1.5 border-b border-line px-3 py-2.5"
      >
        {#each navLinks as link}
          {@const Icon = link.icon}
          <a
            href={link.href}
            aria-current={isActive(link.href) ? 'page' : undefined}
            class="inline-flex items-center gap-2 rounded-xl px-3.5 py-2 text-sm font-medium transition-all duration-200 ease-out-soft {isActive(
              link.href,
            )
              ? 'bg-copper-soft text-copper-bright'
              : 'text-linen-dim hover:bg-elevated hover:text-linen'}"
          >
            <Icon size={16} />
            {link.label}
          </a>
        {/each}
      </nav>
      <div class="min-h-0 flex-1">
        {@render children()}
      </div>
    </main>
  {:else}
    <div class="flex w-full items-center justify-center text-linen-muted">Loading…</div>
  {/if}
</div>
