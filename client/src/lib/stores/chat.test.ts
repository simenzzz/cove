import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import {
  chat,
  activeChannelMessages,
  addOptimisticMessage,
  confirmMessage,
  addReceivedMessage,
  setActiveChannel,
  getLastSeqPerChannel,
  type ChatMessage,
} from './chat';

function msg(overrides: Partial<ChatMessage> = {}): ChatMessage {
  return {
    id: 'm1',
    content: 'hello',
    authorId: 'u1',
    channelId: 'c1',
    createdAt: '2026-01-01T00:00:00.000Z',
    status: 'sent',
    ...overrides,
  };
}

beforeEach(() => {
  chat.set({ channels: new Map(), activeChannelId: null });
});

describe('addOptimisticMessage', () => {
  it('appends a message with status forced to pending', () => {
    addOptimisticMessage('c1', msg({ id: 'temp', nonce: 'n1', status: 'sent' }));
    const channel = get(chat).channels.get('c1');
    expect(channel?.messages).toHaveLength(1);
    expect(channel?.messages[0].status).toBe('pending');
  });

  it('creates the channel bucket lazily', () => {
    expect(get(chat).channels.has('c9')).toBe(false);
    addOptimisticMessage('c9', msg({ channelId: 'c9' }));
    expect(get(chat).channels.get('c9')?.messages).toHaveLength(1);
  });

  it('does not mutate the previous state object (immutability)', () => {
    const before = get(chat);
    addOptimisticMessage('c1', msg());
    expect(get(chat)).not.toBe(before);
    expect(before.channels.size).toBe(0);
  });
});

describe('confirmMessage', () => {
  it('promotes the optimistic message to sent and assigns id + seq', () => {
    addOptimisticMessage('c1', msg({ id: 'temp', nonce: 'n1' }));
    confirmMessage('n1', 'server-id', 7);
    const m = get(chat).channels.get('c1')!.messages[0];
    expect(m.id).toBe('server-id');
    expect(m.seq).toBe(7);
    expect(m.status).toBe('sent');
  });

  it('advances the channel lastSeq to the confirmed seq', () => {
    addOptimisticMessage('c1', msg({ nonce: 'n1' }));
    confirmMessage('n1', 'server-id', 12);
    expect(get(chat).channels.get('c1')!.lastSeq).toBe(12);
  });

  it('is a no-op when the nonce is unknown', () => {
    addOptimisticMessage('c1', msg({ nonce: 'n1' }));
    confirmMessage('does-not-exist', 'x', 99);
    const m = get(chat).channels.get('c1')!.messages[0];
    expect(m.status).toBe('pending');
  });
});

describe('addReceivedMessage', () => {
  it('appends a received message and tracks lastSeq', () => {
    addReceivedMessage('c1', msg({ id: 'r1', seq: 3 }));
    const channel = get(chat).channels.get('c1')!;
    expect(channel.messages).toHaveLength(1);
    expect(channel.lastSeq).toBe(3);
  });

  it('dedupes by id', () => {
    addReceivedMessage('c1', msg({ id: 'r1', seq: 1 }));
    addReceivedMessage('c1', msg({ id: 'r1', seq: 1 }));
    expect(get(chat).channels.get('c1')!.messages).toHaveLength(1);
  });

  it('dedupes by nonce when present', () => {
    addReceivedMessage('c1', msg({ id: 'a', nonce: 'n1' }));
    addReceivedMessage('c1', msg({ id: 'b', nonce: 'n1' }));
    expect(get(chat).channels.get('c1')!.messages).toHaveLength(1);
  });

  it('caps a channel at 200 messages, keeping the newest', () => {
    for (let i = 0; i < 250; i++) {
      addReceivedMessage('c1', msg({ id: `r${i}`, seq: i }));
    }
    const messages = get(chat).channels.get('c1')!.messages;
    expect(messages).toHaveLength(200);
    expect(messages[0].id).toBe('r50');
    expect(messages.at(-1)!.id).toBe('r249');
  });

  it('keeps prior lastSeq when the incoming message has no seq', () => {
    addReceivedMessage('c1', msg({ id: 'r1', seq: 5 }));
    addReceivedMessage('c1', msg({ id: 'r2' }));
    expect(get(chat).channels.get('c1')!.lastSeq).toBe(5);
  });
});

describe('activeChannelMessages derived store', () => {
  it('returns [] when no active channel is set', () => {
    expect(get(activeChannelMessages)).toEqual([]);
  });

  it('reflects the active channel messages', () => {
    addReceivedMessage('c1', msg({ id: 'r1' }));
    setActiveChannel('c1');
    expect(get(activeChannelMessages)).toHaveLength(1);
  });

  it('returns [] for an active channel with no bucket', () => {
    setActiveChannel('ghost');
    expect(get(activeChannelMessages)).toEqual([]);
  });
});

describe('getLastSeqPerChannel', () => {
  it('collects only channels with a positive lastSeq', () => {
    addReceivedMessage('c1', msg({ id: 'r1', seq: 4 }));
    addReceivedMessage('c2', msg({ id: 'r2', channelId: 'c2' })); // no seq -> lastSeq 0
    expect(getLastSeqPerChannel()).toEqual({ c1: 4 });
  });

  it('returns {} when nothing has a seq', () => {
    expect(getLastSeqPerChannel()).toEqual({});
  });
});
