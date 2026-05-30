<script lang="ts">
  import { fly } from 'svelte/transition';
  import { CheckCircle2, AlertTriangle, Info, X } from '@lucide/svelte';
  import { toasts, type ToastTone } from '$stores/toast';

  const tones: Record<ToastTone, { ring: string; icon: string }> = {
    success: { ring: 'border-l-success', icon: 'text-success' },
    error: { ring: 'border-l-danger', icon: 'text-danger' },
    info: { ring: 'border-l-teal', icon: 'text-teal-bright' },
  };
</script>

<div class="pointer-events-none fixed bottom-6 right-6 z-[60] flex w-80 flex-col gap-2.5">
  {#each $toasts as toast (toast.id)}
    <div
      class="glass pointer-events-auto flex items-start gap-3 rounded-xl border border-line-strong border-l-4 px-4 py-3 shadow-lift {tones[
        toast.tone
      ].ring}"
      transition:fly={{ x: 24, duration: 250 }}
      role="status"
    >
      <span class="mt-0.5 shrink-0 {tones[toast.tone].icon}">
        {#if toast.tone === 'success'}
          <CheckCircle2 size={18} />
        {:else if toast.tone === 'error'}
          <AlertTriangle size={18} />
        {:else}
          <Info size={18} />
        {/if}
      </span>
      <p class="flex-1 text-sm leading-snug text-linen">{toast.message}</p>
      <button
        class="-mr-1 rounded-md p-1 text-linen-muted transition-colors hover:text-linen"
        aria-label="Dismiss"
        onclick={() => toasts.dismiss(toast.id)}
      >
        <X size={15} />
      </button>
    </div>
  {/each}
</div>
