<script lang="ts">
  import { login } from '$stores/auth';
  import { goto } from '$app/navigation';

  let username = $state('');
  let password = $state('');
  let error = $state('');
  let loading = $state(false);

  async function handleSubmit(e: Event) {
    e.preventDefault();
    error = '';
    loading = true;
    try {
      await login(username, password);
      goto('/');
    } catch (err) {
      error = err instanceof Error ? err.message : 'Login failed';
    } finally {
      loading = false;
    }
  }
</script>

<div class="flex items-center justify-center min-h-screen">
  <div class="w-full max-w-md p-8 bg-gray-800 rounded-lg">
    <h1 class="text-2xl font-bold mb-6">Login to Nexus</h1>

    {#if error}
      <div class="mb-4 p-3 bg-red-900/50 border border-red-700 rounded text-red-200 text-sm">
        {error}
      </div>
    {/if}

    <form onsubmit={handleSubmit} class="space-y-4">
      <input
        type="text"
        bind:value={username}
        placeholder="Username"
        required
        class="w-full p-3 bg-gray-700 rounded focus:outline-none focus:ring-2 focus:ring-indigo-500"
      />
      <input
        type="password"
        bind:value={password}
        placeholder="Password"
        required
        class="w-full p-3 bg-gray-700 rounded focus:outline-none focus:ring-2 focus:ring-indigo-500"
      />
      <button
        type="submit"
        disabled={loading}
        class="w-full p-3 bg-indigo-600 rounded font-semibold hover:bg-indigo-500 transition-colors disabled:opacity-50"
      >
        {loading ? 'Logging in...' : 'Login'}
      </button>
    </form>
    <p class="mt-4 text-sm text-gray-400">
      Don't have an account?
      <a href="/register" class="text-indigo-400 hover:underline">Register</a>
    </p>
  </div>
</div>
