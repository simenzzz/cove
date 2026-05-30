<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { servers } from '$stores/servers';
  import { auth, logout } from '$stores/auth';
  import { recordKey } from '$lib/utils/record-id';
  import { Compass, Plus, LogOut } from '@lucide/svelte';
  import Logo from '$components/ui/Logo.svelte';
  import Tooltip from '$components/ui/Tooltip.svelte';

  let menuOpen = $state(false);
  let loggingOut = $state(false);

  const displayName = $derived($auth.user?.displayName || $auth.user?.username || '');
  const initial = $derived(displayName.charAt(0).toUpperCase() || '?');

  const activeServerId = $derived(page.params?.serverId ?? '');
  const onHome = $derived(
    page.url.pathname === '/feed' || page.url.pathname.startsWith('/feed/'),
  );

  async function handleLogout() {
    loggingOut = true;
    try {
      await logout();
    } finally {
      loggingOut = false;
      menuOpen = false;
      goto('/login');
    }
  }
</script>

<svelte:window
  onkeydown={(e) => {
    if (e.key === 'Escape') menuOpen = false;
  }}
/>

{#snippet pill(active: boolean)}
  <span
    class="absolute -left-3 top-1/2 w-1.5 -translate-y-1/2 rounded-r-full bg-copper transition-all duration-200 {active
      ? 'h-7 opacity-100'
      : 'h-2 opacity-0 group-hover:opacity-60'}"
  ></span>
{/snippet}

<nav
  class="flex w-[76px] shrink-0 flex-col items-center gap-2 border-r border-line bg-surface/60 py-3.5"
>
  <!-- Home / Feed -->
  <Tooltip text="Home" side="right">
    <a
      href="/feed"
      aria-label="Home"
      class="group relative flex h-12 w-12 items-center justify-center rounded-2xl bg-canvas/60 ring-1 ring-line transition-all duration-200 hover:rounded-xl hover:ring-copper/50"
    >
      {@render pill(onHome)}
      <Logo size={26} />
    </a>
  </Tooltip>

  <div class="my-1 h-px w-8 rounded-full bg-line-strong"></div>

  <!-- Server icons -->
  {#each Array.from($servers.values()) as server (recordKey(server.id))}
    {@const id = recordKey(server.id)}
    <Tooltip text={server.name} side="right">
      <a
        href="/servers/{id}"
        aria-label={server.name}
        class="group relative flex h-12 w-12 items-center justify-center rounded-2xl bg-elevated font-display text-sm font-semibold text-linen-dim transition-all duration-200 hover:rounded-xl hover:bg-copper hover:text-canvas {activeServerId ===
        id
          ? 'rounded-xl bg-copper text-canvas shadow-glow-copper'
          : ''}"
      >
        {@render pill(activeServerId === id)}
        {server.name.charAt(0).toUpperCase()}
      </a>
    </Tooltip>
  {/each}

  <!-- Explore / add servers -->
  <Tooltip text="Explore servers" side="right">
    <a
      href="/explore"
      aria-label="Explore servers"
      class="group flex h-12 w-12 items-center justify-center rounded-2xl bg-elevated text-teal-bright transition-all duration-200 hover:rounded-xl hover:bg-teal hover:text-canvas"
    >
      <Plus size={22} />
    </a>
  </Tooltip>

  <!-- User panel pinned to the bottom -->
  <div class="relative mt-auto">
    {#if menuOpen}
      <button
        type="button"
        aria-label="Close menu"
        class="fixed inset-0 z-10 cursor-default"
        onclick={() => (menuOpen = false)}
      ></button>

      <div
        class="glass absolute bottom-0 left-16 z-20 w-52 rounded-xl border border-line-strong p-2 shadow-lift"
        role="menu"
      >
        <div class="mb-1 border-b border-line px-2 py-2">
          <p class="truncate text-sm font-semibold text-linen">{displayName}</p>
          {#if $auth.user?.username}
            <p class="truncate text-xs text-linen-muted">@{$auth.user.username}</p>
          {/if}
        </div>
        <button
          type="button"
          role="menuitem"
          onclick={handleLogout}
          disabled={loggingOut}
          class="flex w-full items-center gap-2 rounded-lg px-2 py-2 text-left text-sm text-danger transition-colors hover:bg-danger-soft disabled:opacity-50"
        >
          <LogOut size={15} />
          {loggingOut ? 'Logging out…' : 'Log out'}
        </button>
      </div>
    {/if}

    <button
      type="button"
      aria-label="Account menu"
      aria-haspopup="menu"
      aria-expanded={menuOpen}
      onclick={() => (menuOpen = !menuOpen)}
      class="flex h-12 w-12 items-center justify-center rounded-2xl bg-gradient-to-br from-teal to-teal-soft font-display font-semibold text-linen ring-1 ring-line transition-all duration-200 hover:rounded-xl hover:ring-2 hover:ring-teal-bright"
    >
      {initial}
    </button>
  </div>
</nav>
