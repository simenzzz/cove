import { afterEach, vi } from 'vitest';

// Always hand each test a clean real-timer baseline. Suites that opt into fake
// timers do so explicitly; this guarantees they can't leak into the next file.
afterEach(() => {
  vi.useRealTimers();
});

// jsdom does not expose `crypto.randomUUID` in every environment; the watch
// store relies on it for optimistic queue nonces.
if (!globalThis.crypto?.randomUUID) {
  Object.defineProperty(globalThis, 'crypto', {
    value: {
      ...globalThis.crypto,
      randomUUID: () =>
        '00000000-0000-4000-8000-' + Math.random().toString(16).slice(2, 14).padEnd(12, '0'),
    },
    configurable: true,
  });
}
