import * as Y from 'yjs';
import { wsClient, type WsMessage } from '../ws/client';

const TEXT_ROOT = 'content';

/**
 * Binds a Y.Doc to the project's WS bridge for one post. Sends local updates
 * to the server, applies remote updates from peers, and exposes awareness
 * state via a callback.
 */
export class CollabProvider {
  readonly doc: Y.Doc;
  readonly text: Y.Text;
  private readonly postId: string;
  private readonly cleanups: Array<() => void> = [];
  private suppressNextOrigin: symbol | null = null;
  private readonly remoteOrigin = Symbol('remote');
  private awarenessHandlers: Array<(users: Record<string, unknown>) => void> = [];
  private peerCount = 0;
  private peerCountHandlers: Array<(count: number) => void> = [];

  constructor(postId: string) {
    this.postId = postId;
    this.doc = new Y.Doc();
    this.text = this.doc.getText(TEXT_ROOT);

    // Send local updates over WS.
    const onLocalUpdate = (update: Uint8Array, origin: unknown) => {
      if (origin === this.remoteOrigin) return;
      wsClient.send({
        v: 1,
        type: 'collab_update',
        post_id: this.postId,
        update_b64: bytesToBase64(update),
      });
    };
    this.doc.on('update', onLocalUpdate);
    this.cleanups.push(() => this.doc.off('update', onLocalUpdate));

    this.cleanups.push(
      wsClient.on('collab_state', (msg: WsMessage) => {
        if (msg.post_id !== this.postId) return;
        const state = base64ToBytes(msg.state_b64 as string);
        Y.applyUpdate(this.doc, state, this.remoteOrigin);
      }),
    );

    this.cleanups.push(
      wsClient.on('collab_update', (msg: WsMessage) => {
        if (msg.post_id !== this.postId) return;
        const update = base64ToBytes(msg.update_b64 as string);
        Y.applyUpdate(this.doc, update, this.remoteOrigin);
      }),
    );

    this.cleanups.push(
      wsClient.on('awareness_state', (msg: WsMessage) => {
        if (msg.post_id !== this.postId) return;
        const users = (msg.users as Record<string, unknown>) ?? {};
        this.peerCount = Object.keys(users).length;
        for (const h of this.awarenessHandlers) h(users);
        for (const h of this.peerCountHandlers) h(this.peerCount);
      }),
    );

    this.cleanups.push(
      wsClient.on('collab_error', (msg: WsMessage) => {
        if (msg.post_id !== this.postId) return;
        console.error('[collab]', msg.code, msg.message);
      }),
    );

    // Subscribe so the server sends collab_state.
    wsClient.send({ v: 1, type: 'collab_subscribe', post_id: postId });
  }

  /** Replace the entire text contents in a single transaction. */
  replaceText(next: string): void {
    this.doc.transact(() => {
      this.text.delete(0, this.text.length);
      this.text.insert(0, next);
    });
  }

  /** Broadcast an opaque awareness blob (cursor pos, etc.). */
  sendAwareness(state: Record<string, unknown>): void {
    wsClient.send({
      v: 1,
      type: 'awareness_update',
      post_id: this.postId,
      state,
    });
  }

  onAwareness(handler: (users: Record<string, unknown>) => void): () => void {
    this.awarenessHandlers.push(handler);
    return () => {
      this.awarenessHandlers = this.awarenessHandlers.filter((h) => h !== handler);
    };
  }

  onPeerCount(handler: (count: number) => void): () => void {
    this.peerCountHandlers.push(handler);
    handler(this.peerCount);
    return () => {
      this.peerCountHandlers = this.peerCountHandlers.filter((h) => h !== handler);
    };
  }

  destroy(): void {
    wsClient.send({ v: 1, type: 'collab_unsubscribe', post_id: this.postId });
    for (const cleanup of this.cleanups) cleanup();
    this.doc.destroy();
  }
}

function bytesToBase64(bytes: Uint8Array): string {
  let s = '';
  for (let i = 0; i < bytes.byteLength; i++) {
    s += String.fromCharCode(bytes[i]);
  }
  return btoa(s);
}

function base64ToBytes(b64: string): Uint8Array {
  const s = atob(b64);
  const bytes = new Uint8Array(s.length);
  for (let i = 0; i < s.length; i++) bytes[i] = s.charCodeAt(i);
  return bytes;
}
