<script lang="ts">
  import { goto } from '$app/navigation';
  import { createDraft, createPost, postIdToString } from '$lib/stores/posts';
  import Button from '$components/ui/Button.svelte';
  import { Save, Send } from '@lucide/svelte';

  let title = $state('');
  let body = $state('');
  let submitting = $state(false);
  let mode = $state<'draft' | 'post' | null>(null);
  let error = $state<string | null>(null);

  const titleText = $derived(title.trim());
  const bodyText = $derived(body.trim());
  const hasStarted = $derived(Boolean(titleText || bodyText));
  const canSaveDraft = $derived(Boolean(titleText) && !submitting);
  const canPost = $derived(Boolean(titleText && bodyText) && !submitting);

  async function saveDraft() {
    if (!titleText) {
      error = 'Title is required';
      return;
    }
    submitting = true;
    mode = 'draft';
    error = null;
    try {
      const post = await createDraft(titleText, bodyText || undefined);
      const id = postIdToString(post);
      await goto(`/posts/${id}/edit`);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      submitting = false;
      mode = null;
    }
  }

  async function publishNow() {
    if (!titleText || !bodyText) {
      error = titleText ? 'Write something before posting' : 'Title is required';
      return;
    }
    submitting = true;
    mode = 'post';
    error = null;
    try {
      const post = await createPost({ title: titleText, content: bodyText, publish: true });
      const id = postIdToString(post);
      await goto(`/posts/${id}`);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      submitting = false;
      mode = null;
    }
  }

  function onSubmit(e: Event) {
    e.preventDefault();
    if (canPost) {
      publishNow();
    } else {
      saveDraft();
    }
  }
</script>

<section class="relative min-h-full overflow-hidden px-4 py-8 sm:px-8 sm:py-12">
  <form
    class="group relative mx-auto flex min-h-[min(680px,calc(100vh-8rem))] w-full max-w-4xl flex-col overflow-hidden rounded-2xl border border-line-strong/70 bg-surface/80 text-linen shadow-lift backdrop-blur-xl before:absolute before:inset-x-6 before:top-0 before:h-px before:bg-gradient-to-r before:from-transparent before:via-copper/70 before:to-transparent sm:before:inset-x-10"
    onsubmit={onSubmit}
  >
    <div class="border-b border-line/80 bg-elevated/30 px-5 py-5 sm:px-8 sm:py-6">
      <div class="mb-5 flex items-center justify-between gap-4">
        <p class="text-2xs font-semibold uppercase tracking-[0.22em] text-copper-bright">
          New post
        </p>
        <p class="rounded-full border border-line bg-canvas/35 px-3 py-1 text-xs text-linen-muted">
          {bodyText.length} chars
        </p>
      </div>

      <label class="sr-only" for="post-title">Post title</label>
      <input
        id="post-title"
        bind:value={title}
        placeholder="Give this post a title"
        maxlength="200"
        autocomplete="off"
        disabled={submitting}
        class="w-full border-b border-line-strong/80 bg-transparent pb-4 font-display text-3xl font-bold leading-tight text-linen outline-none transition-colors placeholder:text-linen-faint focus:border-copper-bright sm:text-5xl"
      />
    </div>

    <div class="flex min-h-0 flex-1 bg-canvas/20 px-5 py-5 sm:px-8 sm:py-6">
      <label class="sr-only" for="post-body">Post body</label>
      <textarea
        id="post-body"
        bind:value={body}
        placeholder="Write the update, idea, or announcement…"
        rows="14"
        disabled={submitting}
        class="min-h-80 w-full flex-1 resize-none bg-transparent text-lg leading-8 text-linen outline-none transition-colors placeholder:text-linen-muted focus:placeholder:text-linen-faint sm:min-h-[26rem]"
      ></textarea>
    </div>

    <div
      class="flex flex-col items-stretch gap-3 border-t border-line bg-elevated/45 px-5 py-4 transition-all duration-200 sm:flex-row sm:items-center sm:justify-between sm:px-8 {hasStarted
        ? 'pointer-events-auto opacity-100'
        : 'pointer-events-none opacity-0'}"
    >
      <div class="min-w-0 text-sm">
        {#if error}
          <p class="text-danger">{error}</p>
        {:else if bodyText}
          <p class="text-linen-muted">{bodyText.length} characters</p>
        {:else}
          <p class="text-linen-muted">Ready to save</p>
        {/if}
      </div>

      <div class="flex shrink-0 items-center gap-2.5">
        <Button
          variant="outline"
          type="button"
          disabled={!canSaveDraft}
          onclick={saveDraft}
          class="border-line-strong bg-canvas/20 text-linen-dim hover:border-copper hover:bg-copper-soft/50 hover:text-linen"
        >
          <Save class="h-4 w-4" aria-hidden="true" />
          {mode === 'draft' ? 'Saving…' : 'Save draft'}
        </Button>
        <Button type="button" disabled={!canPost} onclick={publishNow}>
          <Send class="h-4 w-4" aria-hidden="true" />
          {mode === 'post' ? 'Posting…' : 'Post'}
        </Button>
      </div>
    </div>
  </form>
</section>
