import { describe, it, expect, beforeEach, vi, type Mock } from 'vitest';
import { get } from 'svelte/store';

vi.mock('$lib/api/client', () => ({
  api: { get: vi.fn(), post: vi.fn() },
}));

import { api } from '$lib/api/client';
import {
  directMessages,
  latestDmChannelId,
  dmTargets,
  dmId,
  dmLabel,
  fetchDms,
  openDm,
  upsertDm,
  type DirectMessageSummary,
} from './direct-messages';
import type { FriendUser } from './friends';

const getReq = api.get as Mock;
const post = api.post as Mock;

function user(id: string, name = 'Alice'): FriendUser {
  return { id: `user:${id}`, username: name.toLowerCase(), display_name: name };
}

function dm(id: string, name = 'Alice', friendId = 'u1'): DirectMessageSummary {
  return {
    channel: { id: `channel:${id}`, name: 'direct', channel_type: 'direct', server: null },
    friend: user(friendId, name),
  };
}

beforeEach(() => {
  directMessages.set(new Map());
  latestDmChannelId.set(null);
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

  it('tracks the first fetched DM as the latest channel', async () => {
    getReq.mockResolvedValueOnce({ dms: [dm('latest'), dm('older', 'Bob', 'u2')] });

    await fetchDms();

    expect(get(latestDmChannelId)).toBe('latest');
  });

  it('opens and stores a direct message', async () => {
    post.mockResolvedValueOnce({ dm: dm('c9') });
    const opened = await openDm('u1');
    expect(post).toHaveBeenCalledWith('/api/dms', { user_id: 'u1' });
    expect(dmId(opened)).toBe('c9');
    expect(get(directMessages).has('c9')).toBe(true);
    expect(get(latestDmChannelId)).toBe('c9');
  });

  it('upserts by channel id', () => {
    upsertDm(dm('c1', 'Old'));
    upsertDm(dm('c1', 'New'));
    expect(get(directMessages).size).toBe(1);
    expect(dmLabel(get(directMessages).get('c1')!)).toBe('New');
    expect(get(latestDmChannelId)).toBe('c1');
  });

  it('merges existing DMs with accepted friends as sidebar targets', () => {
    const targets = dmTargets([dm('c1', 'Alice', 'u1')], [user('u1', 'Alice'), user('u2', 'Bob')]);

    expect(targets.map((target) => target.label)).toEqual(['Alice', 'Bob']);
    expect(targets[0]).toMatchObject({ userId: 'u1', channelId: 'c1', hasDm: true });
    expect(targets[1]).toMatchObject({ userId: 'u2', channelId: null, hasDm: false });
  });

  it('does not duplicate accepted friends that already have DMs', () => {
    const targets = dmTargets([dm('c1', 'Alice', 'u1')], [user('u1', 'Alice')]);

    expect(targets).toHaveLength(1);
    expect(targets[0].hasDm).toBe(true);
  });
});
