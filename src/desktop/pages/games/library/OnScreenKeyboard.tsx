// An on-screen keyboard for controllers: move with the stick or d-pad,
// press A (or Nintendo's confirm) to type, B to delete, Start to finish.
// A real keyboard works too.

import { Delete, CornerDownLeft, Space } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { cx } from '../../../lib/cx';
import { usePadInput } from '../../../lib/pads';

const ROWS = ['1234567890', 'qwertyuiop', 'asdfghjkl\'', 'zxcvbnm-:.'];
const SPECIAL = ['space', 'delete', 'done'] as const;

export function OnScreenKeyboard({ value, onChange, onDone, title = 'Search' }: { value: string; onChange: (v: string) => void; onDone: () => void; title?: string }) {
  const [at, setAt] = useState<[number, number]>([1, 0]);
  const rows = [...ROWS.map((r) => r.split('')), [...SPECIAL]];
  const ref = useRef({ value, at });
  ref.current = { value, at };
  const press = (key: string) => {
    if (key === 'done') return onDone();
    if (key === 'delete') return onChange(ref.current.value.slice(0, -1));
    onChange((ref.current.value + (key === 'space' ? ' ' : key)).slice(0, 40));
  };
  usePadInput(
    true,
    {
      move: (dx, dy) =>
        setAt(([r, c]) => {
          const nr = Math.min(rows.length - 1, Math.max(0, r + dy));
          const nc = Math.min(rows[nr].length - 1, Math.max(0, (nr !== r ? Math.round((c / Math.max(1, rows[r].length - 1)) * (rows[nr].length - 1)) : c) + dx));
          return [nr, nc];
        }),
      ok: () => press(rows[ref.current.at[0]][ref.current.at[1]]),
      back: () => (ref.current.value ? press('delete') : onDone()),
      start: onDone,
      menu: () => press('space'),
      select: onDone,
      bumper: (d) => (d < 0 ? press('delete') : press('space')),
    },
    { haptics: true },
  );
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Enter' || e.key === 'Escape') {
        e.preventDefault();
        onDone();
      } else if (e.key === 'Backspace') {
        e.preventDefault();
        press('delete');
      } else if (e.key.length === 1) {
        e.preventDefault();
        press(e.key === ' ' ? 'space' : e.key.toLowerCase());
      }
    };
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  return (
    <div className="fixed inset-x-0 bottom-0 z-[70] bg-gradient-to-t from-black via-black/95 to-transparent px-6 pb-8 pt-16">
      <div className="mx-auto max-w-[860px]">
        <div className="mb-4 flex items-center gap-3">
          <span className="text-[13px] uppercase tracking-wider text-white/50">{title}</span>
          <div className="min-h-[52px] flex-1 rounded-2xl border border-white/15 bg-white/10 px-5 py-3 font-display text-[24px] text-white">
            {value}
            <span className="ml-0.5 inline-block h-6 w-0.5 animate-pulse bg-white align-middle" />
          </div>
        </div>
        <div className="space-y-2">
          {rows.map((r, ri) => (
            <div key={ri} className="flex justify-center gap-2">
              {r.map((k, ci) => {
                const on = at[0] === ri && at[1] === ci;
                const wide = ri === rows.length - 1;
                return (
                  <button
                    key={k}
                    type="button"
                    onClick={() => press(k)}
                    className={cx('grid h-14 place-items-center rounded-xl text-[20px] font-semibold uppercase transition-transform', wide ? 'w-44' : 'w-14', on ? 'scale-110 bg-white text-black shadow-[0_0_24px_rgba(255,255,255,0.45)]' : 'bg-white/10 text-white')}
                  >
                    {k === 'space' ? <Space size={22} /> : k === 'delete' ? <Delete size={22} /> : k === 'done' ? <CornerDownLeft size={22} /> : k}
                  </button>
                );
              })}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
