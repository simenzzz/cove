<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { fetchPost, type Post } from '$lib/stores/posts';
  import Badge from '$components/ui/Badge.svelte';
  import Skeleton from '$components/ui/Skeleton.svelte';
  import { ArrowLeft } from '@lucide/svelte';

  let postId = $derived($page.params.postId ?? '');
  let post = $state<Post | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  $effect(() => {
    if (!postId) return;
    loading = true;
    fetchPost(postId)
      .then((p) => {
        // Drafts shouldn't render here — bounce to the editor (the API
        // already gated read access, so reaching this means we're allowed).
        if (!p.published) {
          goto(`/posts/${postId}/edit`, { replaceState: true });
          return;
        }
        post = p;
        loading = false;
      })
      .catch((err) => {
        error = err instanceof Error ? err.message : String(err);
        loading = false;
      });
  });
</script>

<div class="mx-auto max-w-2xl px-4 py-10">
  <a
    href="/feed"
    class="mb-6 inline-flex items-center gap-1.5 text-sm text-linen-muted transition-colors hover:text-linen"
  >
    <ArrowLeft size={15} /> Back to feed
  </a>

  {#if loading}
    <Skeleton class="h-9 w-3/4" />
    <Skeleton class="mt-5 h-4 w-full" />
    <Skeleton class="mt-2.5 h-4 w-full" />
    <Skeleton class="mt-2.5 h-4 w-2/3" />
  {:else if error}
    <div class="rounded-2xl border border-danger/40 bg-danger-soft p-4 text-sm text-danger">
      {error}
    </div>
  {:else if post}
    <header class="flex items-start gap-3">
      <h1 class="flex-1 font-display text-3xl font-bold leading-tight text-linen">{post.title}</h1>
      <Badge tone="success">Published</Badge>
    </header>
    {#if post.published_content !== null}
      <article
        class="mt-6 whitespace-pre-wrap font-sans text-base leading-relaxed text-linen-dim"
      >{post.published_content}</article>
    {/if}
  {/if}
</div>
