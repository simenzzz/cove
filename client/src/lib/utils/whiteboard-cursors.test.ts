import { describe, expect, it } from 'vitest';
import { whiteboardCursorLabel, whiteboardPeerCursors } from './whiteboard-cursors';

describe('whiteboardCursorLabel', () => {
  it('uses display_name when present', () => {
    expect(
      whiteboardCursorLabel('user:123456789', {
        cursor: { x: 1, y: 2 },
        display_name: 'Alice Display',
        username: 'alice',
      }),
    ).toBe('Alice Display');
  });

  it('falls back to username', () => {
    expect(
      whiteboardCursorLabel('user:123456789', {
        cursor: { x: 1, y: 2 },
        username: 'alice',
      }),
    ).toBe('alice');
  });

  it('falls back to a truncated user id for legacy awareness payloads', () => {
    expect(whiteboardCursorLabel('user:123456789', { cursor: { x: 1, y: 2 } })).toBe(
      'user:123',
    );
  });
});

describe('whiteboardPeerCursors', () => {
  it('normalizes valid peer cursor awareness entries', () => {
    expect(
      whiteboardPeerCursors({
        'user:alice': {
          cursor: { x: 10, y: 20 },
          display_name: 'Alice',
          color: '#123456',
          tool: 'line',
        },
      }),
    ).toEqual([
      {
        userId: 'user:alice',
        label: 'Alice',
        x: 10,
        y: 20,
        color: '#123456',
        tool: 'line',
      },
    ]);
  });

  it('skips malformed awareness entries without cursor coordinates', () => {
    expect(
      whiteboardPeerCursors({
        'user:alice': { display_name: 'Alice' },
        'user:bob': { cursor: { x: 10 } },
      }),
    ).toEqual([]);
  });
});
