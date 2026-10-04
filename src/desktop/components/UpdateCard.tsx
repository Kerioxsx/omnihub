// "OmniHub x.y.z is ready" in the sidebar, with one-click install, and the
// shared hook the Settings → About rows use.

import type { UpdateInfo, UpdateState } from '@shared/types';
import { AnimatePresence, motion } from 'motion/react';
import { ArrowDownToLine, LoaderCircle } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../api';
import { useEvent } from '../lib/hooks';
import { toast } from '../state/toasts';

export function useUpdate() {
  const [info, setInfo] = useState<UpdateInfo | null>(null);
  useEffect(() => {
    void api.update.state().then(setInfo, () => undefined);
  }, []);
  useEvent<UpdateState>('update:state', (state) => setInfo((i) => (i ? { ...i, state } : i)));
  const check = async () => {
    try {
      setInfo(await api.update.check());
    } catch (e) {
      toast.error('Could not check for updates', errorText(e));
    }
  };
  const install = async () => {
    try {
      await api.update.install();
    } catch (e) {
      toast.error('The update did not install', errorText(e));
    }
  };
  return { info, check, install };
}

export function updateText(s: UpdateState): string {
  switch (s.state) {
    case 'idle':
      return 'Not checked yet';
    case 'checking':
      return 'Checking…';
    case 'upToDate':
      return 'You have the latest version';
    case 'available':
      return `Version ${s.release.version} is available`;
    case 'downloading':
      return `Downloading ${s.version}… ${s.total ? Math.round((s.done / s.total) * 100) : 0}%`;
    case 'installing':
      return `Installing ${s.version} — OmniHub restarts in a moment`;
    case 'failed':
      return s.message;
  }
}

/** Sidebar card, shown while an update is available or installing. */
export function UpdateCard({ collapsed }: { collapsed?: boolean }) {
  const { info, install } = useUpdate();
  const s = info?.state;
  const show = s && (s.state === 'available' || s.state === 'downloading' || s.state === 'installing');
  const busy = s?.state === 'downloading' || s?.state === 'installing';
  const pct = s?.state === 'downloading' && s.total ? (s.done / s.total) * 100 : s?.state === 'installing' ? 100 : 0;
  return (
    <AnimatePresence>
      {show && s && (
        <motion.button
          type="button"
          initial={{ opacity: 0, y: 6 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: 6 }}
          disabled={busy}
          onClick={() => void install()}
          title={updateText(s)}
          className={collapsed ? 'mb-2 grid h-9 w-full place-items-center rounded-[10px] bg-accent-soft text-accent' : 'relative mb-2 w-full overflow-hidden rounded-[10px] border border-accent/40 bg-accent-soft px-3 py-2 text-left transition-colors hover:bg-accent/20 disabled:cursor-default'}
        >
          {collapsed ? (
            busy ? <LoaderCircle size={16} className="animate-spin" /> : <ArrowDownToLine size={16} />
          ) : (
            <>
              <div className="flex items-center gap-2 text-[12.5px] font-semibold text-accent">
                {busy ? <LoaderCircle size={14} className="animate-spin" /> : <ArrowDownToLine size={14} />}
                {s.state === 'available' ? `Update to ${s.release.version}` : s.state === 'downloading' ? `Downloading ${s.version}…` : `Installing ${s.version}…`}
              </div>
              <div className="mt-0.5 text-[11px] text-dim">{s.state === 'available' ? 'One click — your settings and data stay' : s.state === 'downloading' ? `${Math.round(pct)}%` : 'OmniHub restarts in a moment'}</div>
              {busy && <div className="absolute inset-x-0 bottom-0 h-0.5 bg-accent transition-[width]" style={{ width: `${pct}%` }} />}
            </>
          )}
        </motion.button>
      )}
    </AnimatePresence>
  );
}
