// The pop-up windows a hotkey opens at the mouse: the quick-launch wheel
// (#/wheel, see-through) and the clipboard history (#/clipboard). They hide
// when they lose focus, on Esc, and after a pick.

import type { QuickState } from '@shared/types';
import { ClipboardList, Search } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { api, inTauri } from '../../api';
import { cx } from '../../lib/cx';
import { useEvent } from '../../lib/hooks';
import { ClipRow, Wheel, useClipboard } from './QuickPage';

function useHideOnBlur(which: 'wheel' | 'clipboard', onShown: () => void) {
  const shown = useRef(onShown);
  shown.current = onShown;
  useEffect(() => {
    document.documentElement.classList.add(which === 'wheel' ? 'ambient-route' : 'popout-route');
    const hide = () => void api.quick.hide(which);
    const onKey = (e: KeyboardEvent) => e.key === 'Escape' && hide();
    window.addEventListener('keydown', onKey);
    window.addEventListener('blur', hide);
    return () => {
      window.removeEventListener('keydown', onKey);
      window.removeEventListener('blur', hide);
    };
  }, [which]);
  useEvent('popup:shown', () => shown.current());
}

export function WheelPopup() {
  const [state, setState] = useState<QuickState | null>(null);
  const [active, setActive] = useState(0);
  const load = () => void api.quick.state().then(setState);
  useEffect(load, []);
  useEvent<QuickState>('quick:changed', setState);
  useHideOnBlur('wheel', () => {
    setActive(0);
    load();
  });
  const pick = async (i: number) => {
    const b = state?.wheel[i];
    if (!b?.action) return;
    // Errors come back as a Windows notification (this window has no room for them).
    await api.quick.run(b.action, true).catch(() => undefined);
  };
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const n = state?.wheel.length ?? 0;
      if (/^[1-8]$/.test(e.key) && Number(e.key) <= n) void pick(Number(e.key) - 1);
      else if (e.key === 'Enter') void pick(active);
      else if (e.key === 'ArrowRight' || e.key === 'ArrowDown' || e.key === 'Tab') {
        e.preventDefault();
        setActive((a) => (a + 1) % Math.max(n, 1));
      } else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') setActive((a) => (a - 1 + Math.max(n, 1)) % Math.max(n, 1));
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });
  // Point the mouse in a direction to pick that slot.
  const onMove = (e: React.MouseEvent) => {
    const n = state?.wheel.length ?? 0;
    if (!n) return;
    const r = e.currentTarget.getBoundingClientRect();
    const dx = e.clientX - (r.left + r.width / 2);
    const dy = e.clientY - (r.top + r.height / 2);
    if (Math.hypot(dx, dy) < 40) return;
    const a = (Math.atan2(dy, dx) + Math.PI / 2 + Math.PI * 2) % (Math.PI * 2);
    setActive(Math.round((a / (Math.PI * 2)) * n) % n);
  };
  if (!state) return null;
  return (
    <div className={cx('grid h-screen w-screen place-items-center overflow-hidden', !inTauri && 'bg-[#0b0b12]')} onMouseMove={onMove}>
      <div className="animate-[wheel-in_160ms_ease-out]">
        {state.wheel.length ? (
          <Wheel buttons={state.wheel} active={active} onPick={(i) => void pick(i)} onHover={setActive} size={420} />
        ) : (
          <div className="rounded-2xl bg-black/70 px-5 py-4 text-sm text-white">Add slots in OmniHub → Quick actions → Wheel</div>
        )}
      </div>
    </div>
  );
}

export function ClipboardPopup() {
  const { list, reload } = useClipboard();
  const [q, setQ] = useState('');
  const [sel, setSel] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  useHideOnBlur('clipboard', () => {
    setQ('');
    setSel(0);
    reload();
    setTimeout(() => input.current?.focus(), 30);
  });
  const items = useMemo(() => {
    const n = q.trim().toLowerCase();
    const all = list?.items ?? [];
    const hits = n ? all.filter((i) => i.text.toLowerCase().includes(n)) : all;
    return [...hits.filter((i) => i.pinned), ...hits.filter((i) => !i.pinned)].slice(0, 60);
  }, [list, q]);
  // A failure comes back as a Windows notification.
  const paste = (id: string) => void api.clipboard.paste(id).catch(() => undefined);
  useEffect(() => {
    input.current?.focus();
  }, []);
  const onKey = (e: React.KeyboardEvent) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSel((s) => Math.min(s + 1, items.length - 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSel((s) => Math.max(s - 1, 0));
    } else if (e.key === 'Enter' && items[sel]) paste(items[sel].id);
  };
  return (
    <div className="flex h-screen w-screen flex-col overflow-hidden border border-line-strong bg-[var(--bg)] text-fg">
      <div className="flex items-center gap-2 border-b border-line px-3 py-2.5" data-tauri-drag-region>
        <ClipboardList size={15} className="text-accent" aria-hidden />
        <span className="text-[13px] font-semibold" data-tauri-drag-region>
          Clipboard
        </span>
        {list?.paused && <span className="rounded bg-warn/15 px-1.5 text-[11px] text-warn">Paused</span>}
      </div>
      <label className="flex items-center gap-2 border-b border-line px-3 py-2">
        <Search size={14} className="text-faint" aria-hidden />
        <input
          ref={input}
          value={q}
          onChange={(e) => {
            setQ(e.target.value);
            setSel(0);
          }}
          onKeyDown={onKey}
          placeholder="Search… (Enter pastes)"
          aria-label="Search the clipboard history"
          className="flex-1 bg-transparent text-[13px] outline-none placeholder:text-faint"
        />
      </label>
      <div className="flex-1 space-y-0.5 overflow-y-auto p-1">
        {items.map((i, n) => (
          <div key={i.id} className={cx('rounded-xl', n === sel && 'bg-accent-soft')} onMouseEnter={() => setSel(n)}>
            <ClipRow item={i} compact onPaste={() => paste(i.id)} />
          </div>
        ))}
        {!items.length && <div className="p-6 text-center text-[13px] text-faint">{q ? 'Nothing matches' : 'Nothing copied yet'}</div>}
      </div>
    </div>
  );
}
