import { writable } from 'svelte/store';

export interface TypingUser {
  userId: string;
  username: string;
  expiresAt: number;
}

// Map<channelId, Map<userId, TypingUser>>
export const typing = writable<Map<string, Map<string, TypingUser>>>(new Map());

const clearTimeouts = new Map<string, ReturnType<typeof setTimeout>>();

export function setTyping(channelId: string, userId: string, username: string): void {
  const key = `${channelId}:${userId}`;
  const expiresAt = Date.now() + 5000;

  typing.update((map) => {
    const newMap = new Map(map);
    const channelMap = new Map(newMap.get(channelId) ?? new Map());
    channelMap.set(userId, { userId, username, expiresAt });
    newMap.set(channelId, channelMap);
    return newMap;
  });

  // Auto-clear after 5 seconds
  if (clearTimeouts.has(key)) {
    clearTimeout(clearTimeouts.get(key)!);
  }
  clearTimeouts.set(
    key,
    setTimeout(() => {
      typing.update((map) => {
        const newMap = new Map(map);
        const channelMap = new Map(newMap.get(channelId) ?? new Map());
        channelMap.delete(userId);
        if (channelMap.size === 0) {
          newMap.delete(channelId);
        } else {
          newMap.set(channelId, channelMap);
        }
        return newMap;
      });
      clearTimeouts.delete(key);
    }, 5000),
  );
}
