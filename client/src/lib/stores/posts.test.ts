import { describe, it, expect, beforeEach, vi, type Mock } from 'vitest';
import { get } from 'svelte/store';

vi.mock('$lib/api/client', () => ({
  api: { get: vi.fn(), post: vi.fn() },
}));

import { api } from '$lib/api/client';
import {
  posts,
  postIdToString,
  createDraft,
  fetchPost,
  publishPost,
  inviteCollaborator,
  fetchPublishedPosts,
  type Post,
} from './posts';

const getReq = api.get as Mock;
const post = api.post as Mock;

function makePost(idKey: string, overrides: Partial<Post> = {}): Post {
  return {
    id: { tb: 'post', id: { String: idKey } },
    author: { tb: 'user', id: { String: 'u1' } },
    title: 't',
    state_b64: '',
    state_vector_b64: '',
    published: false,
    published_content: null,
    created_at: null,
    updated_at: null,
    ...overrides,
  };
}

beforeEach(() => {
  posts.set({ byId: {} });
});

describe('postIdToString', () => {
  it('unwraps the codec object form', () => {
    expect(postIdToString(makePost('abc'))).toBe('abc');
  });

  it('strips the post: prefix from a string id', () => {
    expect(postIdToString({ ...makePost('x'), id: 'post:abc' })).toBe('abc');
  });

  it('returns "" when the id is unusable', () => {
    expect(postIdToString({ ...makePost('x'), id: { tb: 'post', id: {} } })).toBe('');
  });
});

describe('createDraft', () => {
  it('omits empty content and indexes the returned post', async () => {
    post.mockResolvedValueOnce({ post: makePost('d1') });
    await createDraft('Title', '   ');
    expect(post).toHaveBeenCalledWith('/api/posts', { title: 'Title' });
    expect(get(posts).byId['d1']).toBeDefined();
  });

  it('includes trimmed content when provided', async () => {
    post.mockResolvedValueOnce({ post: makePost('d2') });
    await createDraft('Title', 'hello');
    expect(post).toHaveBeenCalledWith('/api/posts', { title: 'Title', content: 'hello' });
  });
});

describe('fetchPost / publishPost', () => {
  it('fetchPost indexes the post by id', async () => {
    getReq.mockResolvedValueOnce({ post: makePost('p1') });
    const result = await fetchPost('p1');
    expect(result.title).toBe('t');
    expect(get(posts).byId['p1']).toBeDefined();
  });

  it('publishPost replaces the indexed copy', async () => {
    posts.set({ byId: { p1: makePost('p1') } });
    post.mockResolvedValueOnce({ post: makePost('p1', { published: true }) });
    await publishPost('p1');
    expect(get(posts).byId['p1'].published).toBe(true);
  });
});

describe('inviteCollaborator', () => {
  it('POSTs the invite with the user id', async () => {
    post.mockResolvedValueOnce({ ok: true });
    await inviteCollaborator('p1', 'u2');
    expect(post).toHaveBeenCalledWith('/api/posts/p1/invites', { user_id: 'u2' });
  });
});

describe('fetchPublishedPosts', () => {
  it('merges all returned posts into the index', async () => {
    posts.set({ byId: { existing: makePost('existing') } });
    getReq.mockResolvedValueOnce({ posts: [makePost('a'), makePost('b')] });
    await fetchPublishedPosts();
    expect(Object.keys(get(posts).byId).sort()).toEqual(['a', 'b', 'existing']);
  });
});
