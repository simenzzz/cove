<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { page } from '$app/state';
  import ChannelList from '$components/ChannelList.svelte';
  import CollaboratorCursors from '$components/CollaboratorCursors.svelte';
  import { ChannelDocProvider } from '$lib/collab/channel-doc-provider';
  import { fetchChannels } from '$stores/channels';
  import { Feather } from '@lucide/svelte';

  const serverId = $derived(page.params.serverId ?? '');
  const channelId = $derived(page.params.channelId ?? '');

  let provider: ChannelDocProvider | null = $state(null);
  let content = $state('');
  let peers = $state<Record<string, unknown>>({});
  let updateHandler: (() => void) | null = null;

  onMount(async () => {
    if (!serverId || !channelId) return;
    await fetchChannels(serverId);
    provider = new ChannelDocProvider(channelId);
    const handler = () => {
      if (provider) content = provider.text.toString();
    };
    provider.text.observe(handler);
    updateHandler = handler;
    content = provider.text.toString();
    provider.onAwareness((users) => {
      peers = users;
    });
  });

  onDestroy(() => {
    if (provider && updateHandler) provider.text.unobserve(updateHandler);
    provider?.destroy();
  });

  function onInput(e: Event) {
    const target = e.target as HTMLTextAreaElement;
    provider?.replaceText(target.value);
  }

  function onSelect(e: Event) {
    const target = e.target as HTMLTextAreaElement;
    provider?.sendAwareness({
      cursor: target.selectionStart,
      selection_end: target.selectionEnd,
    });
  }
</script>

<div class="flex h-full">
  <ChannelList {serverId} />
  <div class="min-w-0 flex-1 p-5">
    <div class="flex h-full flex-col gap-4">
      <div class="flex items-center justify-between gap-3 border-b border-line pb-4">
        <div class="flex items-center gap-2.5">
          <span class="flex h-9 w-9 items-center justify-center rounded-xl bg-copper-soft text-copper-bright">
            <Feather size={17} />
          </span>
          <h1 class="font-display text-lg font-semibold text-linen">Collaborative document</h1>
        </div>
        <CollaboratorCursors {peers} />
      </div>
      <textarea
        class="min-h-0 w-full flex-1 resize-none rounded-2xl border border-line-strong bg-surface p-4 text-sm leading-relaxed text-linen outline-none transition-colors placeholder:text-linen-muted focus:border-copper"
        aria-label="Document content"
        value={content}
        oninput={onInput}
        onselect={onSelect}
        placeholder="Start writing…"
      ></textarea>
    </div>
  </div>
</div>
