<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { auth } from '$stores/auth';

  onMount(() => {
    // Wait for auth to finish loading, then redirect exactly once.
    // Note: never call `unsubscribe()` from inside this callback — the store
    // fires synchronously during `.subscribe()`, so on an already-resolved
    // session the handle is still in its temporal dead zone. Let onMount's
    // returned cleanup tear the subscription down on unmount instead.
    let navigated = false;
    const unsubscribe = auth.subscribe((state) => {
      if (state.loading || navigated) return;
      navigated = true;
      goto(state.accessToken ? '/feed' : '/login');
    });
    return unsubscribe;
  });
</script>

<div class="flex min-h-screen flex-col items-center justify-center gap-4">
  <div
    class="h-11 w-11 animate-pulse-glow rounded-2xl bg-gradient-to-br from-copper-bright to-copper-deep"
  ></div>
  <p class="text-sm tracking-wide text-linen-muted">Finding your coves…</p>
</div>
