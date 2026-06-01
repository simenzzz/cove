import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { get } from 'svelte/store';
import { toasts } from './toast';

beforeEach(() => {
  vi.useFakeTimers();
  // Drain anything left from a prior test.
  for (const t of get(toasts)) toasts.dismiss(t.id);
});

afterEach(() => {
  vi.clearAllTimers();
  vi.useRealTimers();
});

describe('toasts store', () => {
  it('pushes a toast with the given message and tone', () => {
    toasts.push('hi', 'info');
    const list = get(toasts);
    expect(list).toHaveLength(1);
    expect(list[0]).toMatchObject({ message: 'hi', tone: 'info' });
  });

  it('assigns monotonically increasing ids', () => {
    const a = toasts.push('a');
    const b = toasts.push('b');
    expect(b).toBeGreaterThan(a);
  });

  it('auto-dismisses after the timeout', () => {
    toasts.push('bye', 'info', 4000);
    expect(get(toasts)).toHaveLength(1);
    vi.advanceTimersByTime(4000);
    expect(get(toasts)).toHaveLength(0);
  });

  it('does not auto-dismiss when timeout is 0', () => {
    toasts.push('sticky', 'error', 0);
    vi.advanceTimersByTime(60_000);
    expect(get(toasts)).toHaveLength(1);
  });

  it('dismiss removes only the targeted toast', () => {
    const a = toasts.push('a', 'info', 0);
    toasts.push('b', 'info', 0);
    toasts.dismiss(a);
    const list = get(toasts);
    expect(list).toHaveLength(1);
    expect(list[0].message).toBe('b');
  });

  it('success / error / info helpers set the tone', () => {
    toasts.success('ok', 0);
    toasts.error('no', 0);
    toasts.info('fyi', 0);
    expect(get(toasts).map((t) => t.tone)).toEqual(['success', 'error', 'info']);
  });
});
