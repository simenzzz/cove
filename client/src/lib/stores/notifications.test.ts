import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { notifications, type NotificationKind } from './notifications';

function push(kind: NotificationKind = 'system', key = 'k') {
  return notifications.push({
    key,
    kind,
    title: 'Notice',
    body: 'Something happened',
    href: '/feed',
    createdAt: '2026-01-01T00:00:00.000Z',
  });
}

beforeEach(() => {
  notifications.clear();
});

describe('notifications store', () => {
  it('pushes unread notifications newest first', () => {
    push('system', 'a');
    push('friend_request', 'b');
    const state = get(notifications);
    expect(state.items.map((item) => item.key)).toEqual(['b', 'a']);
    expect(state.unread).toBe(2);
  });

  it('dedupes by key and keeps the newest copy unread', () => {
    const first = push('system', 'same');
    const second = push('system', 'same');
    const state = get(notifications);
    expect(first).not.toBe(second);
    expect(state.items).toHaveLength(1);
    expect(state.items[0].id).toBe(second);
    expect(state.unread).toBe(1);
  });

  it('marks one notification as read', () => {
    const id = push();
    notifications.markRead(id);
    const state = get(notifications);
    expect(state.items[0].read).toBe(true);
    expect(state.unread).toBe(0);
  });

  it('dismisses notifications and updates unread count', () => {
    const id = push();
    notifications.dismiss(id);
    expect(get(notifications).items).toHaveLength(0);
    expect(get(notifications).unread).toBe(0);
  });

  it('caps retained notifications at 30', () => {
    for (let i = 0; i < 35; i++) {
      push('system', `k${i}`);
    }
    const state = get(notifications);
    expect(state.items).toHaveLength(30);
    expect(state.items.at(-1)?.key).toBe('k5');
  });
});
