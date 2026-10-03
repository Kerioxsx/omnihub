import type { LucideIcon } from 'lucide-react';
import { Camera, CornerDownLeft, Crop, HardDrive, Lightbulb, LockKeyhole, Moon, NotebookPen, QrCode, Search, ShieldCheck, Smartphone, Sun } from 'lucide-react';
import { motion } from 'motion/react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { api, errorText } from '../api';
import { cx } from '../lib/cx';
import { navigate } from '../lib/router';
import { MOD_LABEL } from '../lib/util';
import { usePalette } from '../state/dialogs';
import { useLive } from '../state/live';
import { useSettings } from '../state/settings';
import { useStorage } from '../state/storage';
import { toast } from '../state/toasts';
import { formatBytes } from '@shared/format';
import { NAV } from './nav';
import { Kbd } from './ui/Card';
import { Modal } from './ui/Overlay';

interface Cmd {
  id: string;
  label: string;
  hint?: string;
  icon: LucideIcon;
  group: 'Pages' | 'Actions' | 'Drives';
  keywords?: string;
  shortcut?: string;
  run: () => void | Promise<void>;
}

function score(q: string, text: string): number {
  if (!q) return 1;
  const t = text.toLowerCase();
  if (t.startsWith(q)) return 4;
  if (t.includes(` ${q}`)) return 3;
  if (t.includes(q)) return 2;
  let i = 0;
  for (const ch of t) if (ch === q[i]) i++;
  return i === q.length ? 1 : 0;
}

async function attempt(label: string, fn: () => Promise<unknown>) {
  try {
    await fn();
  } catch (e) {
    toast.error(label, errorText(e));
  }
}

function useCommands(): Cmd[] {
  const volumes = useStorage((s) => s.volumes);
  const settings = useSettings((s) => s.settings);
  const remote = useLive((s) => s.remote);
  return useMemo(() => {
    const pages: Cmd[] = NAV.map((n, i) => ({ id: `page-${n.id}`, label: n.label, hint: n.description, icon: n.icon, group: 'Pages', shortcut: `${MOD_LABEL} ${i + 1}`, run: () => navigate(n.id) }));
    const light = settings?.general.theme === 'light';
    const actions: Cmd[] = [
      { id: 'shot-region', label: 'Capture a region', hint: 'Freeze the screen and drag a rectangle', icon: Crop, group: 'Actions', keywords: 'screenshot snip', run: () => attempt('Could not start the capture', () => api.shots.regionBegin()) },
      {
        id: 'shot-screen',
        label: 'Take a screenshot',
        hint: 'Whole screen under the cursor',
        icon: Camera,
        group: 'Actions',
        keywords: 'capture',
        run: () => attempt('Screenshot failed', () => api.shots.capture('screen', 0)),
      },
      { id: 'new-note', label: 'New note', icon: NotebookPen, group: 'Actions', keywords: 'write', run: () => navigate('notes', { new: 'note' }) },
      { id: 'new-idea', label: 'New idea for Claude', icon: Lightbulb, group: 'Actions', keywords: 'claude prompt', run: () => navigate('notes', { new: 'idea' }) },
      { id: 'pair', label: 'Pair a phone', hint: remote?.running ? 'Show the QR code and PIN' : 'Turns on the companion first', icon: QrCode, group: 'Actions', keywords: 'qr connect', run: () => navigate('phone', { pair: '1' }) },
      {
        id: 'companion',
        label: remote?.running ? 'Turn off the phone companion' : 'Turn on the phone companion',
        icon: Smartphone,
        group: 'Actions',
        keywords: 'server remote',
        run: () => void useSettings.getState().update({ remote: { enabled: !remote?.running } }),
      },
      {
        id: 'lock',
        label: 'Lock the vault',
        icon: LockKeyhole,
        group: 'Actions',
        keywords: 'password',
        run: () =>
          attempt('Could not lock', async () => {
            await api.vault.lock();
            toast.info('Vault locked');
          }),
      },
      { id: 'theme', label: light ? 'Switch to dark theme' : 'Switch to light theme', icon: light ? Moon : Sun, group: 'Actions', keywords: 'appearance', run: () => void useSettings.getState().update({ general: { theme: light ? 'dark' : 'light' } }) },
      { id: 'admin', label: 'Restart as administrator', hint: 'Only needed for some cleanup items', icon: ShieldCheck, group: 'Actions', keywords: 'elevate uac', run: () => attempt('Could not restart', () => api.app.restartElevated()) },
    ];
    const drives: Cmd[] = volumes.map((v) => ({
      id: `scan-${v.root}`,
      label: `Scan ${v.root.replace(/\\$/, '')} ${v.label ? `(${v.label})` : ''}`,
      hint: `${formatBytes(v.total - v.free)} used of ${formatBytes(v.total)}${v.mftCapable ? ' · fast MFT scan' : ''}`,
      icon: HardDrive,
      group: 'Drives',
      keywords: 'storage disk analyze',
      run: () => {
        navigate('storage');
        void useStorage.getState().scan(v.root, v.mftCapable ? 'fast' : 'standard');
      },
    }));
    return [...pages, ...actions, ...drives];
  }, [volumes, settings?.general.theme, remote?.running]);
}

export function CommandPalette() {
  const open = usePalette((s) => s.open);
  const setOpen = usePalette((s) => s.setOpen);
  const commands = useCommands();
  const [q, setQ] = useState('');
  const [active, setActive] = useState(0);
  const listRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (open) {
      setQ('');
      setActive(0);
    }
  }, [open]);

  const results = useMemo(() => {
    const query = q.trim().toLowerCase();
    return commands
      .map((c) => ({ c, s: Math.max(score(query, c.label), score(query, `${c.keywords ?? ''} ${c.hint ?? ''}`) * 0.5) }))
      .filter((x) => x.s > 0)
      .sort((a, b) => (query ? b.s - a.s : 0))
      .map((x) => x.c);
  }, [commands, q]);

  useEffect(() => setActive(0), [q]);
  useEffect(() => {
    listRef.current?.querySelector(`[data-index="${active}"]`)?.scrollIntoView({ block: 'nearest' });
  }, [active]);

  const run = (c: Cmd | undefined) => {
    if (!c) return;
    setOpen(false);
    void c.run();
  };

  let lastGroup = '';
  return (
    <Modal open={open} onClose={() => setOpen(false)} bare className="mt-[10vh] max-w-[620px] self-start" labelledBy="palette-input">
      <div className="overflow-hidden rounded-[20px] border border-line-strong bg-elev shadow-[0_40px_100px_-20px_rgba(0,0,0,0.7)]">
        <div className="flex items-center gap-3 border-b border-line px-4">
          <Search size={18} className="text-faint" aria-hidden />
          <input
            id="palette-input"
            data-autofocus
            value={q}
            onChange={(e) => setQ(e.target.value)}
            placeholder="Jump to a page or run an action…"
            className="h-14 flex-1 bg-transparent text-[15px] text-fg outline-none placeholder:text-faint"
            aria-label="Command"
            role="combobox"
            aria-expanded
            aria-controls="palette-list"
            aria-activedescendant={results[active] ? `cmd-${results[active].id}` : undefined}
            onKeyDown={(e) => {
              if (e.key === 'ArrowDown') {
                e.preventDefault();
                setActive((a) => Math.min(results.length - 1, a + 1));
              } else if (e.key === 'ArrowUp') {
                e.preventDefault();
                setActive((a) => Math.max(0, a - 1));
              } else if (e.key === 'Enter') {
                e.preventDefault();
                run(results[active]);
              }
            }}
          />
          <Kbd>Esc</Kbd>
        </div>
        <div ref={listRef} id="palette-list" role="listbox" className="max-h-[52vh] overflow-y-auto p-2">
          {results.length === 0 && <div className="px-3 py-10 text-center text-[13px] text-faint">Nothing matches “{q}”.</div>}
          {results.map((c, i) => {
            const header = c.group !== lastGroup ? c.group : null;
            lastGroup = c.group;
            const Icon = c.icon;
            return (
              <div key={c.id}>
                {header && <div className="px-3 pb-1 pt-2.5 text-[10.5px] font-semibold uppercase tracking-[0.1em] text-faint">{header}</div>}
                <button
                  id={`cmd-${c.id}`}
                  type="button"
                  role="option"
                  aria-selected={i === active}
                  data-index={i}
                  onMouseMove={() => setActive(i)}
                  onClick={() => run(c)}
                  className={cx('relative flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-left transition-colors', i === active ? 'text-fg' : 'text-dim')}
                >
                  {i === active && <motion.span layoutId="palette-active" transition={{ type: 'spring', stiffness: 600, damping: 45 }} className="absolute inset-0 rounded-xl bg-surface-3" />}
                  <span className={cx('relative flex h-8 w-8 shrink-0 items-center justify-center rounded-lg border border-line', i === active ? 'bg-accent-soft text-accent' : 'bg-surface text-faint')}>
                    <Icon size={16} aria-hidden />
                  </span>
                  <span className="relative min-w-0 flex-1">
                    <span className="block truncate text-[13.5px] font-medium">{c.label}</span>
                    {c.hint && <span className="block truncate text-[12px] text-faint">{c.hint}</span>}
                  </span>
                  {c.shortcut && <Kbd className="relative">{c.shortcut}</Kbd>}
                  {i === active && <CornerDownLeft size={14} className="relative text-faint" aria-hidden />}
                </button>
              </div>
            );
          })}
        </div>
      </div>
    </Modal>
  );
}
