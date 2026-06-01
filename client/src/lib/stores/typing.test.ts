import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { get } from 'svelte/store';
import { typing, setTyping } from './typing';

beforeEach(() => {
  vi.useFakeTimers();
  typing.set(new Map());
});

afterEach(() => {
  vi.clearAllTimers();
  vi.useRealTimers();
});

describe('setTyping', () => {
  it('records a typing user under the channel', () => {
    setTyping('c1', 'u1', 'alice');
    const channelMap = get(typing).get('c1');
    expect(channelMap?.get('u1')?.username).toBe('alice');
  });

  it('tracks multiple users in the same channel', () => {
    setTyping('c1', 'u1', 'alice');
    setTyping('c1', 'u2', 'bob');
    expect(get(typing).get('c1')?.size).toBe(2);
  });

  it('auto-clears a user after 5 seconds', () => {
    setTyping('c1', 'u1', 'alice');
    expect(get(typing).get('c1')?.has('u1')).toBe(true);
    vi.advanceTimersByTime(5000);
    expect(get(typing).get('c1')).toBeUndefined();
  });

  it('removes the channel entry once its last typist clears', () => {
    setTyping('c1', 'u1', 'alice');
    setTyping('c1', 'u2', 'bob');
    vi.advanceTimersByTime(5000);
    expect(get(typing).has('c1')).toBe(false);
  });

  it('debounces: a fresh signal resets the 5s window for that user', () => {
    setTyping('c1', 'u1', 'alice');
    vi.advanceTimersByTime(3000);
    setTyping('c1', 'u1', 'alice'); // resets timer
    vi.advanceTimersByTime(3000); // 6s since first, 3s since reset
    expect(get(typing).get('c1')?.has('u1')).toBe(true);
    vi.advanceTimersByTime(2000); // now 5s since reset
    expect(get(typing).get('c1')).toBeUndefined();
  });

  it('does not mutate the previous map (immutability)', () => {
    const before = get(typing);
    setTyping('c1', 'u1', 'alice');
    expect(get(typing)).not.toBe(before);
    expect(before.size).toBe(0);
  });
});
