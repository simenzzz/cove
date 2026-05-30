<script lang="ts">
  import { page } from '$app/stores';
  import CollabEditor from '$lib/components/CollabEditor.svelte';
  import CollabInvite from '$lib/components/CollabInvite.svelte';
  import { fetchPost, publishPost, type Post } from '$lib/stores/posts';
  import Badge from '$components/ui/Badge.svelte';
  import Button from '$components/ui/Button.svelte';
  import Skeleton from '$components/ui/Skeleton.svelte';
  import { Send } from '@lucide/svelte';

  let postId = $derived($page.params.postId ?? '');
  let post = $state<Post | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let publishing = $state(false);

  $effect(() => {
    if (!postId) return;
    loading = true;
    fetchPost(postId)
      .then((p) => {
        post = p;
        loading = false;
      })
      .catch((err) => {
        error = err instanceof Error ? err.message : String(err);
        loading = false;
      });
  });

  async function onPublish() {
    if (!post) return;
    publishing = true;
    try {
      post = await publishPost(postId);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      publishing = false;
    }
  }
</script>

<div class="mx-auto max-w-2xl px-4 py-10">
  {#if loading}
    <Skeleton class="h-9 w-3/4" />
    <Skeleton class="mt-6 h-40 w-full" rounded="rounded-2xl" />
  {:else if error}
    <div class="rounded-2xl border border-danger/40 bg-danger-soft p-4 text-sm text-danger">
      {error}
    </div>
  {:else if post}
    <header class="mb-5 flex items-center gap-3">
      <h1 class="flex-1 font-display text-3xl font-bold leading-tight text-linen">{post.title}</h1>
      {#if post.published}
        <Badge tone="success">Published</Badge>
      {:else}
        <Button onclick={onPublish} disabled={publishing}>
          <Send size={15} />
          {publishing ? 'Publishing…' : 'Publish'}
        </Button>
      {/if}
    </header>

    {#if post.published && post.published_content !== null}
      <article
        class="whitespace-pre-wrap font-sans text-base leading-relaxed text-linen-dim"
      >{post.published_content}</article>
    {:else}
      <CollabInvite {postId} />
      <CollabEditor {postId} />
    {/if}
  {/if}
</div>
