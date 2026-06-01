import { describe, it, expect, beforeEach, vi, type Mock } from 'vitest';
import { get } from 'svelte/store';

vi.mock('$lib/api/client', () => ({
  api: { get: vi.fn(), post: vi.fn() },
}));

import { api } from '$lib/api/client';
import {
  toolState,
  layerVisibility,
  checkpointIdToString,
  fetchWhiteboard,
  createCheckpoint,
  listCheckpoints,
  restoreCheckpoint,
  type WhiteboardCheckpoint,
} from './whiteboards';

const getReq = api.get as Mock;
const post = api.post as Mock;

function checkpoint(idKey: string): WhiteboardCheckpoint {
  return {
    id: { tb: 'whiteboard_checkpoint', id: { String: idKey } },
    channel: { tb: 'channel', id: { String: 'c1' } },
    state_b64: '',
    label: null,
    created_at: null,
  };
}

beforeEach(() => {
  toolState.set({ tool: 'pen', color: '#222222', strokeWidth: 3, activeLayerId: 'default' });
  layerVisibility.set({});
});

describe('checkpointIdToString', () => {
  it('unwraps the codec object form', () => {
    expect(checkpointIdToString(checkpoint('abc'))).toBe('abc');
  });

  it('strips the whiteboard_checkpoint: prefix from a string id', () => {
    expect(checkpointIdToString({ ...checkpoint('x'), id: 'whiteboard_checkpoint:abc' })).toBe(
      'abc',
    );
  });
});

describe('toolState / layerVisibility stores', () => {
  it('updates the active tool immutably', () => {
    const before = get(toolState);
    toolState.update((s) => ({ ...s, tool: 'eraser' }));
    expect(get(toolState).tool).toBe('eraser');
    expect(before.tool).toBe('pen');
  });

  it('tracks per-layer visibility', () => {
    layerVisibility.update((v) => ({ ...v, layerA: false }));
    expect(get(layerVisibility).layerA).toBe(false);
  });
});

describe('whiteboard API helpers', () => {
  it('fetchWhiteboard requests the channel snapshot', async () => {
    getReq.mockResolvedValueOnce({ channel_id: 'c1', state_b64: '', state_vector_b64: '', snapshot_count: 0 });
    const snap = await fetchWhiteboard('c1');
    expect(getReq).toHaveBeenCalledWith('/api/channels/c1/whiteboard');
    expect(snap.channel_id).toBe('c1');
  });

  it('createCheckpoint posts the label and returns the checkpoint', async () => {
    post.mockResolvedValueOnce({ checkpoint: checkpoint('cp1') });
    const cp = await createCheckpoint('c1', 'milestone');
    expect(post).toHaveBeenCalledWith('/api/channels/c1/whiteboard/checkpoints', {
      label: 'milestone',
    });
    expect(checkpointIdToString(cp)).toBe('cp1');
  });

  it('createCheckpoint sends null label when omitted', async () => {
    post.mockResolvedValueOnce({ checkpoint: checkpoint('cp2') });
    await createCheckpoint('c1');
    expect(post).toHaveBeenCalledWith('/api/channels/c1/whiteboard/checkpoints', { label: null });
  });

  it('listCheckpoints returns the array', async () => {
    getReq.mockResolvedValueOnce({ checkpoints: [checkpoint('a'), checkpoint('b')] });
    const list = await listCheckpoints('c1');
    expect(list).toHaveLength(2);
  });

  it('restoreCheckpoint posts to the restore endpoint', async () => {
    post.mockResolvedValueOnce(undefined);
    await restoreCheckpoint('c1', 'cp1');
    expect(post).toHaveBeenCalledWith(
      '/api/channels/c1/whiteboard/checkpoints/cp1/restore',
      {},
    );
  });
});
