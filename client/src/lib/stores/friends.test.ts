import { describe, it, expect, beforeEach, vi, type Mock } from 'vitest';
import { get } from 'svelte/store';

vi.mock('$lib/api/client', () => ({
  api: { get: vi.fn(), post: vi.fn(), delete: vi.fn() },
}));

import { api } from '$lib/api/client';
import {
  friends,
  friendId,
  fetchFriends,
  sendRequest,
  acceptRequest,
  declineRequest,
  removeFriend,
  type FriendUser,
} from './friends';

const getReq = api.get as Mock;
const post = api.post as Mock;
const del = api.delete as Mock;

function user(id: string, name = 'u'): FriendUser {
  return { id: `user:${id}`, username: name, display_name: name };
}

beforeEach(() => {
  friends.set({ friends: [], pending: [], suggestions: [] });
});

describe('friendId', () => {
  it('normalizes the record id', () => {
    expect(friendId(user('abc'))).toBe('abc');
  });
});

describe('fetchFriends', () => {
  it('loads friends, pending and suggestions in parallel', async () => {
    getReq
      .mockResolvedValueOnce({ friends: [user('a')] })
      .mockResolvedValueOnce({ pending: [user('b')] })
      .mockResolvedValueOnce({ suggestions: [user('c')] });
    await fetchFriends();
    const state = get(friends);
    expect(state.friends).toHaveLength(1);
    expect(state.pending).toHaveLength(1);
    expect(state.suggestions).toHaveLength(1);
  });

  it('swallows errors and leaves state intact', async () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    friends.set({ friends: [user('keep')], pending: [], suggestions: [] });
    getReq.mockRejectedValue(new Error('boom'));
    await fetchFriends();
    expect(get(friends).friends).toHaveLength(1);
  });
});

describe('sendRequest', () => {
  it('POSTs the target user id', async () => {
    post.mockResolvedValueOnce(undefined);
    await sendRequest('u9');
    expect(post).toHaveBeenCalledWith('/api/friends/request', { user_id: 'u9' });
  });
});

describe('acceptRequest', () => {
  it('accepts then refetches the lists', async () => {
    post.mockResolvedValueOnce(undefined);
    getReq
      .mockResolvedValueOnce({ friends: [user('a')] })
      .mockResolvedValueOnce({ pending: [] })
      .mockResolvedValueOnce({ suggestions: [] });
    await acceptRequest('sender1');
    expect(post).toHaveBeenCalledWith('/api/friends/accept', { user_id: 'sender1' });
    expect(get(friends).friends).toHaveLength(1);
  });
});

describe('declineRequest', () => {
  it('deletes the edge and removes the user from pending locally', async () => {
    friends.set({ friends: [], pending: [user('s1'), user('s2')], suggestions: [] });
    del.mockResolvedValueOnce(undefined);
    await declineRequest('s1');
    expect(del).toHaveBeenCalledWith('/api/friends/s1');
    expect(get(friends).pending.map(friendId)).toEqual(['s2']);
  });
});

describe('removeFriend', () => {
  it('deletes the edge and removes the user from friends locally', async () => {
    friends.set({ friends: [user('f1'), user('f2')], pending: [], suggestions: [] });
    del.mockResolvedValueOnce(undefined);
    await removeFriend('f1');
    expect(get(friends).friends.map(friendId)).toEqual(['f2']);
  });
});
