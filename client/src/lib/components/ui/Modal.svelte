<script lang="ts">
  import type { Snippet } from 'svelte';
  import { fade, scale } from 'svelte/transition';
  import { X } from '@lucide/svelte';
  import IconButton from './IconButton.svelte';

  interface Props {
    open?: boolean;
    title?: string;
    size?: 'sm' | 'md' | 'lg';
    onclose?: () => void;
    children: Snippet;
    footer?: Snippet;
  }

  let {
    open = $bindable(false),
    title,
    size = 'md',
    onclose,
    children,
    footer,
  }: Props = $props();

  const widths = { sm: 'max-w-sm', md: 'max-w-lg', lg: 'max-w-2xl' };

  let dialogEl: HTMLDivElement | undefined = $state();
  let restoreFocus: HTMLElement | null = null;

  // Focus management: when the dialog opens, remember what was focused, move
  // focus into the dialog, and restore it on close.
  $effect(() => {
    if (open) {
      restoreFocus = document.activeElement as HTMLElement | null;
      queueMicrotask(() => dialogEl?.focus());
    } else if (restoreFocus) {
      restoreFocus.focus();
      restoreFocus = null;
    }
  });

  function close() {
    open = false;
    onclose?.();
  }

  function focusables(): HTMLElement[] {
    if (!dialogEl) return [];
    return Array.from(
      dialogEl.querySelectorAll<HTMLElement>(
        'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
      ),
    );
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      close();
      return;
    }
    // Trap Tab within the dialog so focus can't escape behind the scrim.
    if (e.key === 'Tab') {
      const items = focusables();
      if (items.length === 0) {
        e.preventDefault();
        dialogEl?.focus();
        return;
      }
      const first = items[0];
      const last = items[items.length - 1];
      const active = document.activeElement;
      if (e.shiftKey && active === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && active === last) {
        e.preventDefault();
        first.focus();
      }
    }
  }
</script>

<svelte:window on:keydown={open ? onkeydown : undefined} />

{#if open}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4">
    <button
      class="absolute inset-0 cursor-default bg-overlay/80 backdrop-blur-sm"
      aria-label="Close dialog"
      onclick={close}
      transition:fade={{ duration: 180 }}
    ></button>

    <div
      bind:this={dialogEl}
      role="dialog"
      aria-modal="true"
      aria-label={title}
      tabindex="-1"
      class="relative w-full {widths[size]} overflow-hidden rounded-3xl border border-line-strong bg-surface shadow-lift focus:outline-none"
      transition:scale={{ duration: 220, start: 0.96 }}
    >
      {#if title}
        <header class="flex items-center justify-between border-b border-line px-6 py-4">
          <h3 class="font-display text-lg font-semibold text-linen">{title}</h3>
          <IconButton variant="ghost" size="sm" label="Close" onclick={close}>
            <X size={18} />
          </IconButton>
        </header>
      {/if}

      <div class="px-6 py-5">
        {@render children()}
      </div>

      {#if footer}
        <footer class="flex justify-end gap-3 border-t border-line bg-elevated/40 px-6 py-4">
          {@render footer()}
        </footer>
      {/if}
    </div>
  </div>
{/if}
