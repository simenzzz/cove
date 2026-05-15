const API_URL = import.meta.env.PUBLIC_API_URL ?? 'http://localhost:3001';

interface ApiError {
  error: string;
}

class ApiClient {
  private token: string | null = null;
  private refreshPromise: Promise<boolean> | null = null;
  private onTokenChange?: (token: string | null) => void;

  setToken(token: string | null): void {
    this.token = token;
    this.onTokenChange?.(token);
  }

  getToken(): string | null {
    return this.token;
  }

  setTokenChangeCallback(cb: (token: string | null) => void): void {
    this.onTokenChange = cb;
  }

  private async parseJsonSafe<T>(response: Response): Promise<T> {
    const text = await response.text();
    if (!text) return undefined as T;
    try {
      return JSON.parse(text);
    } catch {
      throw new Error(`Server error: ${response.status} ${response.statusText}`);
    }
  }

  private async request<T>(path: string, options: RequestInit = {}): Promise<T> {
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      ...((options.headers as Record<string, string>) ?? {}),
    };

    if (this.token) {
      headers['Authorization'] = `Bearer ${this.token}`;
    }

    const response = await fetch(`${API_URL}${path}`, {
      ...options,
      headers,
      credentials: 'include',
    });

    if (response.status === 401 && !path.includes('/api/auth/refresh')) {
      const refreshed = await this.silentRefresh();
      if (refreshed) {
        headers['Authorization'] = `Bearer ${this.token}`;
        const retry = await fetch(`${API_URL}${path}`, {
          ...options,
          headers,
          credentials: 'include',
        });
        if (!retry.ok) {
          const err = await this.parseJsonSafe<ApiError>(retry);
          throw new Error(err?.error ?? `Request failed: ${retry.status}`);
        }
        return this.parseJsonSafe<T>(retry);
      }
      throw new Error('Session expired');
    }

    if (!response.ok) {
      const err = await this.parseJsonSafe<ApiError>(response);
      throw new Error(err?.error ?? `Request failed: ${response.status}`);
    }

    return this.parseJsonSafe<T>(response);
  }

  async silentRefresh(): Promise<boolean> {
    if (this.refreshPromise) return this.refreshPromise;

    this.refreshPromise = (async () => {
      try {
        const res = await fetch(`${API_URL}/api/auth/refresh`, {
          method: 'POST',
          credentials: 'include',
          headers: { 'Content-Type': 'application/json' },
        });
        if (!res.ok) return false;
        const data = await res.json();
        this.setToken(data.access_token);
        return true;
      } catch {
        return false;
      } finally {
        this.refreshPromise = null;
      }
    })();

    return this.refreshPromise;
  }

  get<T>(path: string): Promise<T> {
    return this.request<T>(path);
  }

  post<T>(path: string, body?: unknown): Promise<T> {
    return this.request<T>(path, {
      method: 'POST',
      body: body ? JSON.stringify(body) : undefined,
    });
  }

  put<T>(path: string, body: unknown): Promise<T> {
    return this.request<T>(path, {
      method: 'PUT',
      body: JSON.stringify(body),
    });
  }

  delete<T>(path: string): Promise<T> {
    return this.request<T>(path, { method: 'DELETE' });
  }
}

export const api = new ApiClient();
