import { writable, derived } from 'svelte/store';
import { api } from '$lib/api/client';

interface User {
  id: string;
  username: string;
  displayName: string;
}

interface ApiUser {
  id: string;
  username: string;
  display_name?: string;
  displayName?: string;
}

interface AuthState {
  accessToken: string | null;
  user: User | null;
  loading: boolean;
}

const initialState: AuthState = {
  accessToken: null,
  user: null,
  loading: true,
};

export const auth = writable<AuthState>(initialState);

export const isAuthenticated = derived(auth, ($auth) => {
  return $auth.accessToken !== null && $auth.user !== null;
});

function normalizeUser(user: ApiUser): User {
  return {
    id: user.id,
    username: user.username,
    displayName: user.displayName ?? user.display_name ?? user.username,
  };
}

export async function login(username: string, password: string): Promise<void> {
  const data = await api.post<{ access_token: string; user: ApiUser }>(
    '/api/auth/login',
    { username, password },
  );
  api.setToken(data.access_token);
  auth.set({ accessToken: data.access_token, user: normalizeUser(data.user), loading: false });
}

export async function register(
  username: string,
  displayName: string,
  password: string,
): Promise<void> {
  const data = await api.post<{ access_token: string; user: ApiUser }>(
    '/api/auth/register',
    { username, display_name: displayName, password },
  );
  api.setToken(data.access_token);
  auth.set({ accessToken: data.access_token, user: normalizeUser(data.user), loading: false });
}

export async function logout(): Promise<void> {
  try {
    await api.post('/api/auth/logout');
  } catch {
    // Ignore errors on logout
  }
  api.setToken(null);
  auth.set({ accessToken: null, user: null, loading: false });
}

export async function silentRefresh(): Promise<void> {
  const success = await api.silentRefresh();
  if (success && api.getToken()) {
    try {
      const data = await api.get<{ user: ApiUser }>('/api/auth/me');
      auth.set({ accessToken: api.getToken(), user: normalizeUser(data.user), loading: false });
    } catch {
      api.setToken(null);
      auth.set({ accessToken: null, user: null, loading: false });
    }
  } else {
    auth.set({ accessToken: null, user: null, loading: false });
  }
}
