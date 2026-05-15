<script lang="ts">
  import { servers } from '$stores/servers';

  function getServerId(server: { id: unknown }): string {
    if (!server.id) return '';
    if (typeof server.id === 'string') return server.id;
    const obj = server.id as Record<string, unknown>;
    if (obj.id && typeof obj.id === 'string') return obj.id;
    if (obj.id && typeof obj.id === 'object') {
      return String(Object.values(obj.id as Record<string, unknown>)[0] ?? '');
    }
    return '';
  }
</script>

<nav class="w-[72px] bg-gray-900 border-r border-gray-800 flex flex-col items-center py-3 gap-2 shrink-0">
  <!-- Home / Feed -->
  <a
    href="/feed"
    class="w-12 h-12 bg-indigo-600 rounded-2xl flex items-center justify-center hover:rounded-xl transition-all font-bold"
  >
    N
  </a>
  <div class="w-8 h-0.5 bg-gray-700 rounded-full"></div>

  <!-- Server icons -->
  {#each Array.from($servers.values()) as server (getServerId(server))}
    {@const id = getServerId(server)}
    <a
      href="/servers/{id}/channels"
      class="w-12 h-12 bg-gray-700 rounded-2xl flex items-center justify-center hover:rounded-xl hover:bg-indigo-600 transition-all text-sm font-semibold"
      title={server.name}
    >
      {server.name.charAt(0).toUpperCase()}
    </a>
  {/each}

  <!-- Add server -->
  <a
    href="/explore"
    class="w-12 h-12 bg-gray-700 rounded-2xl flex items-center justify-center hover:rounded-xl hover:bg-green-600 transition-all text-green-400 hover:text-white text-xl"
  >
    +
  </a>
</nav>
