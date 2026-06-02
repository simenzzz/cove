import { writable } from 'svelte/store';
import { api } from '$lib/api/client';
import { recordKey } from '$lib/utils/record-id';
import type { Channel } from './channels';
import { friendId, type FriendUser } from './friends';

export interface DirectMessageSummary {
  channel: Channel;
  friend: FriendUser;
}

export interface DirectMessageTarget {
  key: string;
  userId: string;
  channelId: string | null;
  label: string;
  friend: FriendUser;
  dm: DirectMessageSummary | null;
  hasDm: boolean;
}

export const directMessages = writable<Map<string, DirectMessageSummary>>(new Map());
export const latestDmChannelId = writable<string | null>(null);

export function dmId(dm: DirectMessageSummary): string {
  return recordKey(dm.channel.id);
}

export function dmLabel(dm: DirectMessageSummary): string {
  return dm.friend.display_name || dm.friend.username;
}

export function dmTargets(
  dms: Iterable<DirectMessageSummary>,
  acceptedFriends: FriendUser[],
): DirectMessageTarget[] {
  const targets = new Map<string, DirectMessageTarget>();

  for (const dm of dms) {
    const userId = friendId(dm.friend);
    const channelId = dmId(dm);
    const key = userId || channelId;
    if (!key || !channelId) continue;
    targets.set(key, {
      key,
      userId,
      channelId,
      label: dmLabel(dm),
      friend: dm.friend,
      dm,
      hasDm: true,
    });
  }

  for (const friend of acceptedFriends) {
    const userId = friendId(friend);
    if (!userId || targets.has(userId)) continue;
    const label = friend.display_name || friend.username;
    targets.set(userId, {
      key: userId,
      userId,
      channelId: null,
      label,
      friend,
      dm: null,
      hasDm: false,
    });
  }

  return Array.from(targets.values()).sort((a, b) => a.label.localeCompare(b.label));
}

export function upsertDm(dm: DirectMessageSummary): void {
  const id = dmId(dm);
  if (!id) return;
  latestDmChannelId.set(id);
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
    latestDmChannelId.set(data.dms.length > 0 ? dmId(data.dms[0]) || null : null);
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
