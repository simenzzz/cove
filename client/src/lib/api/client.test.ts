import { afterEach, describe, expect, it, vi } from 'vitest';

import { api } from './client';

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

describe('api client auth errors', () => {
  afterEach(() => {
    api.setToken(null);
    vi.unstubAllGlobals();
  });

  it('preserves invalid credential errors from login instead of refreshing', async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(jsonResponse(401, { error: 'Invalid credentials' }));
    vi.stubGlobal('fetch', fetchMock);

    await expect(api.post('/api/auth/login', { email: 'a@example.com', password: 'bad' })).rejects.toThrow(
      'Invalid credentials',
    );

    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(fetchMock.mock.calls[0][0]).toBe('http://localhost:3001/api/auth/login');
  });

  it('retries protected requests after a successful refresh', async () => {
    api.setToken('expired');
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(jsonResponse(401, { error: 'Invalid token' }))
      .mockResolvedValueOnce(jsonResponse(200, { access_token: 'fresh' }))
      .mockResolvedValueOnce(jsonResponse(200, { servers: [] }));
    vi.stubGlobal('fetch', fetchMock);

    await expect(api.get('/api/servers')).resolves.toEqual({ servers: [] });

    expect(fetchMock).toHaveBeenCalledTimes(3);
    expect(fetchMock.mock.calls[1][0]).toBe('http://localhost:3001/api/auth/refresh');
    expect(fetchMock.mock.calls[2][1].headers.Authorization).toBe('Bearer fresh');
  });

  it('reports session expiration when a protected request cannot refresh', async () => {
    api.setToken('expired');
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(jsonResponse(401, { error: 'Invalid token' }))
      .mockResolvedValueOnce(jsonResponse(401, { error: 'Invalid refresh token' }));
    vi.stubGlobal('fetch', fetchMock);

    await expect(api.get('/api/servers')).rejects.toThrow('Session expired');

    expect(fetchMock).toHaveBeenCalledTimes(2);
    expect(fetchMock.mock.calls[1][0]).toBe('http://localhost:3001/api/auth/refresh');
  });
});
