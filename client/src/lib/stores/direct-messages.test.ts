import { describe, it, expect, beforeEach, vi, type Mock } from 'vitest';
import { get } from 'svelte/store';

vi.mock('$lib/api/client', () => ({
  api: { get: vi.fn(), post: vi.fn() },
}));

import { api } from '$lib/api/client';
import {
  directMessages,
  dmId,
  dmLabel,
  fetchDms,
  openDm,
  upsertDm,
  type DirectMessageSummary,
} from './direct-messages';

const getReq = api.get as Mock;
const post = api.post as Mock;

function dm(id: string, name = 'Alice'): DirectMessageSummary {
  return {
    channel: { id: `channel:${id}`, name: 'direct', channel_type: 'direct', server: null },
    friend: { id: 'user:u1', username: name.toLowerCase(), display_name: name },
  };
}

beforeEach(() => {
  directMessages.set(new Map());
  vi.clearAllMocks();
});

describe('direct message store', () => {
  it('normalizes ids and labels', () => {
    const item = dm('c1', 'Alice');
    expect(dmId(item)).toBe('c1');
    expect(dmLabel(item)).toBe('Alice');
  });

  it('fetches direct messages into a keyed map', async () => {
    getReq.mockResolvedValueOnce({ dms: [dm('c1'), dm('c2', 'Bob')] });
    await fetchDms();
    expect(Array.from(get(directMessages).keys())).toEqual(['c1', 'c2']);
  });

  it('opens and stores a direct message', async () => {
    post.mockResolvedValueOnce({ dm: dm('c9') });
    const opened = await openDm('u1');
    expect(post).toHaveBeenCalledWith('/api/dms', { user_id: 'u1' });
    expect(dmId(opened)).toBe('c9');
    expect(get(directMessages).has('c9')).toBe(true);
  });

  it('upserts by channel id', () => {
    upsertDm(dm('c1', 'Old'));
    upsertDm(dm('c1', 'New'));
    expect(get(directMessages).size).toBe(1);
    expect(dmLabel(get(directMessages).get('c1')!)).toBe('New');
  });
});
