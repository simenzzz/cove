import { writable } from 'svelte/store';
import { api } from '$lib/api/client';
import { recordKey } from '$lib/utils/record-id';

export interface Channel {
  id: { id?: string; tb?: string } | string | null;
  name: string;
  channel_type: 'text' | 'voice' | 'collab' | 'whiteboard' | 'watch' | 'direct';
  server: { id?: string; tb?: string } | string | null;
  created_at?: string | null;
}

export type ChannelType = Channel['channel_type'];

export const channels = writable<Map<string, Channel[]>>(new Map());

export async function fetchChannels(serverId: string): Promise<void> {
  try {
    const data = await api.get<{ channels: Channel[] }>(
      `/api/servers/${serverId}/channels`,
    );
    const list = data.channels;
    channels.update((map) => new Map(map).set(serverId, list));
  } catch (err) {
    console.error('Failed to fetch channels:', err);
  }
}

export async function createChannel(
  serverId: string,
  name: string,
  channelType: ChannelType,
): Promise<Channel> {
  const { channel } = await api.post<{ channel: Channel }>(
    `/api/servers/${serverId}/channels`,
    { name, channel_type: channelType },
  );
  await fetchChannels(serverId);
  return channel;
}

export function addChannel(serverId: string, channel: Channel): void {
  channels.update((map) => {
    const next = new Map(map);
    const existing = next.get(serverId) ?? [];
    const channelId = recordKey(channel.id);
    const filtered = existing.filter((c) => recordKey(c.id) !== channelId);
    next.set(serverId, [...filtered, channel]);
    return next;
  });
}
