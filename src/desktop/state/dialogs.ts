import type { ReactNode } from 'react';
import { create } from 'zustand';

export interface ConfirmOptions {
  title: string;
  description?: ReactNode;
  /** Extra content (lists, totals) shown in a scrollable box. */
  details?: ReactNode;
  confirmLabel?: string;
  cancelLabel?: string;
  tone?: 'default' | 'danger';
  /** The user must type this text exactly to enable the confirm button. */
  typed?: string;
}

interface ConfirmState {
  request: (ConfirmOptions & { resolve: (ok: boolean) => void }) | null;
  open: (o: ConfirmOptions) => Promise<boolean>;
  close: (ok: boolean) => void;
}

export const useConfirm = create<ConfirmState>((set, get) => ({
  request: null,
  open: (o) =>
    new Promise<boolean>((resolve) => {
      get().request?.resolve(false);
      set({ request: { ...o, resolve } });
    }),
  close: (ok) => {
    const r = get().request;
    set({ request: null });
    r?.resolve(ok);
  },
}));

/** Ask the user to confirm; resolves false when cancelled. */
export function confirm(o: ConfirmOptions): Promise<boolean> {
  return useConfirm.getState().open(o);
}

interface PaletteState {
  open: boolean;
  setOpen: (v: boolean) => void;
}

export const usePalette = create<PaletteState>((set) => ({ open: false, setOpen: (open) => set({ open }) }));
