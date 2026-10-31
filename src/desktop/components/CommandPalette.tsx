import type { LucideIcon } from 'lucide-react';
import { AppWindow, Calculator, Camera, CornerDownLeft, Crop, File, Folder, Gauge, HardDrive, Image, KeyRound, Lightbulb, LockKeyhole, Moon, Music2, NotebookPen, QrCode, Search, ShieldCheck, Smartphone, StickyNote, Sun } from 'lucide-react';
import { motion } from 'motion/react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { api, errorText } from '../api';
import { cx } from '../lib/cx';
import { navigate } from '../lib/router';
import { quickAnswers } from '../lib/quickTools';
import { copyText, MOD_LABEL } from '../lib/util';
import { usePalette } from '../state/dialogs';
import { useLive } from '../state/live';
import { useSettings } from '../state/settings';
import { useStorage } from '../state/storage';
import { toast } from '../state/toasts';
import { formatBytes } from '@shared/format';
import { NAV } from './nav';
import { usePlayer } from '../lib/nowPlaying';
import { Kbd } from './ui/Card';
import { Modal } from './ui/Overlay';

interface Cmd {
  id: string;
  label: string;
  hint?: string;
  icon: LucideIcon;
  group: 'Answer' | 'Pages' | 'Actions' | 'Drives' | 'Files' | 'Notes' | 'Apps' | 'Screenshots' | 'Vault';
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
    const pages: Cmd[] = NAV.map((n, i) => ({ id: `page-${n.id}`, label: n.label, hint: n.description, icon: n.icon, group: 'Pages', shortcut: i < 9 ? `${MOD_LABEL} ${i + 1}` : undefined, run: () => navigate(n.id) }));
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
      { id: 'player', label: 'Full-screen lyrics', hint: 'What the PC plays, Apple Music style', icon: Music2, group: 'Actions', keywords: 'music song player now playing', run: () => usePlayer.getState().show() },
      { id: 'optimize-pc', label: 'Optimize this PC for games', hint: 'Refresh rate, power plan, Game DVR and more', icon: Gauge, group: 'Actions', keywords: 'fps boost performance', run: () => navigate('games', { tab: 'pc' }) },
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

/** Everything else that matches: files (from the latest scan of each drive), notes, apps, screenshots and vault titles. */
function useEverything(query: string, open: boolean): Cmd[] {
  const [found, setFound] = useState<Cmd[]>([]);
  const vaultOpen = !!useLive((s) => s.vault?.unlocked);
  const apps = useRef<Awaited<ReturnType<typeof api.apps.list>> | null>(null);
  useEffect(() => {
    if (!open) apps.current = null;
  }, [open]);
  useEffect(() => {
    const q = query.trim();
    if (!open || q.length < 2) {
      setFound([]);
      return;
    }
    let alive = true;
    const t = setTimeout(async () => {
      const low = q.toLowerCase();
      const out: Cmd[] = [];
      const safe = async <T,>(p: Promise<T>): Promise<T | null> => p.catch(() => null);
      const [scans, notes, shots, vault, appList] = await Promise.all([
        safe(api.storage.scans()),
        safe(api.notes.list({ query: q, limit: 5 })),
        safe(api.shots.list({ query: q, limit: 5 })),
        vaultOpen ? safe(api.vault.list()) : Promise.resolve(null),
        apps.current ? Promise.resolve(apps.current) : safe(api.apps.list()),
      ]);
      if (appList) apps.current = appList;
      // The newest scan of each drive.
      const latest = new Map<string, NonNullable<typeof scans>[number]>();
      for (const s of scans ?? []) if (!latest.has(s.root) || latest.get(s.root)!.scannedAt < s.scannedAt) latest.set(s.root, s);
      const files = (await Promise.all([...latest.values()].map((s) => safe(api.storage.search(s.scanId, { text: q, limit: 6, excludeHidden: true }))))).flatMap((r) => r?.items ?? []);
      for (const f of files.slice(0, 8)) out.push({ id: `file-${f.path}`, label: f.name, hint: `${f.path}${f.isDir ? '' : ` · ${formatBytes(f.size)}`}`, icon: f.isDir ? Folder : File, group: 'Files', run: () => attempt('Couldn’t open it', () => (f.isDir ? api.app.openPath(f.path) : api.app.revealPath(f.path))) });
      for (const n of notes ?? []) out.push({ id: `note-${n.id}`, label: n.title || 'Untitled note', hint: n.kind === 'idea' ? 'Idea for Claude' : 'Note', icon: n.kind === 'idea' ? Lightbulb : StickyNote, group: 'Notes', run: () => navigate('notes', { id: n.id }) });
      for (const a of (appList ?? []).filter((a) => a.name.toLowerCase().includes(low)).slice(0, 5)) out.push({ id: `app-${a.id}`, label: a.name, hint: `Open · ${a.publisher || a.source}`, icon: AppWindow, group: 'Apps', run: () => attempt('Couldn’t open it', () => api.apps.launch(a.id)) });
      for (const sh of shots ?? []) out.push({ id: `shot-${sh.id}`, label: sh.appTitle || sh.path.split(/[\\/]/).pop() || 'Screenshot', hint: `${sh.width}×${sh.height}${sh.note ? ` · ${sh.note}` : ''}`, icon: Image, group: 'Screenshots', run: () => attempt('Couldn’t open it', () => api.app.openPath(sh.path)) });
      for (const e of (vault ?? []).filter((e) => `${e.title} ${e.username} ${e.url}`.toLowerCase().includes(low)).slice(0, 5)) out.push({ id: `vault-${e.id}`, label: e.title, hint: e.username || e.url || 'Vault entry', icon: KeyRound, group: 'Vault', run: () => navigate('vault', { id: e.id }) });
      if (alive) setFound(out);
    }, 200);
    return () => {
      alive = false;
      clearTimeout(t);
    };
  }, [query, open, vaultOpen]);
  return found;
}

/** Calculator, units, Base64, JSON, UUID, hashes and timestamps: Enter copies the answer. */
function useAnswers(query: string, open: boolean): Cmd[] {
  const [found, setFound] = useState<Cmd[]>([]);
  useEffect(() => {
    if (!open || !query.trim()) {
      setFound([]);
      return;
    }
    let alive = true;
    void quickAnswers(query).then((list) => {
      if (!alive) return;
      setFound(
        list.map((a) => ({
          id: `answer-${a.id}`,
          label: a.value ? a.value.split('\n')[0].slice(0, 160) + (a.value.includes('\n') ? ' …' : '') : a.hint,
          hint: a.value ? `${a.hint} · Enter copies` : undefined,
          icon: Calculator,
          group: 'Answer',
          run: async () => {
            if (!a.value) return;
            if (await copyText(a.value)) toast.success('Copied', a.value.length > 80 ? `${a.value.slice(0, 80)}…` : a.value);
          },
        })),
      );
    });
    return () => {
      alive = false;
    };
  }, [query, open]);
  return found;
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

  const everything = useEverything(q, open);
  const answers = useAnswers(q, open);
  const results = useMemo(() => {
    const query = q.trim().toLowerCase();
    const cmds = commands
      .map((c) => ({ c, s: Math.max(score(query, c.label), score(query, `${c.keywords ?? ''} ${c.hint ?? ''}`) * 0.5) }))
      .filter((x) => x.s > 0)
      .sort((a, b) => (query ? b.s - a.s : 0))
      .map((x) => x.c);
    // The best pages and actions first, each group once, then what was found.
    const top = cmds.slice(0, query ? 6 : cmds.length);
    const order = ['Pages', 'Actions', 'Drives'];
    if (query) top.sort((a, b) => order.indexOf(a.group) - order.indexOf(b.group));
    return [...answers, ...top, ...everything];
  }, [commands, q, everything, answers]);

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
            placeholder="Search files, notes, apps, the vault… run an action, or type 12*4, 5 km in miles, uuid"
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
