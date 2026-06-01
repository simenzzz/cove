<script lang="ts">
  import { postIdToString, type Post } from '$lib/stores/posts';
  import { ArrowUpRight } from '@lucide/svelte';

  let { post }: { post: Post } = $props();

  function preview(content: string | null): string {
    if (!content) return '';
    return content.length > 240 ? content.slice(0, 240) + '…' : content;
  }
</script>

<a
  href={`/posts/${postIdToString(post)}`}
  class="group block rounded-2xl border border-line bg-surface p-5 transition-all duration-200 ease-out-soft hover:border-copper/60 hover:shadow-lift"
>
  <div class="flex items-start justify-between gap-3">
    <h2 class="font-display text-lg font-semibold text-linen transition-colors group-hover:text-copper-bright">
      {post.title}
    </h2>
    <ArrowUpRight
      size={18}
      class="mt-1 shrink-0 text-linen-muted transition-all duration-200 group-hover:-translate-y-0.5 group-hover:translate-x-0.5 group-hover:text-copper-bright"
    />
  </div>
  {#if post.author_display_name || post.author_username}
    <p class="mt-1 text-xs text-linen-muted">
      {post.author_display_name || post.author_username}
    </p>
  {/if}
  {#if post.published_content}
    <p class="mt-2 whitespace-pre-wrap text-sm leading-relaxed text-linen-dim line-clamp-3">
      {preview(post.published_content)}
    </p>
  {/if}
</a>
