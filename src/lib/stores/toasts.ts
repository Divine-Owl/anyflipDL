import { writable, get } from 'svelte/store';

export type ToastVariant = 'info' | 'success' | 'warning' | 'error';

export interface Toast {
  id: string;
  variant: ToastVariant;
  title: string;
  detail?: string;
  action?: { label: string; onclick: () => void };
  duration: number;
}

export const toasts = writable<Toast[]>([]);

export function showToast(input: {
  variant?: ToastVariant;
  title: string;
  detail?: string;
  action?: { label: string; onclick: () => void };
  duration?: number;
}): string {
  const id = (typeof crypto !== 'undefined' && 'randomUUID' in crypto)
    ? crypto.randomUUID()
    : `t-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
  const variant: ToastVariant = input.variant ?? 'info';
  const duration = input.duration ?? (variant === 'error' ? 0 : 4000);
  const toast: Toast = {
    id,
    variant,
    title: input.title,
    detail: input.detail,
    action: input.action,
    duration,
  };
  toasts.update((list) => {
    const next = [...list, toast];
    return next.length > 3 ? next.slice(next.length - 3) : next;
  });
  if (duration > 0) {
    setTimeout(() => dismissToast(id), duration);
  }
  return id;
}

export function dismissToast(id: string): void {
  toasts.update((list) => list.filter((t) => t.id !== id));
}

export function clearToasts(): void {
  toasts.set([]);
}
