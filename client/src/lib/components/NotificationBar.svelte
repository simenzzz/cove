<script lang="ts">
  import { goto } from '$app/navigation';
  import {
    Bell,
    Check,
    Hash,
    LogIn,
    MessageCircle,
    UserCheck,
    UserMinus,
    UserPlus,
    X,
  } from '@lucide/svelte';
  import { notifications, type AppNotification, type NotificationKind } from '$stores/notifications';

  let { open = $bindable(false) }: { open?: boolean } = $props();
  let autoClose: ReturnType<typeof setTimeout> | null = null;

  const visible = $derived($notifications.items.slice(0, 4));

  const icons = {
    friend_request: UserPlus,
    friend_accepted: UserCheck,
    channel_message: MessageCircle,
    direct_message: MessageCircle,
    server_member_joined: UserPlus,
    server_member_left: UserMinus,
    server_joined: LogIn,
    channel_created: Hash,
    system: Bell,
  } satisfies Record<NotificationKind, typeof Bell>;

  function toggle() {
    open = !open;
    if (open) notifications.markAllRead();
  }

  async function activate(note: AppNotification) {
    notifications.markRead(note.id);
    open = false;
    if (note.href) await goto(note.href);
  }

  function dismiss(event: MouseEvent, id: number) {
    event.stopPropagation();
    notifications.dismiss(id);
  }

  $effect(() => {
    if ($notifications.lastPushedId == null) return;
    open = true;
    if (autoClose) clearTimeout(autoClose);
    autoClose = setTimeout(() => {
      open = false;
    }, 5200);
  });
</script>

<svelte:window
  onkeydown={(event) => {
    if (event.key === 'Escape') open = false;
  }}
/>

<div class="relative">
  {#if open}
    <button
      type="button"
      aria-label="Close notifications"
      class="fixed inset-0 z-10 cursor-default"
      onclick={() => (open = false)}
    ></button>
  {/if}

  <button
    type="button"
    aria-label="Notifications"
    aria-expanded={open}
    onclick={toggle}
    class="group relative flex h-12 w-12 items-center justify-center rounded-2xl bg-elevated text-linen-dim ring-1 ring-line transition-all duration-200 hover:rounded-xl hover:text-linen hover:ring-teal/60 {open
      ? 'rounded-xl text-teal-bright shadow-glow-teal ring-teal/50'
      : ''}"
  >
    <Bell size={20} />
    {#if $notifications.unread > 0}
      <span
        class="absolute -right-1 -top-1 flex h-5 min-w-5 items-center justify-center rounded-full bg-copper px-1 text-2xs font-bold text-canvas ring-2 ring-surface"
      >
        {Math.min($notifications.unread, 9)}
      </span>
    {/if}
  </button>

  {#if open}
    <div
      class="glass absolute bottom-0 left-16 z-20 w-80 rounded-xl border border-line-strong p-2 shadow-lift"
    >
      <div class="flex items-center justify-between border-b border-line px-2 pb-2 pt-1">
        <div>
          <p class="font-display text-sm font-semibold text-linen">Notifications</p>
          <p class="text-xs text-linen-muted">
            {$notifications.items.length === 0
              ? 'All quiet'
              : `${$notifications.items.length} recent`}
          </p>
        </div>
        {#if $notifications.items.length > 0}
          <button
            type="button"
            class="rounded-md p-1.5 text-linen-muted transition-colors hover:bg-elevated hover:text-linen"
            aria-label="Mark all read"
            title="Mark all read"
            onclick={() => notifications.markAllRead()}
          >
            <Check size={15} />
          </button>
        {/if}
      </div>

      {#if visible.length === 0}
        <p class="px-3 py-6 text-center text-sm text-linen-muted">No notifications yet.</p>
      {:else}
        <div class="mt-2 flex max-h-96 flex-col gap-1.5 overflow-y-auto">
          {#each visible as note (note.id)}
            {@const Icon = icons[note.kind]}
            <div
              role="button"
              tabindex="0"
              class="group/item flex w-full items-start gap-2.5 rounded-lg px-2.5 py-2.5 text-left transition-colors hover:bg-elevated"
              onclick={() => activate(note)}
              onkeydown={(event) => {
                if (event.key === 'Enter' || event.key === ' ') {
                  event.preventDefault();
                  activate(note);
                }
              }}
            >
              <span
                class="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-lg {note.read
                  ? 'bg-elevated text-linen-muted'
                  : 'bg-teal-soft text-teal-bright'}"
              >
                <Icon size={16} />
              </span>
              <span class="min-w-0 flex-1">
                <span class="block truncate text-sm font-semibold text-linen">{note.title}</span>
                <span class="mt-0.5 line-clamp-2 block text-xs leading-snug text-linen-muted">
                  {note.body}
                </span>
              </span>
              <span
                class="mt-0.5 h-2 w-2 shrink-0 rounded-full {note.read
                  ? 'bg-transparent'
                  : 'bg-copper'}"
              ></span>
              <button
                type="button"
                aria-label="Dismiss notification"
                class="rounded-md p-1 text-linen-faint opacity-0 transition-all hover:bg-canvas/60 hover:text-linen group-hover/item:opacity-100"
                onclick={(event) => dismiss(event, note.id)}
              >
                <X size={14} />
              </button>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>
