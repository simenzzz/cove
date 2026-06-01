import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { get } from 'svelte/store';

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

import {
  watchRooms,
  watchRoomStore,
  bindWatchRoom,
  sendQueueAdd,
  sendQueueRemove,
  sendVote,
  parseYouTubeId,
} from './watch';

function emit(type: string, msg: Record<string, unknown>) {
  for (const h of ws.handlers.get(type) ?? []) h(msg);
}

const CH = 'c1';
let unbind: () => void;

beforeEach(() => {
  ws.handlers.clear();
  watchRooms.set({});
  unbind = bindWatchRoom(CH);
});

afterEach(() => {
  unbind();
});

describe('bindWatchRoom lifecycle', () => {
  it('sends a watch_subscribe on bind', () => {
    expect(ws.send).toHaveBeenCalledWith({ v: 1, type: 'watch_subscribe', channel_id: CH });
  });

  it('sends watch_unsubscribe and clears room state on unbind', () => {
    emit('watch_state', { channel_id: CH, leader_id: 'u1', queue: [], viewers: [] });
    unbind();
    expect(ws.send).toHaveBeenCalledWith({ v: 1, type: 'watch_unsubscribe', channel_id: CH });
    expect(get(watchRooms)[CH]).toBeUndefined();
    unbind = () => {}; // afterEach no-op
  });

  it('ignores messages addressed to a different channel', () => {
    emit('watch_state', { channel_id: 'other', leader_id: 'x', queue: [], viewers: [] });
    expect(get(watchRooms)[CH]).toBeUndefined();
  });
});

describe('watch_state / playback handlers', () => {
  it('hydrates leader, queue and viewers from watch_state', () => {
    emit('watch_state', {
      channel_id: CH,
      leader_id: 'u1',
      queue: [{ id: 'q1', video_id: 'v', title: 't', duration_ms: 1, score: 0, added_by: 'u1' }],
      viewers: [{ user_id: 'u1', username: 'a', is_leader: true }],
    });
    const room = get(watchRoomStore(CH))!;
    expect(room.leader_id).toBe('u1');
    expect(room.queue).toHaveLength(1);
    expect(room.viewers).toHaveLength(1);
  });

  it('watch_playback pause sets paused true', () => {
    emit('watch_playback', { channel_id: CH, action: 'pause', position_ms: 100, server_ts: 5 });
    expect(get(watchRoomStore(CH))!.playback.paused).toBe(true);
  });

  it('watch_playback play sets paused false', () => {
    emit('watch_playback', { channel_id: CH, action: 'pause', position_ms: 0, server_ts: 1 });
    emit('watch_playback', { channel_id: CH, action: 'play', position_ms: 200, server_ts: 6 });
    const pb = get(watchRoomStore(CH))!.playback;
    expect(pb.paused).toBe(false);
    expect(pb.position_ms).toBe(200);
  });

  it('watch_leader_changed re-flags the viewers', () => {
    emit('watch_state', {
      channel_id: CH,
      leader_id: 'u1',
      queue: [],
      viewers: [
        { user_id: 'u1', username: 'a', is_leader: true },
        { user_id: 'u2', username: 'b', is_leader: false },
      ],
    });
    emit('watch_leader_changed', { channel_id: CH, leader_id: 'u2' });
    const room = get(watchRoomStore(CH))!;
    expect(room.leader_id).toBe('u2');
    expect(room.viewers.find((v) => v.user_id === 'u2')!.is_leader).toBe(true);
    expect(room.viewers.find((v) => v.user_id === 'u1')!.is_leader).toBe(false);
  });
});

describe('optimistic queue add + reconciliation', () => {
  it('sendQueueAdd inserts a pending entry and sends the add', () => {
    sendQueueAdd(CH, { video_id: 'vid12345678', title: 'T', duration_ms: 1000 });
    const room = get(watchRoomStore(CH))!;
    expect(room.queue).toHaveLength(1);
    expect(room.queue[0].pending).toBe(true);
    expect(ws.send).toHaveBeenCalledWith(
      expect.objectContaining({ type: 'watch_queue_add', channel_id: CH, video_id: 'vid12345678' }),
    );
  });

  it('watch_queue_ack drops the pending optimistic entry by nonce', () => {
    sendQueueAdd(CH, { video_id: 'vid12345678', title: 'T', duration_ms: 1000 });
    const nonce = get(watchRoomStore(CH))!.queue[0].nonce!;
    emit('watch_queue_ack', { channel_id: CH, nonce });
    expect(get(watchRoomStore(CH))!.queue).toHaveLength(0);
  });

  it('watch_queue_update preserves local my_vote across server re-broadcasts', () => {
    // `my_vote` is client-local optimistic state — the server never echoes it.
    // Establish it the real way (sendVote), then verify a server re-broadcast
    // (which carries no my_vote) does not clear the local highlight.
    emit('watch_queue_update', {
      channel_id: CH,
      queue: [{ id: 'q1', video_id: 'v', title: 't', duration_ms: 1, score: 3, added_by: 'u1' }],
    });
    sendVote(CH, 'q1', 1); // local vote -> my_vote = 1, score 4
    emit('watch_queue_update', {
      channel_id: CH,
      queue: [{ id: 'q1', video_id: 'v', title: 't', duration_ms: 1, score: 4, added_by: 'u1' }],
    });
    const stored = get(watchRoomStore(CH))!.queue.find((q) => q.id === 'q1')!;
    expect(stored.score).toBe(4);
    expect(stored.my_vote).toBe(1);
  });
});

describe('sendVote optimism', () => {
  it('applies the vote delta and re-sorts by score', () => {
    emit('watch_queue_update', {
      channel_id: CH,
      queue: [
        { id: 'q1', video_id: 'a', title: 'a', duration_ms: 1, score: 0, added_by: 'u1' },
        { id: 'q2', video_id: 'b', title: 'b', duration_ms: 1, score: 0, added_by: 'u1' },
      ],
    });
    sendVote(CH, 'q2', 1);
    const queue = get(watchRoomStore(CH))!.queue;
    expect(queue[0].id).toBe('q2');
    expect(queue[0].score).toBe(1);
    expect(queue[0].my_vote).toBe(1);
  });

  it('switching a vote applies the correct delta', () => {
    emit('watch_queue_update', {
      channel_id: CH,
      queue: [{ id: 'q1', video_id: 'a', title: 'a', duration_ms: 1, score: 0, added_by: 'u1' }],
    });
    sendVote(CH, 'q1', 1); // 0 -> +1 (my_vote 1, score 1)
    sendVote(CH, 'q1', -1); // delta = -1 - 1 = -2 -> score -1
    expect(get(watchRoomStore(CH))!.queue[0].score).toBe(-1);
    expect(get(watchRoomStore(CH))!.queue[0].my_vote).toBe(-1);
  });
});

describe('sendQueueRemove', () => {
  it('removes the item locally and sends a remove', () => {
    emit('watch_queue_update', {
      channel_id: CH,
      queue: [{ id: 'q1', video_id: 'a', title: 'a', duration_ms: 1, score: 0, added_by: 'u1' }],
    });
    sendQueueRemove(CH, 'q1');
    expect(get(watchRoomStore(CH))!.queue).toHaveLength(0);
    expect(ws.send).toHaveBeenCalledWith({ v: 1, type: 'watch_queue_remove', channel_id: CH, item_id: 'q1' });
  });
});

describe('watch_reaction handler', () => {
  it('appends a reaction and prunes it after the TTL', () => {
    vi.useFakeTimers();
    emit('watch_reaction', { channel_id: CH, user_id: 'u1', username: 'a', emoji: '🎉', ts: 123 });
    expect(get(watchRoomStore(CH))!.reactions).toHaveLength(1);
    vi.advanceTimersByTime(3000);
    expect(get(watchRoomStore(CH))!.reactions).toHaveLength(0);
    vi.useRealTimers();
  });
});

describe('watch_error handler', () => {
  it('records a formatted error string', () => {
    emit('watch_error', { channel_id: CH, code: 'not_leader', message: 'only the leader' });
    expect(get(watchRoomStore(CH))!.error).toBe('not_leader: only the leader');
  });
});

describe('parseYouTubeId', () => {
  it('accepts a raw 11-char id', () => {
    expect(parseYouTubeId('dQw4w9WgXcQ')).toBe('dQw4w9WgXcQ');
  });

  it('parses a youtu.be short link', () => {
    expect(parseYouTubeId('https://youtu.be/dQw4w9WgXcQ')).toBe('dQw4w9WgXcQ');
  });

  it('parses a youtube.com watch link', () => {
    expect(parseYouTubeId('https://www.youtube.com/watch?v=dQw4w9WgXcQ')).toBe('dQw4w9WgXcQ');
  });

  it('returns null for unrecognized input', () => {
    expect(parseYouTubeId('not a url')).toBeNull();
    expect(parseYouTubeId('https://example.com/watch?v=abc')).toBeNull();
  });
});
