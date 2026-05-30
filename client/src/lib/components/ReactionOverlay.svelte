<script lang="ts">
  import { REACTION_TTL_MS, sendReaction, type WatchReactionEvent } from '$stores/watch';

  let {
    channelId,
    reactions,
  }: { channelId: string; reactions: WatchReactionEvent[] } = $props();

  // Small curated palette. Free-text input is intentionally avoided here —
  // protocol caps emoji at 32 bytes server-side, so a picker is enough.
  const palette = ['❤️', '😂', '🔥', '👏', '😢', '😮', '🎉', '🤔'];

  // Deterministic horizontal placement keyed by local_id so a reaction stays
  // in the same column across re-renders (Svelte doesn't re-mount keyed list
  // items but we still want stable values).
  function offsetFor(id: string): number {
    let h = 0;
    for (let i = 0; i < id.length; i++) h = (h * 31 + id.charCodeAt(i)) | 0;
    return Math.abs(h) % 80; // 0-79% of width
  }
</script>

<div class="reaction-layer">
  {#each reactions as r (r.local_id)}
    <span
      class="floater"
      style:left="{offsetFor(r.local_id) + 10}%"
      style:animation-duration="{REACTION_TTL_MS}ms"
      title={r.username}
    >
      {r.emoji}
    </span>
  {/each}
</div>

<div class="reaction-picker">
  {#each palette as emoji}
    <button
      class="reaction-btn"
      onclick={() => sendReaction(channelId, emoji)}
      aria-label="React with {emoji}"
      title="React with {emoji}"
    >
      {emoji}
    </button>
  {/each}
</div>

<style>
  .reaction-layer {
    position: absolute;
    inset: 0;
    pointer-events: none;
    overflow: hidden;
  }
  .floater {
    position: absolute;
    bottom: 0;
    font-size: 2rem;
    animation-name: float-up;
    animation-timing-function: ease-out;
    animation-fill-mode: forwards;
  }
  @keyframes float-up {
    0% {
      transform: translateY(0) scale(0.6);
      opacity: 0;
    }
    10% {
      opacity: 1;
      transform: translateY(-10%) scale(1);
    }
    100% {
      transform: translateY(-100%) scale(1.2);
      opacity: 0;
    }
  }
  .reaction-picker {
    display: flex;
    gap: 0.3rem;
    padding: 0.35rem 0.5rem;
    background: color-mix(in oklab, var(--color-surface) 85%, transparent);
    backdrop-filter: blur(10px);
    border: 1px solid var(--color-line);
    border-radius: var(--radius-xl);
    align-self: flex-start;
  }
  .reaction-btn {
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-md);
    padding: 0.2rem 0.45rem;
    font-size: 1.1rem;
    cursor: pointer;
    transition:
      background 140ms ease,
      transform 140ms ease;
  }
  .reaction-btn:hover {
    background: var(--color-elevated);
    transform: scale(1.12);
  }
</style>
