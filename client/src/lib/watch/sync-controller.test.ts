import { describe, it, expect, vi } from 'vitest';
import { createSyncController } from './sync-controller';
import type { YouTubePlayer } from './youtube-player';
import type { WatchPlayback } from '$stores/watch';

function player(position = 0): YouTubePlayer & { calls: string[]; rates: number[] } {
  const calls: string[] = [];
  const rates: number[] = [];
  return {
    calls,
    rates,
    ready: Promise.resolve(),
    play: vi.fn(() => calls.push('play')),
    pause: vi.fn(() => calls.push('pause')),
    seekTo: vi.fn((ms: number) => {
      calls.push(`seek:${ms}`);
      position = ms;
    }),
    loadVideo: vi.fn((videoId: string, startMs: number) => calls.push(`load:${videoId}:${startMs}`)),
    cueVideo: vi.fn((videoId: string, startMs: number) => calls.push(`cue:${videoId}:${startMs}`)),
    getPosition: vi.fn(() => position),
    getDuration: vi.fn(() => 0),
    setRate: vi.fn((rate: number) => {
      rates.push(rate);
      calls.push(`rate:${rate}`);
    }),
    on: vi.fn(() => () => {}),
    destroy: vi.fn(),
  };
}

function playback(patch: Partial<WatchPlayback> = {}): WatchPlayback {
  return {
    video_id: 'dQw4w9WgXcQ',
    position_ms: 1_000,
    paused: false,
    server_ts: Date.now(),
    rate: 1.5,
    ...patch,
  };
}

describe('createSyncController', () => {
  it('reapplies the authoritative room playback rate after hydrating a paused video', () => {
    const p = player(1_000);
    const sync = createSyncController(p);

    sync.apply(playback({ paused: true }), false);

    expect(p.rates.at(-1)).toBe(1.5);
    expect(p.calls.at(-2)).toBe('cue:dQw4w9WgXcQ:1000');
    expect(p.calls.at(-1)).toBe('rate:1.5');
  });

  it('restores the authoritative rate after temporary drift correction', () => {
    vi.useFakeTimers();
    try {
      const p = player(1_260);
      const sync = createSyncController(p);

      sync.reconcile(playback({ rate: 1.5 }), false);
      expect(p.rates.at(-1)).toBeCloseTo(1.485);

      vi.advanceTimersByTime(4_000);
      expect(p.rates.at(-1)).toBe(1.5);
    } finally {
      vi.useRealTimers();
    }
  });
});
