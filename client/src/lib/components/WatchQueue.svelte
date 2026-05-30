<script lang="ts">
  import {
    sendQueueAdd,
    sendQueueRemove,
    sendSkip,
    sendVote,
    parseYouTubeId,
    type WatchQueueItem,
  } from '$stores/watch';

  let {
    channelId,
    queue,
    currentUserId,
    isLeader,
    hasCurrentVideo,
  }: {
    channelId: string;
    queue: WatchQueueItem[];
    currentUserId: string;
    isLeader: boolean;
    hasCurrentVideo: boolean;
  } = $props();

  let urlInput = $state('');
  let titleInput = $state('');
  let addError = $state('');

  function onAdd(e: SubmitEvent): void {
    e.preventDefault();
    addError = '';
    const videoId = parseYouTubeId(urlInput);
    if (!videoId) {
      addError = 'Could not parse a YouTube id from that URL';
      return;
    }
    const title = titleInput.trim() || `Video ${videoId}`;
    sendQueueAdd(channelId, {
      video_id: videoId,
      title,
      // We don't know the real duration yet — it'll be filled in by the
      // server later. 0 is a sentinel for "unknown".
      duration_ms: 0,
      thumbnail_url: `https://i.ytimg.com/vi/${videoId}/mqdefault.jpg`,
    });
    urlInput = '';
    titleInput = '';
  }

  function canRemove(item: WatchQueueItem): boolean {
    if (item.pending) return true; // optimistic — local-only
    return isLeader || item.added_by === currentUserId;
  }

  function onVote(item: WatchQueueItem, value: -1 | 1): void {
    if (item.pending || !item.id) return;
    // Toggle: clicking the active vote clears it.
    const next = (item.my_vote ?? 0) === value ? 0 : value;
    sendVote(channelId, item.id, next as -1 | 0 | 1);
  }
</script>

<aside class="queue-pane">
  <div class="queue-header">
    <p class="text-2xs font-semibold uppercase tracking-[0.16em] text-linen-muted">
      Queue ({queue.length})
    </p>
    {#if isLeader && hasCurrentVideo}
      <button
        class="rounded-lg border border-line-strong px-2 py-1 text-xs text-linen-dim transition-colors hover:bg-elevated hover:text-linen"
        onclick={() => sendSkip(channelId)}
        title="Skip the current video"
      >
        Skip ▶
      </button>
    {/if}
  </div>

  <form class="add-form" onsubmit={onAdd}>
    <input
      type="text"
      bind:value={urlInput}
      aria-label="YouTube URL or id"
      placeholder="YouTube URL or id"
      class="add-input"
      required
    />
    <input
      type="text"
      bind:value={titleInput}
      aria-label="Video title (optional)"
      placeholder="Title (optional)"
      class="add-input"
    />
    <button type="submit" class="add-button">Add</button>
  </form>
  {#if addError}
    <p class="mt-1 text-xs text-danger">{addError}</p>
  {/if}

  <ul class="queue-list">
    {#each queue as item (item.id)}
      <li class="queue-item" class:pending={item.pending}>
        {#if item.thumbnail_url}
          <img
            src={item.thumbnail_url}
            alt=""
            class="thumb"
            loading="lazy"
          />
        {/if}
        <div class="meta">
          <p class="title">{item.title}</p>
          <p class="sub">
            {item.pending ? 'pending…' : `score ${item.score}`}
          </p>
        </div>
        <div class="actions">
          <button
            class="vote-btn"
            class:active={item.my_vote === 1}
            disabled={item.pending}
            onclick={() => onVote(item, 1)}
            title="Upvote"
          >
            ▲
          </button>
          <button
            class="vote-btn"
            class:active={item.my_vote === -1}
            disabled={item.pending}
            onclick={() => onVote(item, -1)}
            title="Downvote"
          >
            ▼
          </button>
          {#if canRemove(item)}
            <button
              class="remove-btn"
              onclick={() => sendQueueRemove(channelId, item.id)}
              title="Remove"
            >
              ✕
            </button>
          {/if}
        </div>
      </li>
    {:else}
      <li class="empty">Queue is empty.</li>
    {/each}
  </ul>
</aside>

<style>
  .queue-pane {
    width: 320px;
    border-left: 1px solid var(--color-line);
    padding: 0.85rem;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .queue-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .add-form {
    display: grid;
    grid-template-columns: 1fr;
    gap: 0.35rem;
  }
  .add-input {
    background: var(--color-elevated);
    border: 1px solid var(--color-line-strong);
    border-radius: var(--radius-lg);
    padding: 0.4rem 0.6rem;
    color: var(--color-linen);
    font-size: 0.85rem;
  }
  .add-input:focus {
    outline: none;
    border-color: var(--color-copper);
  }
  .add-button {
    background: var(--color-copper);
    color: var(--color-canvas);
    border-radius: var(--radius-lg);
    padding: 0.4rem 0.6rem;
    font-size: 0.85rem;
    font-weight: 600;
  }
  .add-button:hover {
    background: var(--color-copper-bright);
  }
  .queue-list {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    margin-top: 0.4rem;
  }
  .queue-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem;
    background: var(--color-surface);
    border: 1px solid var(--color-line);
    border-radius: var(--radius-lg);
  }
  .queue-item.pending {
    opacity: 0.6;
  }
  .thumb {
    width: 64px;
    height: 36px;
    object-fit: cover;
    border-radius: var(--radius-sm);
  }
  .meta {
    flex: 1;
    min-width: 0;
  }
  .title {
    font-size: 0.85rem;
    color: var(--color-linen);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sub {
    font-size: 0.7rem;
    color: var(--color-linen-muted);
  }
  .actions {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .vote-btn,
  .remove-btn {
    background: transparent;
    color: var(--color-linen-muted);
    border: 1px solid var(--color-line-strong);
    border-radius: var(--radius-sm);
    width: 24px;
    height: 20px;
    font-size: 0.7rem;
    line-height: 1;
    cursor: pointer;
    transition:
      background 140ms ease,
      color 140ms ease;
  }
  .vote-btn:hover:not(:disabled),
  .remove-btn:hover {
    background: var(--color-elevated);
    color: var(--color-linen);
  }
  .vote-btn.active {
    background: var(--color-copper);
    color: var(--color-canvas);
    border-color: var(--color-copper);
  }
  .vote-btn:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }
  .empty {
    color: var(--color-linen-muted);
    font-size: 0.8rem;
    text-align: center;
    padding: 1rem 0;
  }
</style>
