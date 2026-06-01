import { describe, it, expect, beforeEach, vi, type Mock } from 'vitest';
import { get } from 'svelte/store';

vi.mock('$lib/api/client', () => ({
  api: { get: vi.fn(), post: vi.fn() },
}));

import { api } from '$lib/api/client';
import { servers, fetchServers, addServer, type Server } from './servers';

const getReq = api.get as Mock;

function server(id: unknown, name = 'srv'): Server {
  return { id: id as Server['id'], name, owner: 'user:o' };
}

beforeEach(() => {
  servers.set(new Map());
});

describe('addServer', () => {
  it('keys the server by its normalized record id', async () => {
    await addServer(server('server:abc', 'Cove'));
    expect(get(servers).get('abc')?.name).toBe('Cove');
  });

  it('ignores a server whose id cannot be resolved', async () => {
    await addServer(server(null));
    expect(get(servers).size).toBe(0);
  });

  it('overwrites an existing entry with the same id', async () => {
    await addServer(server('server:abc', 'old'));
    await addServer(server('server:abc', 'new'));
    expect(get(servers).size).toBe(1);
    expect(get(servers).get('abc')?.name).toBe('new');
  });
});

describe('fetchServers', () => {
  it('builds a map keyed by record id', async () => {
    getReq.mockResolvedValueOnce({
      servers: [server('server:a', 'A'), server('server:b', 'B')],
    });
    await fetchServers();
    expect([...get(servers).keys()].sort()).toEqual(['a', 'b']);
  });

  it('swallows API errors and leaves the store untouched', async () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    await addServer(server('server:existing', 'keep'));
    getReq.mockRejectedValueOnce(new Error('boom'));
    await fetchServers();
    expect(get(servers).get('existing')?.name).toBe('keep');
  });
});
