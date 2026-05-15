<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { CollabProvider } from '$lib/collab/provider';
  import CollaboratorCursors from './CollaboratorCursors.svelte';

  let { postId }: { postId: string } = $props();

  let provider: CollabProvider | null = $state(null);
  let content = $state('');
  let peers = $state<Record<string, unknown>>({});
  let saving = $state(false);

  let updateHandler: (() => void) | null = null;

  onMount(() => {
    provider = new CollabProvider(postId);

    // Mirror Y.Text → local state so the <textarea> stays bound.
    const handler = () => {
      if (provider) content = provider.text.toString();
    };
    provider.text.observe(handler);
    updateHandler = handler;
    // Pull initial value (in case state arrived before observer attached).
    content = provider.text.toString();

    const offAw = provider.onAwareness((users) => {
      peers = users;
    });

    return () => offAw();
  });

  onDestroy(() => {
    if (provider && updateHandler) {
      provider.text.unobserve(updateHandler);
    }
    provider?.destroy();
  });

  function onInput(e: Event) {
    if (!provider) return;
    const target = e.target as HTMLTextAreaElement;
    saving = true;
    // Naive replace — fine for kickoff; richer diff/patch later.
    provider.replaceText(target.value);
    // Lower the "saving" indicator after a short tick.
    setTimeout(() => (saving = false), 500);
  }

  function onSelect(e: Event) {
    if (!provider) return;
    const target = e.target as HTMLTextAreaElement;
    provider.sendAwareness({
      cursor: target.selectionStart,
      selection_end: target.selectionEnd,
    });
  }
</script>

<div class="collab-editor">
  <CollaboratorCursors {peers} />

  <textarea
    class="editor"
    value={content}
    oninput={onInput}
    onselect={onSelect}
    placeholder="Start writing..."
    rows="20"
  ></textarea>

  {#if saving}
    <div class="status">Saving...</div>
  {/if}
</div>

<style>
  .collab-editor {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .editor {
    width: 100%;
    padding: 0.75rem;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.95rem;
    line-height: 1.5;
    border: 1px solid var(--color-border, #ccc);
    border-radius: 0.375rem;
    resize: vertical;
  }

  .status {
    font-size: 0.75rem;
    color: var(--color-muted, #888);
  }
</style>
