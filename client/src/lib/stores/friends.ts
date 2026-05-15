import { writable } from 'svelte/store';
import { api } from '$lib/api/client';

export interface FriendUser {
  id: { id?: string; tb?: string } | null;
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

export async function sendRequest(userId: string): Promise<void> {
  await api.post('/api/friends/request', { user_id: userId });
}

export async function acceptRequest(userId: string): Promise<void> {
  await api.post('/api/friends/accept', { user_id: userId });
  friends.update((state) => ({
    ...state,
    pending: state.pending.filter((u) => u.id !== userId),
  }));
}

export async function removeFriend(userId: string): Promise<void> {
  await api.delete(`/api/friends/${userId}`);
  friends.update((state) => ({
    ...state,
    friends: state.friends.filter((u) => u.id !== userId),
  }));
}
