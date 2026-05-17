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
    gap: 0.25rem;
    padding: 0.25rem 0.5rem;
    background: rgba(17, 24, 39, 0.85);
    border-radius: 0.375rem;
    align-self: flex-start;
  }
  .reaction-btn {
    background: transparent;
    border: 1px solid rgb(55, 65, 81);
    border-radius: 0.25rem;
    padding: 0.125rem 0.375rem;
    font-size: 1.1rem;
    cursor: pointer;
  }
  .reaction-btn:hover {
    background: rgb(55, 65, 81);
  }
</style>
