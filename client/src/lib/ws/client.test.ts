import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

// The ticket fetch is mocked so connect() never touches the network.
vi.mock('$lib/api/client', () => ({
  api: { post: vi.fn().mockResolvedValue({ ticket: 'tkt', nonce: 'nce' }) },
}));

/** Minimal scriptable WebSocket stand-in. */
class MockWebSocket {
  static CONNECTING = 0;
  static OPEN = 1;
  static CLOSING = 2;
  static CLOSED = 3;
  static instances: MockWebSocket[] = [];

  url: string;
  readyState = MockWebSocket.CONNECTING;
  sent: string[] = [];
  onopen: (() => void) | null = null;
  onmessage: ((e: { data: string }) => void) | null = null;
  onclose: (() => void) | null = null;
  onerror: (() => void) | null = null;

  constructor(url: string) {
    this.url = url;
    MockWebSocket.instances.push(this);
  }

  send(data: string) {
    this.sent.push(data);
  }

  close() {
    this.readyState = MockWebSocket.CLOSED;
    this.onclose?.();
  }

  // --- test helpers ---
  _open() {
    this.readyState = MockWebSocket.OPEN;
    this.onopen?.();
  }

  _message(obj: unknown) {
    this.onmessage?.({ data: JSON.stringify(obj) });
  }

  sentTypes(): string[] {
    return this.sent.map((s) => JSON.parse(s).type as string);
  }
}

type WsClient = typeof import('./client').wsClient;
let wsClient: WsClient;

beforeEach(async () => {
  MockWebSocket.instances = [];
  vi.stubGlobal('WebSocket', MockWebSocket);
  vi.spyOn(console, 'log').mockImplementation(() => {});
  vi.spyOn(console, 'error').mockImplementation(() => {});
  vi.resetModules();
  ({ wsClient } = await import('./client'));
});

afterEach(() => {
  wsClient.disconnect();
  vi.unstubAllGlobals();
  vi.useRealTimers();
});

async function connectAndAuth(heartbeat = 30_000): Promise<MockWebSocket> {
  const p = wsClient.connect();
  await vi.waitFor(() => expect(MockWebSocket.instances.length).toBe(1));
  const sock = MockWebSocket.instances[0];
  sock._open();
  sock._message({ type: 'auth_ok', heartbeat_interval: heartbeat });
  await p;
  return sock;
}

describe('connect + authenticate', () => {
  it('sends the auth frame with the fetched ticket on open', async () => {
    const sock = await connectAndAuth();
    const auth = sock.sent.map((s) => JSON.parse(s)).find((m) => m.type === 'auth');
    expect(auth).toMatchObject({ v: 1, type: 'auth', ticket: 'tkt', nonce: 'nce' });
  });

  it('flushes messages queued before auth_ok', async () => {
    const p = wsClient.connect();
    await vi.waitFor(() => expect(MockWebSocket.instances.length).toBe(1));
    const sock = MockWebSocket.instances[0];
    wsClient.send({ v: 1, type: 'subscribe', channel_id: 'c1' });
    expect(sock.sentTypes()).not.toContain('subscribe'); // still queued
    sock._open();
    sock._message({ type: 'auth_ok', heartbeat_interval: 0 });
    await p;
    expect(sock.sentTypes()).toContain('subscribe');
  });
});

describe('message dispatch', () => {
  it('routes messages to typed and wildcard handlers', async () => {
    const sock = await connectAndAuth();
    const typed: unknown[] = [];
    const wild: unknown[] = [];
    const offTyped = wsClient.on('chat_message', (m) => typed.push(m));
    const offWild = wsClient.on('*', (m) => wild.push(m));

    sock._message({ type: 'chat_message', content: 'hi' });
    expect(typed).toHaveLength(1);
    expect(wild).toHaveLength(1);

    offTyped();
    offWild();
    sock._message({ type: 'chat_message', content: 'again' });
    expect(typed).toHaveLength(1); // handler removed
  });

  it('ignores heartbeat_ack and unparseable frames without throwing', async () => {
    const sock = await connectAndAuth();
    const seen: unknown[] = [];
    wsClient.on('*', (m) => seen.push(m));
    sock._message({ type: 'heartbeat_ack' });
    sock.onmessage?.({ data: 'not json{' });
    expect(seen).toHaveLength(0);
  });
});

describe('send gating', () => {
  it('queues non-auth messages until the connection is ready', async () => {
    const p = wsClient.connect();
    await vi.waitFor(() => expect(MockWebSocket.instances.length).toBe(1));
    const sock = MockWebSocket.instances[0];
    wsClient.send({ v: 1, type: 'typing', channel_id: 'c1' });
    expect(sock.sent).toHaveLength(0);
    sock._open();
    sock._message({ type: 'auth_ok', heartbeat_interval: 0 });
    await p;
    expect(sock.sentTypes()).toContain('typing');
  });
});

describe('heartbeat', () => {
  it('sends heartbeats on the server-specified interval', async () => {
    vi.useFakeTimers();
    const p = wsClient.connect();
    await vi.advanceTimersByTimeAsync(0);
    const sock = MockWebSocket.instances[0];
    sock._open();
    sock._message({ type: 'auth_ok', heartbeat_interval: 1000 });
    await p;
    sock.sent.length = 0;
    await vi.advanceTimersByTimeAsync(1000);
    expect(sock.sentTypes()).toContain('heartbeat');
  });
});

describe('reconnection', () => {
  it('reconnects with backoff after an unexpected close', async () => {
    vi.useFakeTimers();
    const p = wsClient.connect();
    await vi.advanceTimersByTimeAsync(0);
    const sock = MockWebSocket.instances[0];
    sock._open();
    sock._message({ type: 'auth_ok', heartbeat_interval: 30_000 });
    await p;

    sock.close(); // unexpected drop -> schedules reconnect at 1000ms
    await vi.advanceTimersByTimeAsync(1000);
    expect(MockWebSocket.instances.length).toBe(2);
  });

  it('does not reconnect after an explicit disconnect()', async () => {
    vi.useFakeTimers();
    const p = wsClient.connect();
    await vi.advanceTimersByTimeAsync(0);
    const sock = MockWebSocket.instances[0];
    sock._open();
    sock._message({ type: 'auth_ok', heartbeat_interval: 30_000 });
    await p;

    wsClient.disconnect();
    await vi.advanceTimersByTimeAsync(5000);
    expect(MockWebSocket.instances.length).toBe(1);
  });
});
