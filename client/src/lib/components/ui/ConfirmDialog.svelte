<script lang="ts">
  import type { Snippet } from 'svelte';
  import Modal from './Modal.svelte';
  import Button from './Button.svelte';

  interface Props {
    open?: boolean;
    title: string;
    confirmLabel?: string;
    cancelLabel?: string;
    danger?: boolean;
    onconfirm: () => void | Promise<void>;
    oncancel?: () => void;
    children: Snippet;
  }

  let {
    open = $bindable(false),
    title,
    confirmLabel = 'Confirm',
    cancelLabel = 'Cancel',
    danger = false,
    onconfirm,
    oncancel,
    children,
  }: Props = $props();

  function close() {
    open = false;
    oncancel?.();
  }

  function submitConfirmation() {
    onconfirm();
    open = false;
  }
</script>

<Modal bind:open {title} size="sm" onclose={oncancel}>
  <div class="space-y-4">
    <div class="text-sm leading-relaxed text-linen-dim">
      {@render children()}
    </div>

    <div class="flex justify-end gap-2 border-t border-line pt-4">
      <Button type="button" variant="ghost" onclick={close}>{cancelLabel}</Button>
      <Button type="button" variant={danger ? 'danger' : 'primary'} onclick={submitConfirmation}>
        {confirmLabel}
      </Button>
    </div>
  </div>
</Modal>
