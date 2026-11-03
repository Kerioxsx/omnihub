// While a Bleach theme is on: switch between Bankai, Hollow and Mugetsu from
// the sidebar, with one short diagonal slash (none with reduced motion).

import type { Look } from '@shared/types';
import { useState } from 'react';
import { createPortal } from 'react-dom';
import { cx } from '../lib/cx';
import { BLEACH } from '../lib/looks';
import { useSettings } from '../state/settings';

function reducedMotion(setting: boolean | undefined): boolean {
  return !!setting || window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

export function BleachSwitcher({ collapsed }: { collapsed?: boolean }) {
  const look = useSettings((s) => s.settings?.general.look) as Look | undefined;
  const reduced = useSettings((s) => s.settings?.general.reducedMotion);
  const update = useSettings((s) => s.update);
  const [flash, setFlash] = useState<{ mode: Look; key: number } | null>(null);
  const at = BLEACH.findIndex((m) => m.id === look);
  if (at < 0) return null;
  const go = (id: Look) => {
    if (id === look) return;
    const still = reducedMotion(reduced);
    if (!still) {
      const key = Date.now();
      setFlash({ mode: id, key });
      setTimeout(() => setFlash((f) => (f?.key === key ? null : f)), 600);
    }
    // The colours change under the slash.
    setTimeout(() => void update({ general: { look: id } }, { silent: true }), still ? 0 : 160);
  };
  const next = BLEACH[(at + 1) % BLEACH.length];
  return (
    <>
      {collapsed ? (
        <button type="button" onClick={() => go(next.id)} title={`Switch to ${next.name}`} aria-label={`Bleach theme: ${BLEACH[at].name}. Switch to ${next.name}`} className="mb-2 grid h-9 w-full place-items-center rounded-[10px] border border-line bg-surface text-[13px] font-bold text-accent hover:border-line-strong">
          {BLEACH[at].kanji.slice(0, 1)}
        </button>
      ) : (
        <div role="radiogroup" aria-label="Bleach theme" className="mb-2 grid grid-cols-3 gap-1 rounded-[10px] border border-line bg-surface p-1">
          {BLEACH.map((m) => (
            <button
              key={m.id}
              type="button"
              role="radio"
              aria-checked={m.id === look}
              onClick={() => go(m.id)}
              title={m.name}
              className={cx('flex flex-col items-center rounded-lg px-1 py-1 leading-tight transition-colors', m.id === look ? 'bg-accent-soft text-accent' : 'text-dim hover:bg-surface-2 hover:text-fg')}
            >
              <span className="text-[13px] font-bold">{m.kanji}</span>
              <span className="font-display text-[10.5px] font-semibold uppercase tracking-[0.08em]">{m.name}</span>
            </button>
          ))}
        </div>
      )}
      {flash && createPortal(<div key={flash.key} className="bleach-flash" data-mode={flash.mode} aria-hidden />, document.body)}
    </>
  );
}
