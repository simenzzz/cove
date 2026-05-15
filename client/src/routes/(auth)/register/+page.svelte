<script lang="ts">
  import { register } from '$stores/auth';
  import { goto } from '$app/navigation';

  let username = $state('');
  let displayName = $state('');
  let password = $state('');
  let confirmPassword = $state('');
  let error = $state('');
  let loading = $state(false);

  async function handleSubmit(e: Event) {
    e.preventDefault();
    error = '';

    if (password !== confirmPassword) {
      error = 'Passwords do not match';
      return;
    }
    if (password.length < 8) {
      error = 'Password must be at least 8 characters';
      return;
    }
    if (username.length < 3) {
      error = 'Username must be at least 3 characters';
      return;
    }

    loading = true;
    try {
      await register(username, displayName || username, password);
      goto('/');
    } catch (err) {
      error = err instanceof Error ? err.message : 'Registration failed';
    } finally {
      loading = false;
    }
  }
</script>

<div class="flex items-center justify-center min-h-screen">
  <div class="w-full max-w-md p-8 bg-gray-800 rounded-lg">
    <h1 class="text-2xl font-bold mb-6">Create your Nexus account</h1>

    {#if error}
      <div class="mb-4 p-3 bg-red-900/50 border border-red-700 rounded text-red-200 text-sm">
        {error}
      </div>
    {/if}

    <form onsubmit={handleSubmit} class="space-y-4">
      <input
        type="text"
        bind:value={username}
        placeholder="Username (3-32 chars, alphanumeric + underscore)"
        required
        class="w-full p-3 bg-gray-700 rounded focus:outline-none focus:ring-2 focus:ring-indigo-500"
      />
      <input
        type="text"
        bind:value={displayName}
        placeholder="Display Name"
        class="w-full p-3 bg-gray-700 rounded focus:outline-none focus:ring-2 focus:ring-indigo-500"
      />
      <input
        type="password"
        bind:value={password}
        placeholder="Password (8+ characters)"
        required
        class="w-full p-3 bg-gray-700 rounded focus:outline-none focus:ring-2 focus:ring-indigo-500"
      />
      <input
        type="password"
        bind:value={confirmPassword}
        placeholder="Confirm Password"
        required
        class="w-full p-3 bg-gray-700 rounded focus:outline-none focus:ring-2 focus:ring-indigo-500"
      />
      <button
        type="submit"
        disabled={loading}
        class="w-full p-3 bg-indigo-600 rounded font-semibold hover:bg-indigo-500 transition-colors disabled:opacity-50"
      >
        {loading ? 'Creating account...' : 'Register'}
      </button>
    </form>
    <p class="mt-4 text-sm text-gray-400">
      Already have an account?
      <a href="/login" class="text-indigo-400 hover:underline">Login</a>
    </p>
  </div>
</div>
