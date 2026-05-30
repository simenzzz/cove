<script lang="ts">
  import { goto } from '$app/navigation';
  import { createDraft, createPost, postIdToString } from '$lib/stores/posts';
  import Button from '$components/ui/Button.svelte';

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

<section class="min-h-full px-4 py-10 sm:px-8">
  <form class="mx-auto flex w-full max-w-3xl flex-col gap-2" onsubmit={onSubmit}>
    <p class="mb-1 text-2xs font-semibold uppercase tracking-[0.18em] text-copper">New post</p>

    <label class="sr-only" for="post-title">Post title</label>
    <input
      id="post-title"
      bind:value={title}
      placeholder="Give this post a title"
      maxlength="200"
      autocomplete="off"
      disabled={submitting}
      class="w-full border-b border-line-strong bg-transparent pb-3 pt-1 font-display text-4xl font-bold leading-tight text-linen outline-none transition-colors placeholder:text-linen-muted focus:border-copper"
    />

    <label class="sr-only" for="post-body">Post body</label>
    <textarea
      id="post-body"
      bind:value={body}
      placeholder="Write the update, idea, or announcement…"
      rows="14"
      disabled={submitting}
      class="min-h-80 w-full resize-y bg-transparent py-2 text-lg leading-relaxed text-linen outline-none placeholder:text-linen-muted"
    ></textarea>

    <div
      class="flex flex-col items-stretch gap-3 border-t border-line pt-4 transition-all duration-150 sm:flex-row sm:items-center sm:justify-between {hasStarted
        ? 'pointer-events-auto opacity-100'
        : 'pointer-events-none opacity-0'}"
    >
      <div class="min-w-0 text-sm">
        {#if error}
          <p class="text-danger">{error}</p>
        {:else if bodyText}
          <p class="text-linen-muted">{bodyText.length} characters</p>
        {:else}
          <p class="text-linen-muted">Drafts can be published from the collaborative editor.</p>
        {/if}
      </div>

      <div class="flex shrink-0 items-center gap-2.5">
        <Button variant="outline" type="button" disabled={!canSaveDraft} onclick={saveDraft}>
          {mode === 'draft' ? 'Saving…' : 'Save draft'}
        </Button>
        <Button type="button" disabled={!canPost} onclick={publishNow}>
          {mode === 'post' ? 'Posting…' : 'Post'}
        </Button>
      </div>
    </div>
  </form>
</section>
