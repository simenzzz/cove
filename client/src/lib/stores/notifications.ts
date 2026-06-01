import { writable } from 'svelte/store';

export type NotificationKind =
  | 'friend_request'
  | 'friend_accepted'
  | 'channel_message'
  | 'direct_message'
  | 'server_member_joined'
  | 'server_member_left'
  | 'server_joined'
  | 'channel_created'
  | 'system';

export interface AppNotification {
  id: number;
  key: string;
  kind: NotificationKind;
  title: string;
  body: string;
  href?: string;
  createdAt: string;
  read: boolean;
}

export interface NotificationState {
  items: AppNotification[];
  unread: number;
  lastPushedId: number | null;
}

export interface NotificationInput {
  key?: string;
  kind: NotificationKind;
  title: string;
  body: string;
  href?: string;
  createdAt?: string;
}

const MAX_NOTIFICATIONS = 30;

const initialState: NotificationState = {
  items: [],
  unread: 0,
  lastPushedId: null,
};

function countUnread(items: AppNotification[]): number {
  return items.filter((item) => !item.read).length;
}

function defaultKey(input: NotificationInput): string {
  return `${input.kind}:${input.href ?? ''}:${input.title}:${input.body}`;
}

function createNotificationsStore() {
  const { subscribe, update, set } = writable<NotificationState>(initialState);
  let nextId = 0;

  function push(input: NotificationInput): number {
    const key = input.key ?? defaultKey(input);
    const id = ++nextId;
    update((state) => {
      const existing = state.items.filter((item) => item.key !== key);
      const item: AppNotification = {
        id,
        key,
        kind: input.kind,
        title: input.title,
        body: input.body,
        href: input.href,
        createdAt: input.createdAt ?? new Date().toISOString(),
        read: false,
      };
      const items = [item, ...existing].slice(0, MAX_NOTIFICATIONS);
      return { items, unread: countUnread(items), lastPushedId: id };
    });
    return id;
  }

  function dismiss(id: number): void {
    update((state) => {
      const items = state.items.filter((item) => item.id !== id);
      return { ...state, items, unread: countUnread(items) };
    });
  }

  function markRead(id: number): void {
    update((state) => {
      const items = state.items.map((item) =>
        item.id === id ? { ...item, read: true } : item,
      );
      return { ...state, items, unread: countUnread(items) };
    });
  }

  function markAllRead(): void {
    update((state) => ({
      ...state,
      items: state.items.map((item) => ({ ...item, read: true })),
      unread: 0,
    }));
  }

  function clear(): void {
    set(initialState);
  }

  return {
    subscribe,
    push,
    dismiss,
    markRead,
    markAllRead,
    clear,
  };
}

export const notifications = createNotificationsStore();
