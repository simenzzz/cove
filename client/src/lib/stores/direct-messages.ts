import { writable } from 'svelte/store';
import { api } from '$lib/api/client';
import { recordKey } from '$lib/utils/record-id';
import type { Channel } from './channels';
import type { FriendUser } from './friends';

export interface DirectMessageSummary {
  channel: Channel;
  friend: FriendUser;
}

export const directMessages = writable<Map<string, DirectMessageSummary>>(new Map());

export function dmId(dm: DirectMessageSummary): string {
  return recordKey(dm.channel.id);
}

export function dmLabel(dm: DirectMessageSummary): string {
  return dm.friend.display_name || dm.friend.username;
}

export function upsertDm(dm: DirectMessageSummary): void {
  const id = dmId(dm);
  if (!id) return;
  directMessages.update((map) => {
    const next = new Map(map);
    next.set(id, dm);
    return next;
  });
}

export async function fetchDms(): Promise<void> {
  try {
    const data = await api.get<{ dms: DirectMessageSummary[] }>('/api/dms');
    directMessages.set(new Map(data.dms.map((dm) => [dmId(dm), dm])));
  } catch (err) {
    console.error('Failed to fetch direct messages:', err);
  }
}

export async function openDm(userId: string): Promise<DirectMessageSummary> {
  const { dm } = await api.post<{ dm: DirectMessageSummary }>('/api/dms', {
    user_id: userId,
  });
  upsertDm(dm);
  return dm;
}
