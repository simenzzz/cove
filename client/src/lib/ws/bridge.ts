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
import { addChannel, type Channel } from '$stores/channels';

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
      const chatMsg: ChatMessage = {
        id: msg.message_id as string,
        channelId: msg.channel_id as string,
        content: msg.content as string,
        authorId: author ? recordKey(author.id) : '',
        authorUsername: author ? String(author.username ?? '') : '',
        authorDisplayName: author ? String(author.display_name ?? '') : '',
        authorAvatarUrl: author?.avatar_url == null ? null : String(author.avatar_url),
        seq: msg.seq as number,
        createdAt: new Date(msg.ts as number).toISOString(),
        status: 'sent',
      };
      addReceivedMessage(chatMsg.channelId, chatMsg);
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
    wsClient.on('unread', () => {
      // TODO: Update unread counts store
    }),
  );

  cleanups.push(
    wsClient.on('server_joined', (msg) => {
      const server = msg.server as Server | undefined;
      if (server) addServer(server);
    }),
  );

  cleanups.push(
    wsClient.on('channel_created', (msg) => {
      const serverId = String(msg.server_id ?? '');
      const channel = msg.channel as Channel | undefined;
      if (serverId && channel) addChannel(serverId, channel);
    }),
  );

  // Return cleanup function to unregister all handlers
  return () => {
    for (const cleanup of cleanups) {
      cleanup();
    }
  };
}
