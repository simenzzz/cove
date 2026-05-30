<script lang="ts">
  interface Props {
    username: string;
    avatarUrl?: string | null;
    size?: 'sm' | 'md' | 'lg';
  }

  let { username, avatarUrl = null, size = 'md' }: Props = $props();

  const sizeClasses: Record<string, string> = {
    sm: 'w-8 h-8 text-xs',
    md: 'w-10 h-10 text-sm',
    lg: 'w-16 h-16 text-lg',
  };
</script>

{#if avatarUrl}
  <img
    src={avatarUrl}
    alt={username}
    class="rounded-2xl object-cover ring-1 ring-line {sizeClasses[size]}"
  />
{:else}
  <div
    class="flex items-center justify-center rounded-2xl bg-gradient-to-br from-copper to-copper-deep font-display font-semibold text-canvas ring-1 ring-copper-bright/30 {sizeClasses[
      size
    ]}"
  >
    {(username.charAt(0) || '?').toUpperCase()}
  </div>
{/if}
