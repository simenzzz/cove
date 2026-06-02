<script lang="ts">
  import Modal from './Modal.svelte';
  import Button from './Button.svelte';
  import Input from './Input.svelte';
  import Textarea from './Textarea.svelte';

  interface Props {
    open?: boolean;
    title: string;
    label: string;
    value?: string;
    placeholder?: string;
    submitLabel?: string;
    cancelLabel?: string;
    multiline?: boolean;
    required?: boolean;
    onsubmit: (value: string) => void | Promise<void>;
    oncancel?: () => void;
  }

  let {
    open = $bindable(false),
    title,
    label,
    value = '',
    placeholder = '',
    submitLabel = 'Save',
    cancelLabel = 'Cancel',
    multiline = false,
    required = false,
    onsubmit,
    oncancel,
  }: Props = $props();

  let draft = $state('');

  $effect(() => {
    if (open) draft = value;
  });

  function close() {
    open = false;
    oncancel?.();
  }

  function submit(e: Event) {
    e.preventDefault();
    if (required && !draft.trim()) return;
    onsubmit(draft);
    open = false;
  }
</script>

<Modal bind:open {title} size="sm" onclose={oncancel}>
  <form onsubmit={submit} class="space-y-4">
    <label class="space-y-1.5">
      <span class="text-xs font-medium text-linen-dim">{label}</span>
      {#if multiline}
        <Textarea bind:value={draft} {placeholder} rows={4} required={required} />
      {:else}
        <Input bind:value={draft} {placeholder} required={required} />
      {/if}
    </label>

    <div class="flex justify-end gap-2 border-t border-line pt-4">
      <Button type="button" variant="ghost" onclick={close}>{cancelLabel}</Button>
      <Button type="submit" disabled={required && !draft.trim()}>{submitLabel}</Button>
    </div>
  </form>
</Modal>
