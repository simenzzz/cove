import { writable } from 'svelte/store';
import { api } from '$lib/api/client';
import { recordKey } from '$lib/utils/record-id';

export interface Server {
  id: { id?: string; tb?: string } | string | null;
  name: string;
  description?: string | null;
  icon_url?: string | null;
  owner: { id?: string; tb?: string } | string | null;
  created_at?: string | null;
}

export const servers = writable<Map<string, Server>>(new Map());

export async function fetchServers(): Promise<void> {
  try {
    const data = await api.get<{ servers: Server[] }>('/api/servers');
    const map = new Map<string, Server>();
    for (const server of data.servers) {
      const id = recordKey(server.id);
      if (id) map.set(id, server);
    }
    servers.set(map);
  } catch (err) {
    console.error('Failed to fetch servers:', err);
  }
}

export async function addServer(server: Server): Promise<void> {
  servers.update((map) => {
    const id = recordKey(server.id);
    if (id) {
      return new Map(map).set(id, server);
    }
    return map;
  });
}
