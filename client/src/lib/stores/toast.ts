import { writable } from 'svelte/store';

export type ToastTone = 'info' | 'success' | 'error';

export interface Toast {
  id: number;
  message: string;
  tone: ToastTone;
}

function createToastStore() {
  const { subscribe, update } = writable<Toast[]>([]);
  let nextId = 0;

  function push(message: string, tone: ToastTone = 'info', timeout = 4000): number {
    const id = ++nextId;
    // Immutable update: append a new array rather than mutating in place.
    update((current) => [...current, { id, message, tone }]);
    if (timeout > 0) {
      setTimeout(() => dismiss(id), timeout);
    }
    return id;
  }

  function dismiss(id: number): void {
    update((current) => current.filter((toast) => toast.id !== id));
  }

  return {
    subscribe,
    dismiss,
    push,
    info: (message: string, timeout?: number) => push(message, 'info', timeout),
    success: (message: string, timeout?: number) => push(message, 'success', timeout),
    error: (message: string, timeout?: number) => push(message, 'error', timeout),
  };
}

export const toasts = createToastStore();
