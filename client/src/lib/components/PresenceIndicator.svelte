<script lang="ts">
  import type { UserStatus } from '$stores/presence';

  interface Props {
    status: UserStatus;
  }

  let { status }: Props = $props();

  const statusColors: Record<UserStatus, string> = {
    online: 'bg-success',
    idle: 'bg-warn',
    dnd: 'bg-danger',
    offline: 'bg-linen-faint',
  };
</script>

<span
  class="inline-block h-3 w-3 rounded-full border-2 border-surface {statusColors[status]}"
></span>
