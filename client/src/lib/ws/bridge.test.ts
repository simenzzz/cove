import { describe, it, expect, beforeEach, vi } from 'vitest';
import { get } from 'svelte/store';

// Capture the handlers the bridge registers via wsClient.on so we can emit
// synthetic server messages at them and assert the resulting store mutations.
const ws = vi.hoisted(() => {
  const handlers = new Map<string, ((m: Record<string, unknown>) => void)[]>();
  return {
    handlers,
    on: (type: string, fn: (m: Record<string, unknown>) => void) => {
      const arr = handlers.get(type) ?? [];
      arr.push(fn);
      handlers.set(type, arr);
      return () => {
        handlers.set(type, (handlers.get(type) ?? []).filter((h) => h !== fn));
      };
    },
    onReady: () => () => {},
    send: vi.fn(),
  };
});

vi.mock('$ws/client', () => ({ wsClient: { on: ws.on, onReady: ws.onReady, send: ws.send } }));

import { initBridge } from './bridge';
import { chat, addOptimisticMessage } from '$stores/chat';
import { typing } from '$stores/typing';
import { presence } from '$stores/presence';
import { servers } from '$stores/servers';
import { channels } from '$stores/channels';
import { directMessages } from '$stores/direct-messages';
import { notifications } from '$stores/notifications';
import { auth } from '$stores/auth';

function emit(type: string, msg: Record<string, unknown>) {
  for (const h of ws.handlers.get(type) ?? []) h(msg);
}

let teardown: () => void;

beforeEach(() => {
  ws.handlers.clear();
  chat.set({ channels: new Map(), activeChannelId: null });
  typing.set(new Map());
  presence.set({ statuses: new Map() });
  servers.set(new Map());
  channels.set(new Map());
  directMessages.set(new Map());
  notifications.clear();
  auth.set({
    accessToken: 'token',
    user: { id: 'me', email: 'me@test.example.com', username: 'me', displayName: 'Me' },
    loading: false,
  });
  teardown = initBridge();
});

describe('chat_message handler', () => {
  it('maps a server chat_message into a received ChatMessage', () => {
    emit('chat_message', {
      message_id: 'm1',
      channel_id: 'c1',
      content: 'hi',
      author: { id: 'user:u1', username: 'alice', display_name: 'Alice', avatar_url: null },
      seq: 5,
      ts: 1735689600000,
    });
    const stored = get(chat).channels.get('c1')!.messages[0];
    expect(stored).toMatchObject({
      id: 'm1',
      content: 'hi',
      authorId: 'u1',
      authorUsername: 'alice',
      authorDisplayName: 'Alice',
      authorAvatarUrl: null,
      seq: 5,
      status: 'sent',
    });
    expect(stored.createdAt).toBe(new Date(1735689600000).toISOString());
  });

  it('tolerates a missing author object', () => {
    emit('chat_message', { message_id: 'm2', channel_id: 'c1', content: 'x', seq: 1, ts: 0 });
    const stored = get(chat).channels.get('c1')!.messages[0];
    expect(stored.authorId).toBe('');
    expect(stored.authorUsername).toBe('');
  });

  it('creates a notification for another user message', () => {
    channels.set(
      new Map([
        [
          's1',
          [{ id: 'channel:c1', name: 'general', channel_type: 'text', server: 'server:s1' }],
        ],
      ]),
    );
    emit('chat_message', {
      message_id: 'm3',
      channel_id: 'c1',
      content: 'hello',
      author: { id: 'user:u1', username: 'alice', display_name: 'Alice', avatar_url: null },
      seq: 6,
      ts: 1735689600000,
    });
    const note = get(notifications).items[0];
    expect(note).toMatchObject({
      kind: 'channel_message',
      title: 'New message in #general',
      href: '/servers/s1/channels/c1',
    });
  });

  it('does not notify for the current user message', () => {
    emit('chat_message', {
      message_id: 'm4',
      channel_id: 'c1',
      content: 'mine',
      author: { id: 'user:me', username: 'me', display_name: 'Me' },
      seq: 7,
      ts: 1735689600000,
    });
    expect(get(notifications).items).toHaveLength(0);
  });
});

describe('message_ack handler', () => {
  it('confirms a pending optimistic message', () => {
    addOptimisticMessage('c1', {
      id: 'temp',
      nonce: 'n1',
      content: 'hi',
      authorId: 'u1',
      channelId: 'c1',
      createdAt: '2026-01-01T00:00:00.000Z',
      status: 'pending',
    });
    emit('message_ack', { nonce: 'n1', message_id: 'server-id', seq: 9 });
    const m = get(chat).channels.get('c1')!.messages[0];
    expect(m.id).toBe('server-id');
    expect(m.status).toBe('sent');
    expect(m.seq).toBe(9);
  });
});

describe('typing handler', () => {
  it('records the typing user in the typing store', () => {
    emit('typing', { channel_id: 'c1', user_id: 'u1', username: 'alice' });
    expect(get(typing).get('c1')?.get('u1')?.username).toBe('alice');
  });
});

describe('presence handler', () => {
  it('updates the user status map', () => {
    emit('presence', { user_id: 'u1', status: 'online' });
    expect(get(presence).statuses.get('u1')).toBe('online');
  });

  it('overwrites an existing status', () => {
    emit('presence', { user_id: 'u1', status: 'online' });
    emit('presence', { user_id: 'u1', status: 'idle' });
    expect(get(presence).statuses.get('u1')).toBe('idle');
  });
});

describe('server_joined / channel_created handlers', () => {
  it('adds a joined server to the servers store', () => {
    emit('server_joined', { server: { id: 'server:s1', name: 'Cove', owner: 'user:o' } });
    expect(get(servers).get('s1')?.name).toBe('Cove');
    expect(get(notifications).items[0]).toMatchObject({
      kind: 'server_joined',
      href: '/servers/s1',
    });
  });

  it('ignores server_joined without a server payload', () => {
    emit('server_joined', {});
    expect(get(servers).size).toBe(0);
  });

  it('adds a created channel under its server', () => {
    servers.set(new Map([['s1', { id: 'server:s1', name: 'Cove', owner: 'user:o' }]]));
    emit('channel_created', {
      server_id: 's1',
      channel: { id: 'channel:c1', name: 'general', channel_type: 'text', server: 'server:s1' },
    });
    expect(get(channels).get('s1')).toHaveLength(1);
    expect(get(notifications).items[0]).toMatchObject({
      kind: 'channel_created',
      title: '#general was created',
      href: '/servers/s1/channels/c1',
    });
  });

  it('ignores channel_created without server_id or channel', () => {
    emit('channel_created', { channel: { id: 'channel:c1', name: 'x', channel_type: 'text', server: 's' } });
    emit('channel_created', { server_id: 's1' });
    expect(get(channels).size).toBe(0);
  });
});

describe('notification event handlers', () => {
  it('notifies for friend requests and accepted requests', () => {
    emit('friend_request_received', {
      from_user: { id: 'user:u2', username: 'sam', display_name: 'Sam' },
    });
    emit('friend_request_accepted', {
      user: { id: 'user:u3', username: 'lee', display_name: 'Lee' },
    });
    expect(get(notifications).items.map((item) => item.kind)).toEqual([
      'friend_accepted',
      'friend_request',
    ]);
    expect(get(notifications).items.every((item) => item.href === '/friends')).toBe(true);
  });

  it('notifies for owner-scoped member joins and leaves', () => {
    servers.set(new Map([['s1', { id: 'server:s1', name: 'Cove', owner: 'user:me' }]]));
    emit('server_member_joined', {
      server_id: 's1',
      user: { id: 'user:u2', username: 'sam', display_name: 'Sam' },
    });
    emit('server_member_left', {
      server_id: 's1',
      user: { id: 'user:u2', username: 'sam', display_name: 'Sam' },
    });
    expect(get(notifications).items.map((item) => item.kind)).toEqual([
      'server_member_left',
      'server_member_joined',
    ]);
  });
});

describe('dm_channel_updated handler', () => {
  it('upserts the DM and notifies when another user sends a message', () => {
    emit('dm_channel_updated', {
      dm: {
        channel: { id: 'channel:dm1', name: 'direct', channel_type: 'direct', server: null },
        friend: { id: 'user:u2', username: 'sam', display_name: 'Sam' },
      },
      last_message_preview: 'hello',
      from_user: { id: 'user:u2', username: 'sam', display_name: 'Sam' },
      ts: 1735689600000,
    });

    expect(get(directMessages).get('dm1')?.friend.username).toBe('sam');
    expect(get(notifications).items[0]).toMatchObject({
      kind: 'direct_message',
      title: 'Sam',
      body: 'hello',
      href: '/dms/dm1',
    });
  });

  it('does not notify for the current user direct-message echo', () => {
    emit('dm_channel_updated', {
      dm: {
        channel: { id: 'channel:dm1', name: 'direct', channel_type: 'direct', server: null },
        friend: { id: 'user:u2', username: 'sam', display_name: 'Sam' },
      },
      last_message_preview: 'mine',
      from_user: { id: 'user:me', username: 'me', display_name: 'Me' },
      ts: 1735689600000,
    });

    expect(get(directMessages).has('dm1')).toBe(true);
    expect(get(notifications).items).toHaveLength(0);
  });
});

describe('teardown', () => {
  it('unregisters every handler so later events are ignored', () => {
    teardown();
    emit('presence', { user_id: 'u1', status: 'online' });
    expect(get(presence).statuses.size).toBe(0);
  });
});
