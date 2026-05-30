<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api/client';
  import { servers, fetchServers, type Server } from '$stores/servers';
  import { addServer } from '$stores/servers';
  import { addChannel, type Channel } from '$stores/channels';
  import { recordKey } from '$lib/utils/record-id';
  import Button from '$components/ui/Button.svelte';
  import Input from '$components/ui/Input.svelte';
  import SectionHeading from '$components/ui/SectionHeading.svelte';
  import Skeleton from '$components/ui/Skeleton.svelte';
  import { Plus, X, Users, Sparkles } from '@lucide/svelte';

  interface DiscoveredServer {
    id: string;
    name: string;
    description: string;
    friendCount: number;
  }

  // ── Discovery ──
  let discovered = $state<DiscoveredServer[]>([]);
  let loadingDiscovery = $state(true);
  let discoveryError = $state<string | null>(null);
  let joiningId = $state<string | null>(null);

  const joinedIds = $derived(new Set($servers.keys()));

  async function loadDiscovery() {
    loadingDiscovery = true;
    discoveryError = null;
    try {
      const data = await api.get<{ servers: unknown[] }>('/api/discover/servers');
      discovered = normalize(data.servers ?? []);
    } catch (err) {
      discoveryError = err instanceof Error ? err.message : 'Could not load servers.';
    } finally {
      loadingDiscovery = false;
    }
  }

  // The discovery endpoint returns a uniform shape: full server documents each
  // carrying a `friend_count` (0 in the no-friends branch). Names are hydrated
  // server-side, so no per-server follow-up fetch is needed here.
  function normalize(raw: unknown[]): DiscoveredServer[] {
    return raw
      .map((item) => {
        const obj = item as Record<string, unknown>;
        return {
          id: recordKey(obj.id),
          name: typeof obj.name === 'string' ? obj.name : '',
          description: typeof obj.description === 'string' ? obj.description : '',
          friendCount: Number(obj.friend_count ?? 0),
        };
      })
      // Keep every entry with a valid id; fall back to a readable label when the
      // name couldn't be resolved, rather than silently dropping the server.
      .filter((e) => e.id)
      .map((e) => (e.name ? e : { ...e, name: 'Unnamed server' }));
  }

  async function joinServer(id: string) {
    joiningId = id;
    try {
      await api.post(`/api/servers/${id}/join`);
      await fetchServers();
      goto(`/servers/${id}`);
    } catch (err) {
      discoveryError = err instanceof Error ? err.message : 'Failed to join server.';
    } finally {
      joiningId = null;
    }
  }

  // ── Create ──
  let createOpen = $state(false);
  let newName = $state('');
  let newDescription = $state('');
  let creating = $state(false);
  let createError = $state<string | null>(null);

  async function createServer(e: Event) {
    e.preventDefault();
    createError = null;
    const name = newName.trim();
    if (name.length < 1 || name.length > 100) {
      createError = 'Server name must be 1–100 characters.';
      return;
    }
    creating = true;
    try {
      const { server, channel } = await api.post<{ server: Server; channel?: Channel }>('/api/servers', {
        name,
        description: newDescription.trim() || null,
      });
      const id = recordKey(server.id);
      await addServer(server);
      if (id && channel) addChannel(id, channel);
      newName = '';
      newDescription = '';
      createOpen = false;
      if (id) goto(`/servers/${id}`);
    } catch (err) {
      createError = err instanceof Error ? err.message : 'Failed to create server.';
    } finally {
      creating = false;
    }
  }

  onMount(loadDiscovery);
</script>

<div class="mx-auto max-w-2xl px-4 py-8">
  <SectionHeading
    eyebrow="Find your people"
    title="Explore"
    subtitle="Start your own server or join one your friends are already in."
  />

  <!-- Create a server -->
  <section class="mt-7 overflow-hidden rounded-2xl border border-line bg-surface">
    <button
      type="button"
      onclick={() => (createOpen = !createOpen)}
      aria-expanded={createOpen}
      class="flex w-full items-center justify-between px-5 py-4 text-left font-display font-semibold text-linen transition-colors hover:bg-elevated/50"
    >
      <span>Create a server</span>
      <span class="text-copper-bright">{#if createOpen}<X size={18} />{:else}<Plus size={18} />{/if}</span>
    </button>

    {#if createOpen}
      <form onsubmit={createServer} class="space-y-3 border-t border-line px-5 pb-5 pt-4">
        {#if createError}
          <p class="rounded-xl border border-danger/40 bg-danger-soft p-2.5 text-sm text-danger">
            {createError}
          </p>
        {/if}
        <Input
          bind:value={newName}
          aria-label="Server name"
          placeholder="Server name"
          maxlength={100}
          required
        />
        <Input
          bind:value={newDescription}
          aria-label="Server description"
          placeholder="Description (optional)"
        />
        <Button type="submit" loading={creating}>
          {creating ? 'Creating…' : 'Create server'}
        </Button>
      </form>
    {/if}
  </section>

  <!-- Discover servers -->
  <section class="mt-9">
    <div class="mb-3 flex items-center gap-2">
      <Sparkles size={16} class="text-teal-bright" />
      <h2 class="font-display text-lg font-semibold text-linen">Discover servers</h2>
    </div>

    {#if loadingDiscovery}
      <div class="space-y-2.5">
        {#each Array(3) as _}
          <Skeleton class="h-[68px] w-full" rounded="rounded-2xl" />
        {/each}
      </div>
    {:else if discoveryError}
      <div class="rounded-2xl border border-line bg-surface p-5 text-center">
        <p class="mb-3 text-sm text-danger">{discoveryError}</p>
        <Button variant="outline" size="sm" onclick={loadDiscovery}>Retry</Button>
      </div>
    {:else if discovered.length === 0}
      <p class="rounded-2xl border border-dashed border-line-strong bg-surface/40 px-5 py-10 text-center text-sm text-linen-muted">
        Nothing to discover yet. Add some friends to see their servers!
      </p>
    {:else}
      <ul class="space-y-2.5">
        {#each discovered as server (server.id)}
          {@const joined = joinedIds.has(server.id)}
          <li class="flex items-center gap-3.5 rounded-2xl border border-line bg-surface p-3.5">
            <div class="flex h-11 w-11 shrink-0 items-center justify-center rounded-2xl bg-gradient-to-br from-copper to-copper-deep font-display font-semibold text-canvas">
              {server.name.charAt(0).toUpperCase()}
            </div>
            <div class="min-w-0 flex-1">
              <p class="truncate font-semibold text-linen">{server.name}</p>
              {#if server.description}
                <p class="truncate text-sm text-linen-dim">{server.description}</p>
              {/if}
              <p class="mt-0.5 flex items-center gap-1 text-xs text-linen-muted">
                {#if server.friendCount > 0}
                  <Users size={12} />
                  {server.friendCount} friend{server.friendCount === 1 ? '' : 's'} inside
                {:else}
                  New to you
                {/if}
              </p>
            </div>
            {#if joined}
              <span class="shrink-0 px-3 py-1.5 text-sm text-linen-muted">Joined</span>
            {:else}
              <Button
                variant="secondary"
                size="sm"
                class="shrink-0"
                onclick={() => joinServer(server.id)}
                disabled={joiningId === server.id}
              >
                {joiningId === server.id ? 'Joining…' : 'Join'}
              </Button>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  </section>
</div>
