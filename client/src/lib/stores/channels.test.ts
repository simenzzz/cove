import { describe, it, expect, beforeEach, vi, type Mock } from 'vitest';
import { get } from 'svelte/store';

vi.mock('$lib/api/client', () => ({
  api: { get: vi.fn(), post: vi.fn() },
}));

import { api } from '$lib/api/client';
import { channels, fetchChannels, createChannel, addChannel, type Channel } from './channels';

const getReq = api.get as Mock;
const post = api.post as Mock;

function channel(id: unknown, name = 'general'): Channel {
  return { id: id as Channel['id'], name, channel_type: 'text', server: 'server:s' };
}

beforeEach(() => {
  channels.set(new Map());
});

describe('addChannel', () => {
  it('adds a channel under its server', () => {
    addChannel('s1', channel('channel:a'));
    expect(get(channels).get('s1')).toHaveLength(1);
  });

  it('dedupes by normalized channel id (replaces, not duplicates)', () => {
    addChannel('s1', channel('channel:a', 'old'));
    addChannel('s1', channel('channel:a', 'renamed'));
    const list = get(channels).get('s1')!;
    expect(list).toHaveLength(1);
    expect(list[0].name).toBe('renamed');
  });

  it('keeps channels from different servers separate', () => {
    addChannel('s1', channel('channel:a'));
    addChannel('s2', channel('channel:b'));
    expect(get(channels).get('s1')).toHaveLength(1);
    expect(get(channels).get('s2')).toHaveLength(1);
  });
});

describe('fetchChannels', () => {
  it('stores the fetched list under the server id', async () => {
    getReq.mockResolvedValueOnce({ channels: [channel('channel:a'), channel('channel:b')] });
    await fetchChannels('s1');
    expect(get(channels).get('s1')).toHaveLength(2);
  });

  it('swallows errors without throwing', async () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    getReq.mockRejectedValueOnce(new Error('boom'));
    await expect(fetchChannels('s1')).resolves.toBeUndefined();
  });
});

describe('createChannel', () => {
  it('POSTs the new channel then refetches the list', async () => {
    post.mockResolvedValueOnce({ channel: channel('channel:new', 'new') });
    getReq.mockResolvedValueOnce({ channels: [channel('channel:new', 'new')] });
    const created = await createChannel('s1', 'new', 'voice');
    expect(post).toHaveBeenCalledWith('/api/servers/s1/channels', {
      name: 'new',
      channel_type: 'voice',
    });
    expect(created.name).toBe('new');
    expect(get(channels).get('s1')).toHaveLength(1);
  });
});
