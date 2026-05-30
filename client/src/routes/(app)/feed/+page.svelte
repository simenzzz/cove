<script lang="ts">
  import PostCard from '$lib/components/PostCard.svelte';
  import SectionHeading from '$components/ui/SectionHeading.svelte';
  import Skeleton from '$components/ui/Skeleton.svelte';
  import Button from '$components/ui/Button.svelte';
  import { fetchPublishedPosts, postIdToString, type Post } from '$lib/stores/posts';
  import { PenLine, Newspaper } from '@lucide/svelte';

  let items = $state<Post[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  $effect(() => {
    loading = true;
    fetchPublishedPosts()
      .then((p) => {
        items = p;
        loading = false;
      })
      .catch((err) => {
        error = err instanceof Error ? err.message : String(err);
        loading = false;
      });
  });
</script>

<div class="mx-auto max-w-2xl px-4 py-8">
  <SectionHeading
    eyebrow="The latest"
    title="Your feed"
    subtitle="Fresh posts from across your coves."
  >
    {#snippet actions()}
      <Button href="/posts/new" size="sm">
        <PenLine size={15} /> Write
      </Button>
    {/snippet}
  </SectionHeading>

  <div class="mt-7">
    {#if loading}
      <div class="space-y-3">
        {#each Array(4) as _}
          <div class="rounded-2xl border border-line bg-surface p-5">
            <Skeleton class="h-5 w-2/3" />
            <Skeleton class="mt-3 h-3.5 w-full" />
            <Skeleton class="mt-2 h-3.5 w-4/5" />
          </div>
        {/each}
      </div>
    {:else if error}
      <div class="rounded-2xl border border-danger/40 bg-danger-soft p-4 text-sm text-danger">
        {error}
      </div>
    {:else if items.length === 0}
      <div class="flex flex-col items-center rounded-2xl border border-dashed border-line-strong bg-surface/40 px-6 py-14 text-center">
        <span class="mb-4 flex h-12 w-12 items-center justify-center rounded-2xl bg-copper-soft text-copper-bright">
          <Newspaper size={22} />
        </span>
        <p class="font-display text-lg font-semibold text-linen">It's quiet in here</p>
        <p class="mt-1 text-sm text-linen-muted">No published posts yet — be the first.</p>
        <Button href="/posts/new" class="mt-5">
          <PenLine size={15} /> Write a post
        </Button>
      </div>
    {:else}
      <ul class="flex flex-col gap-3">
        {#each items as post (postIdToString(post))}
          <li>
            <PostCard {post} />
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>
