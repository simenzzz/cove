import { writable } from 'svelte/store';
import { api } from '$lib/api/client';
import { recordKey } from '$lib/utils/record-id';

export interface FriendUser {
  id: { id?: string; tb?: string } | string | null;
  username: string;
  display_name: string;
  avatar_url?: string | null;
}

export interface FriendState {
  friends: FriendUser[];
  pending: FriendUser[];
  suggestions: FriendUser[];
}

const initialState: FriendState = {
  friends: [],
  pending: [],
  suggestions: [],
};

export const friends = writable<FriendState>(initialState);

/** Bare user key for a friend record, regardless of serialization shape. */
export function friendId(user: FriendUser): string {
  return recordKey(user.id);
}

export function isAcceptedFriend(userOrId: FriendUser | string, state: FriendState): boolean {
  const id = typeof userOrId === 'string' ? userOrId : friendId(userOrId);
  if (!id) return false;
  return state.friends.some((user) => friendId(user) === id);
}

export async function fetchFriends(): Promise<void> {
  try {
    const [friendsData, pendingData, suggestionsData] = await Promise.all([
      api.get<{ friends: FriendUser[] }>('/api/friends'),
      api.get<{ pending: FriendUser[] }>('/api/friends/pending'),
      api.get<{ suggestions: FriendUser[] }>('/api/friends/suggestions'),
    ]);
    friends.set({
      friends: friendsData.friends,
      pending: pendingData.pending,
      suggestions: suggestionsData.suggestions,
    });
  } catch (err) {
    console.error('Failed to fetch friends:', err);
  }
}

/** Resolve a username to its user record. Throws if not found. */
export async function lookupUser(username: string): Promise<FriendUser> {
  const { user } = await api.get<{ user: FriendUser }>(
    `/api/users/by-username/${encodeURIComponent(username)}`,
  );
  return user;
}

export async function sendRequest(userId: string): Promise<void> {
  await api.post('/api/friends/request', { user_id: userId });
}

/** Accept an incoming request from `senderId`, then refresh lists. */
export async function acceptRequest(senderId: string): Promise<void> {
  await api.post('/api/friends/accept', { user_id: senderId });
  await fetchFriends();
}

/** Decline an incoming request (deletes the pending edge). */
export async function declineRequest(senderId: string): Promise<void> {
  await api.delete(`/api/friends/${senderId}`);
  friends.update((state) => ({
    ...state,
    pending: state.pending.filter((u) => friendId(u) !== senderId),
  }));
}

export async function removeFriend(userId: string): Promise<void> {
  await api.delete(`/api/friends/${userId}`);
  friends.update((state) => ({
    ...state,
    friends: state.friends.filter((u) => friendId(u) !== userId),
  }));
}
