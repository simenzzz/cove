import { writable, derived, get } from 'svelte/store';

type MessageStatus = 'pending' | 'sent' | 'failed';

export interface ChatMessage {
  id: string;
  nonce?: string;
  content: string;
  authorId: string;
  channelId: string;
  seq?: number;
  createdAt: string;
  status: MessageStatus;
}

export interface ChannelState {
  messages: ChatMessage[];
  lastSeq: number;
}

export interface ChatState {
  channels: Map<string, ChannelState>;
  activeChannelId: string | null;
}

const MAX_MESSAGES_PER_CHANNEL = 200;

const initialState: ChatState = {
  channels: new Map(),
  activeChannelId: null,
};

export const chat = writable<ChatState>(initialState);

export const activeChannelMessages = derived(chat, ($chat) => {
  if (!$chat.activeChannelId) return [];
  const channel = $chat.channels.get($chat.activeChannelId);
  return channel?.messages ?? [];
});

export function addOptimisticMessage(channelId: string, msg: ChatMessage): void {
  chat.update((state) => {
    const channel = state.channels.get(channelId) ?? { messages: [], lastSeq: 0 };
    const messages = [...channel.messages, { ...msg, status: 'pending' as const }];
    return {
      ...state,
      channels: new Map(state.channels).set(channelId, { ...channel, messages }),
    };
  });
}

export function confirmMessage(nonce: string, messageId: string, seq: number): void {
  chat.update((state) => {
    const newChannels = new Map(state.channels);
    for (const [channelId, channel] of newChannels) {
      const idx = channel.messages.findIndex((m) => m.nonce === nonce);
      if (idx !== -1) {
        const messages = [...channel.messages];
        messages[idx] = {
          ...messages[idx],
          id: messageId,
          seq,
          status: 'sent',
        };
        newChannels.set(channelId, { ...channel, messages, lastSeq: seq });
        break;
      }
    }
    return { ...state, channels: newChannels };
  });
}

export function addReceivedMessage(channelId: string, msg: ChatMessage): void {
  chat.update((state) => {
    const channel = state.channels.get(channelId) ?? { messages: [], lastSeq: 0 };

    // Dedupe by id or nonce
    if (
      channel.messages.some(
        (m) => m.id === msg.id || (msg.nonce && m.nonce === msg.nonce),
      )
    )
      return state;

    let messages = [...channel.messages, msg];
    if (messages.length > MAX_MESSAGES_PER_CHANNEL) {
      messages = messages.slice(-MAX_MESSAGES_PER_CHANNEL);
    }

    const lastSeq = msg.seq ?? channel.lastSeq;
    return {
      ...state,
      channels: new Map(state.channels).set(channelId, { ...channel, messages, lastSeq }),
    };
  });
}

export function setActiveChannel(channelId: string | null): void {
  chat.update((state) => ({ ...state, activeChannelId: channelId }));
}

export function getLastSeqPerChannel(): Record<string, number> {
  const state = get(chat);
  const result: Record<string, number> = {};
  for (const [id, channel] of state.channels) {
    if (channel.lastSeq > 0) {
      result[id] = channel.lastSeq;
    }
  }
  return result;
}
