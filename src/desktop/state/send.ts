// The "send to phone" dialog, openable from anywhere: the Phone page, file
// menus, dropping files on the window, or Explorer's Send to menu.

import { create } from 'zustand';

export type SendMode = 'files' | 'text';

interface SendState {
  open: boolean;
  mode: SendMode;
  paths: string[];
  text: string;
  openFiles: (paths?: string[]) => void;
  openText: (text?: string) => void;
  setPaths: (paths: string[]) => void;
  setText: (text: string) => void;
  setMode: (mode: SendMode) => void;
  close: () => void;
}

export const useSend = create<SendState>((set, get) => ({
  open: false,
  mode: 'files',
  paths: [],
  text: '',
  openFiles: (paths = []) => set({ open: true, mode: 'files', paths: get().open ? [...new Set([...get().paths, ...paths])] : paths }),
  openText: (text = '') => set({ open: true, mode: 'text', text }),
  setPaths: (paths) => set({ paths }),
  setText: (text) => set({ text }),
  setMode: (mode) => set({ mode }),
  close: () => set({ open: false }),
}));
