<script lang="ts">
  import { goto } from '$app/navigation';
  import { createDraft, postIdToString } from '$lib/stores/posts';

  let title = $state('');
  let submitting = $state(false);
  let error = $state<string | null>(null);

  async function onSubmit(e: Event) {
    e.preventDefault();
    if (!title.trim()) {
      error = 'Title is required';
      return;
    }
    submitting = true;
    error = null;
    try {
      const post = await createDraft(title.trim());
      const id = postIdToString(post);
      await goto(`/posts/${id}/edit`);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      submitting = false;
    }
  }
</script>

<div class="container">
  <h1>New post</h1>
  <form onsubmit={onSubmit}>
    <label>
      Title
      <input bind:value={title} placeholder="My new post" maxlength="200" />
    </label>
    {#if error}<p class="error">{error}</p>{/if}
    <button type="submit" disabled={submitting}>
      {submitting ? 'Creating...' : 'Create draft'}
    </button>
  </form>
</div>

<style>
  .container {
    max-width: 32rem;
    margin: 2rem auto;
    padding: 0 1rem;
  }
  form {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.85rem;
  }
  input {
    padding: 0.5rem;
    font-size: 1rem;
    border: 1px solid #ccc;
    border-radius: 0.25rem;
  }
  .error {
    color: #e11d48;
    font-size: 0.85rem;
  }
  button {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 0.25rem;
    background: #2563eb;
    color: white;
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>
