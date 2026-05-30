import { writable, derived } from 'svelte/store';
import { api } from '$lib/api/client';
import { wsClient, type WsMessage } from '$lib/ws/client';

export interface WatchRecommendation {
  video_id: string;
  score: number;
}

/// Fetch the graph-based "what your server-mates are watching" list for the
/// current user. Bounded by the REST handler's clamp; pass a small limit.
export async function fetchRecommendations(
  channelId: string,
  limit = 10,
): Promise<WatchRecommendation[]> {
  const data = await api.get<{ recommendations: WatchRecommendation[] }>(
    `/api/channels/${channelId}/watch/recommendations?limit=${limit}`,
  );
  return data.recommendations ?? [];
}

export interface WatchViewer {
  user_id: string;
  username: string;
  is_leader: boolean;
}

export interface WatchPlayback {
  video_id: string | null;
  position_ms: number;
  paused: boolean;
  server_ts: number;
  rate: number;
}

export interface WatchQueueItem {
  id: string;
  video_id: string;
  title: string;
  duration_ms: number;
  thumbnail_url: string | null;
  added_by: string;
  score: number;
  /// `pending` until the server's `watch_queue_ack` arrives for the matching
  /// nonce; then collapses into the canonical entry from `watch_queue_update`.
  pending?: boolean;
  /// Local id placeholder used for optimistic entries — replaced by the
  /// server-assigned id once the ack arrives.
  nonce?: string;
  /// Local optimistic vote, applied immediately. Reset by `watch_queue_update`.
  my_vote?: -1 | 0 | 1;
}

export interface WatchReactionEvent {
  /// Local id (timestamp + random) so we can key + remove without coordinating
  /// with the server. Reactions are fire-and-forget; the server doesn't track them.
  local_id: string;
  user_id: string;
  username: string;
  emoji: string;
  ts: number;
}

export interface WatchRoomState {
  channel_id: string;
  leader_id: string | null;
  playback: WatchPlayback;
  queue: WatchQueueItem[];
  viewers: WatchViewer[];
  /// Ephemeral floating reactions. Auto-pruned after `REACTION_TTL_MS`.
  reactions: WatchReactionEvent[];
  /// Most recent error message from the server (e.g. not_leader, rate_limited).
  error: string | null;
}

/// How long a reaction stays in the store (matches the CSS animation length).
export const REACTION_TTL_MS = 3000;

/// Hard cap on simultaneous floating reactions. Defends against unbounded
/// growth if a server-side rate-limit bug or replay storm floods us — the
/// store would otherwise leak DOM nodes proportional to message volume.
const REACTION_BUFFER_CAP = 50;

const initialPlayback: WatchPlayback = {
  video_id: null,
  position_ms: 0,
  paused: true,
  server_ts: Date.now(),
  rate: 1,
};

export const watchRooms = writable<Record<string, WatchRoomState>>({});

function update(channelId: string, patch: (s: WatchRoomState) => WatchRoomState): void {
  watchRooms.update((rooms) => {
    const existing = rooms[channelId] ?? {
      channel_id: channelId,
      leader_id: null,
      playback: { ...initialPlayback },
      queue: [],
      viewers: [],
      reactions: [],
      error: null,
    };
    return { ...rooms, [channelId]: patch(existing) };
  });
}

export function watchRoomStore(channelId: string) {
  return derived(watchRooms, ($r) => $r[channelId] ?? null);
}

/// Wire the WS handlers for a single watch channel. Returns an unsubscribe
/// that detaches every handler — call it on component teardown so we don't
/// leak across navigations.
export function bindWatchRoom(channelId: string): () => void {
  const offs: Array<() => void> = [];

  offs.push(
    wsClient.on('watch_state', (msg: WsMessage) => {
      if (msg.channel_id !== channelId) return;
      update(channelId, (s) => ({
        ...s,
        leader_id: (msg.leader_id as string | null) ?? null,
        playback: (msg.playback as WatchPlayback) ?? s.playback,
        queue: (msg.queue as WatchQueueItem[]) ?? [],
        viewers: (msg.viewers as WatchViewer[]) ?? [],
      }));
    }),
  );

  offs.push(
    wsClient.on('watch_playback', (msg: WsMessage) => {
      if (msg.channel_id !== channelId) return;
      const action = msg.action as 'play' | 'pause' | 'seek';
      update(channelId, (s) => ({
        ...s,
        playback: {
          ...s.playback,
          position_ms: msg.position_ms as number,
          server_ts: msg.server_ts as number,
          paused: action === 'pause' ? true : action === 'play' ? false : s.playback.paused,
        },
      }));
    }),
  );

  offs.push(
    wsClient.on('watch_sync_pulse', (msg: WsMessage) => {
      if (msg.channel_id !== channelId) return;
      update(channelId, (s) => ({
        ...s,
        playback: {
          ...s.playback,
          position_ms: msg.position_ms as number,
          server_ts: msg.server_ts as number,
          paused: msg.paused as boolean,
        },
      }));
    }),
  );

  offs.push(
    wsClient.on('watch_leader_changed', (msg: WsMessage) => {
      if (msg.channel_id !== channelId) return;
      const newLeader = msg.leader_id as string;
      update(channelId, (s) => ({
        ...s,
        leader_id: newLeader,
        viewers: s.viewers.map((v) => ({ ...v, is_leader: v.user_id === newLeader })),
      }));
    }),
  );

  offs.push(
    wsClient.on('watch_queue_update', (msg: WsMessage) => {
      if (msg.channel_id !== channelId) return;
      update(channelId, (s) => {
        const incoming = (msg.queue as WatchQueueItem[]) ?? [];
        // Preserve each user's local `my_vote` highlight across server
        // re-broadcasts — the server never echoes per-user vote state, so a
        // naive overwrite would clear the up/down arrow highlight whenever
        // any peer voted. Keyed by item id.
        const priorVotes = new Map(s.queue.map((q) => [q.id, q.my_vote]));
        const merged = incoming.map((q) => ({
          ...q,
          my_vote: priorVotes.get(q.id),
        }));
        // Keep any still-pending optimistic adds that the server hasn't yet
        // echoed (they have no server id and weren't matched by ack).
        const pending = s.queue.filter((q) => q.pending && q.nonce);
        return { ...s, queue: [...merged, ...pending] };
      });
    }),
  );

  offs.push(
    wsClient.on('watch_queue_ack', (msg: WsMessage) => {
      if (msg.channel_id !== channelId) return;
      const nonce = msg.nonce as string;
      update(channelId, (s) => ({
        ...s,
        queue: s.queue.filter((q) => q.nonce !== nonce),
      }));
    }),
  );

  offs.push(
    wsClient.on('watch_advance', (msg: WsMessage) => {
      if (msg.channel_id !== channelId) return;
      update(channelId, (s) => ({
        ...s,
        playback: (msg.playback as WatchPlayback) ?? s.playback,
        queue: (msg.queue as WatchQueueItem[]) ?? [],
      }));
    }),
  );

  offs.push(
    wsClient.on('watch_reaction', (msg: WsMessage) => {
      if (msg.channel_id !== channelId) return;
      const local_id = `${msg.ts as number}-${Math.random().toString(36).slice(2, 8)}`;
      const event: WatchReactionEvent = {
        local_id,
        user_id: msg.user_id as string,
        username: msg.username as string,
        emoji: msg.emoji as string,
        ts: msg.ts as number,
      };
      update(channelId, (s) => {
        const next = [...s.reactions, event];
        // Drop oldest if we exceed the cap — bounds memory under a flood.
        const trimmed = next.length > REACTION_BUFFER_CAP
          ? next.slice(next.length - REACTION_BUFFER_CAP)
          : next;
        return { ...s, reactions: trimmed };
      });
      // Auto-prune after the CSS animation has finished. Independent timer
      // per reaction keeps the cleanup simple — bounded by the rate limit.
      setTimeout(() => {
        update(channelId, (s) => ({
          ...s,
          reactions: s.reactions.filter((r) => r.local_id !== local_id),
        }));
      }, REACTION_TTL_MS);
    }),
  );

  offs.push(
    wsClient.on('watch_error', (msg: WsMessage) => {
      if (msg.channel_id !== channelId) return;
      update(channelId, (s) => ({
        ...s,
        error: `${msg.code as string}: ${msg.message as string}`,
      }));
    }),
  );

  wsClient.send({ v: 1, type: 'watch_subscribe', channel_id: channelId });
  offs.push(
    wsClient.onReady(() => {
      wsClient.send({ v: 1, type: 'watch_subscribe', channel_id: channelId });
    }),
  );

  return () => {
    wsClient.send({ v: 1, type: 'watch_unsubscribe', channel_id: channelId });
    for (const off of offs) off();
    watchRooms.update((rooms) => {
      const next = { ...rooms };
      delete next[channelId];
      return next;
    });
  };
}

/// Leader-only: send a playback control. `action` is "play" | "pause" | "seek".
export function sendPlayback(
  channelId: string,
  action: 'play' | 'pause' | 'seek',
  positionMs: number,
): void {
  wsClient.send({
    v: 1,
    type: 'watch_playback',
    channel_id: channelId,
    action,
    position_ms: Math.max(0, Math.round(positionMs)),
    client_ts: Date.now(),
  });
}

export function sendTransferLeader(channelId: string, toUserId: string): void {
  wsClient.send({
    v: 1,
    type: 'watch_transfer_leader',
    channel_id: channelId,
    to_user_id: toUserId,
  });
}

export interface QueueAddPayload {
  video_id: string;
  title: string;
  duration_ms: number;
  thumbnail_url?: string | null;
}

/// Optimistically insert a pending entry and send the add over WS.
/// `watch_queue_ack` will drop the pending entry; `watch_queue_update` will
/// re-add it with the server-assigned id and canonical ordering.
export function sendQueueAdd(channelId: string, payload: QueueAddPayload): void {
  const nonce = crypto.randomUUID();
  update(channelId, (s) => ({
    ...s,
    queue: [
      ...s.queue,
      {
        id: nonce,
        video_id: payload.video_id,
        title: payload.title,
        duration_ms: payload.duration_ms,
        thumbnail_url: payload.thumbnail_url ?? null,
        added_by: '',
        score: 0,
        pending: true,
        nonce,
      },
    ],
  }));
  wsClient.send({
    v: 1,
    type: 'watch_queue_add',
    channel_id: channelId,
    video_id: payload.video_id,
    title: payload.title,
    duration_ms: payload.duration_ms,
    thumbnail_url: payload.thumbnail_url ?? null,
    nonce,
  });
}

export function sendQueueRemove(channelId: string, itemId: string): void {
  update(channelId, (s) => ({
    ...s,
    queue: s.queue.filter((q) => q.id !== itemId),
  }));
  wsClient.send({
    v: 1,
    type: 'watch_queue_remove',
    channel_id: channelId,
    item_id: itemId,
  });
}

export function sendVote(channelId: string, itemId: string, value: -1 | 0 | 1): void {
  // Optimistically reflect the new vote and shift the score so ordering
  // updates locally before the server's authoritative update echoes back.
  update(channelId, (s) => ({
    ...s,
    queue: s.queue
      .map((q) => {
        if (q.id !== itemId) return q;
        const prev = q.my_vote ?? 0;
        const delta = value - prev;
        return { ...q, score: q.score + delta, my_vote: value };
      })
      .sort((a, b) => b.score - a.score),
  }));
  wsClient.send({
    v: 1,
    type: 'watch_vote',
    channel_id: channelId,
    item_id: itemId,
    value,
  });
}

export function sendSkip(channelId: string): void {
  wsClient.send({ v: 1, type: 'watch_skip', channel_id: channelId });
}

export function sendReaction(channelId: string, emoji: string): void {
  wsClient.send({
    v: 1,
    type: 'watch_reaction',
    channel_id: channelId,
    emoji,
  });
}

/// Leader-only. Periodically reported to the server so it can detect
/// completion (>=90% of duration) and auto-advance at end-of-stream.
/// No-ops for followers — the server validates leadership.
export function sendProgress(channelId: string, positionMs: number): void {
  wsClient.send({
    v: 1,
    type: 'watch_progress',
    channel_id: channelId,
    position_ms: Math.max(0, Math.round(positionMs)),
  });
}

/// Parse a YouTube video id from a URL or accept a raw 11-char id.
/// Returns `null` if the input isn't recognizable.
export function parseYouTubeId(input: string): string | null {
  const trimmed = input.trim();
  if (/^[A-Za-z0-9_-]{11}$/.test(trimmed)) return trimmed;
  try {
    const url = new URL(trimmed);
    if (url.hostname === 'youtu.be') {
      const id = url.pathname.replace(/^\//, '');
      return /^[A-Za-z0-9_-]{11}$/.test(id) ? id : null;
    }
    if (url.hostname.endsWith('youtube.com')) {
      const v = url.searchParams.get('v');
      if (v && /^[A-Za-z0-9_-]{11}$/.test(v)) return v;
    }
  } catch {
    return null;
  }
  return null;
}
