import { describe, it, expect, beforeEach, vi, type Mock } from 'vitest';
import { get } from 'svelte/store';

vi.mock('$lib/api/client', () => ({
  api: {
    get: vi.fn(),
    post: vi.fn(),
    setToken: vi.fn(),
    getToken: vi.fn(),
    silentRefresh: vi.fn(),
  },
}));

import { api } from '$lib/api/client';
import { auth, isAuthenticated, login, register, logout, silentRefresh } from './auth';

const post = api.post as Mock;
const getReq = api.get as Mock;
const setToken = api.setToken as Mock;
const getToken = api.getToken as Mock;
const refresh = api.silentRefresh as Mock;

beforeEach(() => {
  auth.set({ accessToken: null, user: null, loading: true });
});

describe('login', () => {
  it('stores the token and normalized user on success', async () => {
    post.mockResolvedValueOnce({
      access_token: 'tok',
      user: { id: 'u1', email: 'alice@example.com', username: 'alice', display_name: 'Alice' },
    });
    await login('alice@example.com', 'pw');
    expect(setToken).toHaveBeenCalledWith('tok');
    const state = get(auth);
    expect(state.accessToken).toBe('tok');
    expect(state.user).toEqual({ id: 'u1', email: 'alice@example.com', username: 'alice', displayName: 'Alice' });
    expect(state.loading).toBe(false);
  });

  it('falls back to username when no display name is present', async () => {
    post.mockResolvedValueOnce({ access_token: 't', user: { id: 'u', email: 'bob@example.com', username: 'bob' } });
    await login('bob@example.com', 'pw');
    expect(get(auth).user?.displayName).toBe('bob');
  });

  it('propagates errors from the API', async () => {
    post.mockRejectedValueOnce(new Error('bad creds'));
    await expect(login('x@example.com', 'y')).rejects.toThrow('bad creds');
  });
});

describe('register', () => {
  it('sends email, snake_case display_name and stores the session', async () => {
    post.mockResolvedValueOnce({
      access_token: 'tok',
      user: { id: 'u2', email: 'carol@example.com', username: 'carol', display_name: 'Carol' },
    });
    await register('carol@example.com', 'carol', 'Carol', 'pw');
    expect(post).toHaveBeenCalledWith('/api/auth/register', {
      email: 'carol@example.com',
      username: 'carol',
      display_name: 'Carol',
      password: 'pw',
    });
    expect(get(auth).user?.displayName).toBe('Carol');
  });
});

describe('logout', () => {
  it('clears token and session even when the API call fails', async () => {
    post.mockRejectedValueOnce(new Error('network'));
    auth.set({ accessToken: 'tok', user: { id: 'u', email: 'a@example.com', username: 'a', displayName: 'A' }, loading: false });
    await logout();
    expect(setToken).toHaveBeenCalledWith(null);
    expect(get(auth)).toEqual({ accessToken: null, user: null, loading: false });
  });
});

describe('silentRefresh', () => {
  it('hydrates the session when refresh + /me succeed', async () => {
    refresh.mockResolvedValueOnce(true);
    getToken.mockReturnValue('tok');
    getReq.mockResolvedValueOnce({ user: { id: 'u', email: 'a@example.com', username: 'a', display_name: 'A' } });
    await silentRefresh();
    expect(get(auth).user?.id).toBe('u');
    expect(get(auth).accessToken).toBe('tok');
  });

  it('clears the session when refresh fails', async () => {
    refresh.mockResolvedValueOnce(false);
    await silentRefresh();
    expect(get(auth)).toMatchObject({ accessToken: null, user: null, loading: false });
  });

  it('clears the session when /me throws after a successful refresh', async () => {
    refresh.mockResolvedValueOnce(true);
    getToken.mockReturnValue('tok');
    getReq.mockRejectedValueOnce(new Error('401'));
    await silentRefresh();
    expect(setToken).toHaveBeenCalledWith(null);
    expect(get(auth).user).toBeNull();
  });
});

describe('isAuthenticated derived store', () => {
  it('is false unless both token and user are present', () => {
    auth.set({ accessToken: null, user: null, loading: false });
    expect(get(isAuthenticated)).toBe(false);
    auth.set({ accessToken: 'tok', user: null, loading: false });
    expect(get(isAuthenticated)).toBe(false);
    auth.set({ accessToken: 'tok', user: { id: 'u', email: 'a@example.com', username: 'a', displayName: 'A' }, loading: false });
    expect(get(isAuthenticated)).toBe(true);
  });
});
