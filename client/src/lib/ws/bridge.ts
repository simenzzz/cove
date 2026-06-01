import { wsClient } from './client';
import {
  addReceivedMessage,
  confirmMessage,
  type ChatMessage,
} from '$stores/chat';
import { setTyping } from '$stores/typing';
import { presence, type UserStatus } from '$stores/presence';
import { recordKey } from '$lib/utils/record-id';
import { addServer, type Server } from '$stores/servers';
import { servers } from '$stores/servers';
import { addChannel, channels, type Channel } from '$stores/channels';
import { notifications } from '$stores/notifications';
import { auth } from '$stores/auth';
import { get } from 'svelte/store';
import { upsertDm, dmLabel, type DirectMessageSummary } from '$stores/direct-messages';

interface NotificationUser {
  id?: unknown;
  username?: unknown;
  display_name?: unknown;
  avatar_url?: unknown;
}

function userLabel(user: NotificationUser | undefined, fallback = 'Someone'): string {
  const display = user?.display_name == null ? '' : String(user.display_name);
  const username = user?.username == null ? '' : String(user.username);
  return display || username || fallback;
}

function isCurrentUser(id: unknown): boolean {
  const current = get(auth).user?.id ?? '';
  return current !== '' && recordKey(id) === recordKey(current);
}

function channelHref(channelId: string): string | undefined {
  const allChannels = get(channels);
  for (const [serverId, list] of allChannels) {
    const channel = list.find((candidate) => recordKey(candidate.id) === channelId);
    if (!channel) continue;
    if (channel.channel_type === 'voice') return `/servers/${serverId}/channels/${channelId}/voice`;
    if (channel.channel_type === 'collab') return `/servers/${serverId}/channels/${channelId}/collab`;
    if (channel.channel_type === 'whiteboard') {
      return `/servers/${serverId}/channels/${channelId}/whiteboard`;
    }
    if (channel.channel_type === 'watch') return `/servers/${serverId}/channels/${channelId}/watch`;
    return `/servers/${serverId}/channels/${channelId}`;
  }
  return undefined;
}

function channelName(channelId: string): string {
  for (const list of get(channels).values()) {
    const channel = list.find((candidate) => recordKey(candidate.id) === channelId);
    if (channel) return channel.name;
  }
  return 'channel';
}

/**
 * Registers WS message handlers that dispatch to the appropriate stores.
 * Call once after WS connection is established.
 */
export function initBridge(): () => void {
  const cleanups: (() => void)[] = [];

  // chat_message → addReceivedMessage
  cleanups.push(
    wsClient.on('chat_message', (msg) => {
      const author = msg.author as Record<string, unknown> | undefined;
      const authorId = author ? recordKey(author.id) : '';
      const chatMsg: ChatMessage = {
        id: msg.message_id as string,
        channelId: msg.channel_id as string,
        content: msg.content as string,
        authorId,
        authorUsername: author ? String(author.username ?? '') : '',
        authorDisplayName: author ? String(author.display_name ?? '') : '',
        authorAvatarUrl: author?.avatar_url == null ? null : String(author.avatar_url),
        seq: msg.seq as number,
        createdAt: new Date(msg.ts as number).toISOString(),
        status: 'sent',
      };
      addReceivedMessage(chatMsg.channelId, chatMsg);
      if (!isCurrentUser(authorId)) {
        const name = chatMsg.authorDisplayName || chatMsg.authorUsername || 'Someone';
        const href = channelHref(chatMsg.channelId);
        notifications.push({
          key: `message:${chatMsg.id}`,
          kind: 'channel_message',
          title: `New message in #${channelName(chatMsg.channelId)}`,
          body: `${name}: ${chatMsg.content}`,
          href,
          createdAt: chatMsg.createdAt,
        });
      }
    }),
  );

  // message_ack → confirmMessage
  cleanups.push(
    wsClient.on('message_ack', (msg) => {
      confirmMessage(
        msg.nonce as string,
        msg.message_id as string,
        msg.seq as number,
      );
    }),
  );

  // typing → update typing store
  cleanups.push(
    wsClient.on('typing', (msg) => {
      setTyping(
        msg.channel_id as string,
        msg.user_id as string,
        msg.username as string,
      );
    }),
  );

  // presence → update presence store
  cleanups.push(
    wsClient.on('presence', (msg) => {
      const userId = msg.user_id as string;
      const status = msg.status as UserStatus;
      presence.update((state) => {
        const newStatuses = new Map(state.statuses);
        newStatuses.set(userId, status);
        return { statuses: newStatuses };
      });
    }),
  );

  // unread → (placeholder for future notification badge support)
  cleanups.push(
    wsClient.on('unread', (msg) => {
      const channelId = String(msg.channel_id ?? '');
      if (!channelId) return;
      const count = Number(msg.count ?? 0);
      const preview = String(msg.last_message_preview ?? '').trim();
      notifications.push({
        key: `unread:${channelId}`,
        kind: 'channel_message',
        title: `${count || 1} unread in #${channelName(channelId)}`,
        body: preview || 'New activity in this channel.',
        href: channelHref(channelId),
      });
    }),
  );

  cleanups.push(
    wsClient.on('dm_channel_updated', (msg) => {
      const dm = msg.dm as DirectMessageSummary | undefined;
      if (!dm) return;
      upsertDm(dm);
      const channelId = recordKey(dm.channel.id);
      const fromUser = msg.from_user as NotificationUser | undefined;
      if (!isCurrentUser(fromUser?.id)) {
        notifications.push({
          key: `direct_message:${channelId}:${String(msg.ts ?? '')}`,
          kind: 'direct_message',
          title: dmLabel(dm),
          body: String(msg.last_message_preview ?? 'New message'),
          href: channelId ? `/dms/${channelId}` : undefined,
          createdAt:
            typeof msg.ts === 'number' ? new Date(msg.ts).toISOString() : undefined,
        });
      }
    }),
  );

  cleanups.push(
    wsClient.on('server_joined', (msg) => {
      const server = msg.server as Server | undefined;
      if (server) {
        addServer(server);
        const id = recordKey(server.id);
        notifications.push({
          key: `server_joined:${id}`,
          kind: 'server_joined',
          title: `Joined ${server.name}`,
          body: 'The server is now in your sidebar.',
          href: id ? `/servers/${id}` : undefined,
        });
      }
    }),
  );

  cleanups.push(
    wsClient.on('channel_created', (msg) => {
      const serverId = String(msg.server_id ?? '');
      const channel = msg.channel as Channel | undefined;
      if (serverId && channel) {
        addChannel(serverId, channel);
        const channelId = recordKey(channel.id);
        notifications.push({
          key: `channel_created:${serverId}:${channelId}`,
          kind: 'channel_created',
          title: `#${channel.name} was created`,
          body: `${get(servers).get(serverId)?.name ?? 'A server'} has a new channel.`,
          href: channelHref(channelId) ?? `/servers/${serverId}`,
        });
      }
    }),
  );

  cleanups.push(
    wsClient.on('friend_request_received', (msg) => {
      const fromUser = msg.from_user as NotificationUser | undefined;
      notifications.push({
        key: `friend_request:${recordKey(fromUser?.id)}`,
        kind: 'friend_request',
        title: 'New friend request',
        body: `${userLabel(fromUser)} wants to connect.`,
        href: '/friends',
      });
    }),
  );

  cleanups.push(
    wsClient.on('friend_request_accepted', (msg) => {
      const user = msg.user as NotificationUser | undefined;
      notifications.push({
        key: `friend_accepted:${recordKey(user?.id)}`,
        kind: 'friend_accepted',
        title: 'Friend request accepted',
        body: `${userLabel(user)} is now your friend.`,
        href: '/friends',
      });
    }),
  );

  cleanups.push(
    wsClient.on('server_member_joined', (msg) => {
      const serverId = String(msg.server_id ?? '');
      const user = msg.user as NotificationUser | undefined;
      if (!serverId || isCurrentUser(user?.id)) return;
      const server = get(servers).get(serverId);
      notifications.push({
        key: `server_member_joined:${serverId}:${recordKey(user?.id)}`,
        kind: 'server_member_joined',
        title: `${userLabel(user)} joined`,
        body: server ? `${server.name} has a new member.` : 'A server has a new member.',
        href: `/servers/${serverId}`,
      });
    }),
  );

  cleanups.push(
    wsClient.on('server_member_left', (msg) => {
      const serverId = String(msg.server_id ?? '');
      const user = msg.user as NotificationUser | undefined;
      if (!serverId || isCurrentUser(user?.id)) return;
      const server = get(servers).get(serverId);
      notifications.push({
        key: `server_member_left:${serverId}:${recordKey(user?.id)}`,
        kind: 'server_member_left',
        title: `${userLabel(user)} left`,
        body: server ? `${server.name} membership changed.` : 'A server membership changed.',
        href: `/servers/${serverId}`,
      });
    }),
  );

  // Return cleanup function to unregister all handlers
  return () => {
    for (const cleanup of cleanups) {
      cleanup();
    }
  };
}
