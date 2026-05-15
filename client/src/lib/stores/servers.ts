import { writable, get } from 'svelte/store';
import { api } from '$lib/api/client';

export interface Server {
  id: { id?: string; tb?: string } | null;
  name: string;
  description?: string | null;
  icon_url?: string | null;
  owner: { id?: string; tb?: string } | null;
  created_at?: string | null;
}

function extractId(recordId: { id?: string; tb?: string } | null): string {
  if (!recordId) return '';
  // SurrealDB RecordId serialized as {tb: "server", id: {String: "abc"}}
  if (typeof recordId === 'string') return recordId;
  if (recordId.id && typeof recordId.id === 'string') return recordId.id;
  // Handle nested object format
  const inner = (recordId as Record<string, unknown>).id;
  if (inner && typeof inner === 'string') return inner;
  if (inner && typeof inner === 'object') {
    return String(Object.values(inner as Record<string, unknown>)[0] ?? '');
  }
  return String(recordId);
}

export const servers = writable<Map<string, Server>>(new Map());

export async function fetchServers(): Promise<void> {
  try {
    const data = await api.get<{ servers: Server[] }>('/api/servers');
    const map = new Map<string, Server>();
    for (const server of data.servers) {
      const id = extractId(server.id);
      if (id) map.set(id, server);
    }
    servers.set(map);
  } catch (err) {
    console.error('Failed to fetch servers:', err);
  }
}

export async function addServer(server: Server): Promise<void> {
  servers.update((map) => {
    const id = extractId(server.id);
    if (id) {
      return new Map(map).set(id, server);
    }
    return map;
  });
}
