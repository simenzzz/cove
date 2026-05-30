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

<div class="flex flex-col gap-2">
  <CollaboratorCursors {peers} />

  <textarea
    class="w-full resize-y rounded-2xl border border-line-strong bg-surface p-4 font-mono text-sm leading-relaxed text-linen outline-none transition-colors placeholder:text-linen-muted focus:border-copper"
    aria-label="Document content"
    value={content}
    oninput={onInput}
    onselect={onSelect}
    placeholder="Start writing…"
    rows="20"
  ></textarea>

  {#if saving}
    <div class="text-2xs text-linen-muted">Saving…</div>
  {/if}
</div>
