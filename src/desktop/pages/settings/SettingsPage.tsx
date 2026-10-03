import type { Bind, ScanMode, Theme } from '@shared/types';
import { motion } from 'motion/react';
import type { LucideIcon } from 'lucide-react';
import { Camera, Check, FolderOpen, HardDrive, Info, KeyRound, Laptop, Moon, NotebookPen, Palette, Plus, RotateCcw, Settings as SettingsIcon, ShieldCheck, Smartphone, Sun, Trash, X } from 'lucide-react';
import { type ReactNode, useEffect, useRef, useState } from 'react';
import { api, errorText } from '../../api';
import { Logo } from '../../components/Logo';
import { Page } from '../../components/Page';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Kbd } from '../../components/ui/Card';
import { Segmented, Select, Switch, TextInput } from '../../components/ui/Form';
import { Callout } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { navigate, useRoute } from '../../lib/router';
import { confirm } from '../../state/dialogs';
import { useSettings } from '../../state/settings';
import { toast } from '../../state/toasts';

const SECTIONS: { id: string; label: string; icon: LucideIcon }[] = [
  { id: 'appearance', label: 'Appearance', icon: Palette },
  { id: 'general', label: 'General', icon: SettingsIcon },
  { id: 'storage', label: 'Storage', icon: HardDrive },
  { id: 'notes', label: 'Notes', icon: NotebookPen },
  { id: 'vault', label: 'Vault', icon: KeyRound },
  { id: 'phone', label: 'Phone', icon: Smartphone },
  { id: 'screenshots', label: 'Screenshots', icon: Camera },
  { id: 'privacy', label: 'Privacy & security', icon: ShieldCheck },
  { id: 'about', label: 'About', icon: Info },
];

const ACCENTS: { id: string; color: string; label: string }[] = [
  { id: 'violet', color: '#8b5cf6', label: 'Violet' },
  { id: 'cyan', color: '#06b6d4', label: 'Cyan' },
  { id: 'rose', color: '#f43f5e', label: 'Rose' },
  { id: 'emerald', color: '#10b981', label: 'Emerald' },
  { id: 'amber', color: '#f59e0b', label: 'Amber' },
];

function Section({ id, title, description, children }: { id: string; title: string; description?: string; children: ReactNode }) {
  return (
    <section id={`settings-${id}`} data-section={id} className="scroll-mt-4">
      <h2 className="font-display text-[17px] font-semibold tracking-tight text-fg">{title}</h2>
      {description && <p className="mt-0.5 text-[13px] text-dim">{description}</p>}
      <div className="card mt-3 divide-y divide-line">{children}</div>
    </section>
  );
}

function Row({ title, hint, children, stack }: { title: ReactNode; hint?: ReactNode; children?: ReactNode; stack?: boolean }) {
  return (
    <div className={cx('px-5 py-3.5', stack ? 'space-y-2.5' : 'flex items-center justify-between gap-6')}>
      <div className="min-w-0">
        <div className="text-[13.5px] font-medium text-fg">{title}</div>
        {hint && <div className="mt-0.5 text-[12.5px] leading-snug text-faint">{hint}</div>}
      </div>
      {children && <div className={cx(stack ? '' : 'shrink-0')}>{children}</div>}
    </div>
  );
}

function PathRow({ title, hint, value, fallback, pickTitle, onPick, onClear }: { title: string; hint?: string; value: string | null; fallback?: string; pickTitle: string; onPick: (p: string) => void; onClear?: () => void }) {
  return (
    <Row title={title} hint={hint} stack>
      <div className="flex items-center gap-2">
        <div className="flex min-w-0 flex-1 items-center gap-1 rounded-xl border border-line bg-surface py-1 pl-3 pr-1">
          <span className={cx('min-w-0 flex-1 truncate font-mono text-[12px]', value ? 'text-fg' : 'text-faint')}>{value ?? fallback ?? 'Not set'}</span>
          {(value ?? fallback) && <IconButton icon={FolderOpen} label="Open folder" size="sm" onClick={() => void api.app.openPath(value ?? fallback ?? '').catch((e: unknown) => toast.error('Could not open', errorText(e)))} />}
          {value && onClear && <IconButton icon={RotateCcw} label="Reset" size="sm" onClick={onClear} />}
        </div>
        <Button
          size="sm"
          onClick={async () => {
            const p = await api.app.pickFolder(pickTitle);
            if (p) onPick(p);
          }}
        >
          Choose…
        </Button>
      </div>
    </Row>
  );
}

/** Text/number input that saves on blur or Enter. */
function CommitInput({ value, onCommit, type = 'text', className, label, validate }: { value: string; onCommit: (v: string) => void; type?: string; className?: string; label: string; validate?: (v: string) => string | null }) {
  const [v, setV] = useState(value);
  const [err, setErr] = useState<string | null>(null);
  useEffect(() => setV(value), [value]);
  const commit = () => {
    if (v === value) return;
    const e = validate?.(v) ?? null;
    setErr(e);
    if (!e) onCommit(v);
  };
  return (
    <div className={className}>
      <TextInput type={type} value={v} onChange={(e) => setV(e.target.value)} onBlur={commit} onKeyDown={(e) => e.key === 'Enter' && (e.currentTarget as HTMLInputElement).blur()} aria-label={label} aria-invalid={!!err} inputSize="sm" />
      {err && <div className="mt-1 text-[11.5px] text-bad">{err}</div>}
    </div>
  );
}

function ThemePreview({ theme }: { theme: Theme }) {
  const half = (light: boolean) => (
    <div className={cx('flex h-full flex-1 gap-1 p-1.5', light ? 'bg-[#f5f5fa]' : 'bg-[#0d0d16]')}>
      <div className={cx('w-1/4 rounded-sm', light ? 'bg-[#e6e6f0]' : 'bg-[#1a1a26]')} />
      <div className="flex flex-1 flex-col gap-1">
        <div className="h-1.5 w-2/3 rounded-full bg-accent/70" />
        <div className={cx('flex-1 rounded-sm', light ? 'bg-white' : 'bg-[#16161f]')} />
      </div>
    </div>
  );
  return <div className="flex h-16 overflow-hidden rounded-lg border border-line">{theme === 'system' ? <>{half(false)}{half(true)}</> : half(theme === 'light')}</div>;
}

export function SettingsPage() {
  const route = useRoute();
  const s = useSettings((st) => st.settings);
  const info = useSettings((st) => st.info);
  const update = useSettings((st) => st.update);
  const [active, setActive] = useState('appearance');
  const [exclude, setExclude] = useState('');
  const scroller = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const root = scroller.current;
    if (!root) return;
    const io = new IntersectionObserver(
      (entries) => {
        const vis = entries.filter((e) => e.isIntersecting).sort((a, b) => a.boundingClientRect.top - b.boundingClientRect.top);
        if (vis[0]) setActive((vis[0].target as HTMLElement).dataset.section ?? 'appearance');
      },
      { root, rootMargin: '0px 0px -70% 0px' },
    );
    root.querySelectorAll('[data-section]').forEach((el) => io.observe(el));
    return () => io.disconnect();
  }, [s != null]);

  const go = (id: string) => scroller.current?.querySelector(`#settings-${id}`)?.scrollIntoView({ behavior: 'smooth', block: 'start' });
  useEffect(() => {
    const sec = route.params.get('section');
    if (sec) setTimeout(() => go(sec), 250);
  }, [route.params]);

  if (!s) return null;

  const setAutostart = async (v: boolean) => {
    try {
      await api.app.setAutostart(v);
      await update({ general: { launchAtLogin: v } });
    } catch (e) {
      toast.error('Could not change autostart', errorText(e));
    }
  };

  return (
    <Page title="Settings" subtitle="Everything saves as soon as you change it." scroll={false}>
      <div className="grid min-h-0 flex-1 grid-cols-[210px_minmax(0,1fr)] gap-6">
        <nav aria-label="Settings sections" className="space-y-0.5">
          {SECTIONS.map((sec) => (
            <button key={sec.id} type="button" onClick={() => go(sec.id)} aria-current={active === sec.id ? 'true' : undefined} className={cx('relative flex w-full items-center gap-2.5 rounded-[10px] px-3 py-2 text-[13px] font-medium transition-colors', active === sec.id ? 'text-fg' : 'text-dim hover:bg-surface hover:text-fg')}>
              {active === sec.id && <motion.span layoutId="settings-nav" transition={{ type: 'spring', stiffness: 500, damping: 40 }} className="absolute inset-0 rounded-[10px] bg-surface-2" />}
              <sec.icon size={15} className={cx('relative', active === sec.id ? 'text-accent' : 'text-faint')} aria-hidden />
              <span className="relative">{sec.label}</span>
            </button>
          ))}
        </nav>
        <div ref={scroller} className="min-h-0 space-y-8 overflow-y-auto pb-[50vh] pr-2">
          <Section id="appearance" title="Appearance">
            <Row title="Theme" stack>
              <div className="grid grid-cols-3 gap-3">
                {(
                  [
                    ['system', 'System', Laptop],
                    ['dark', 'Dark', Moon],
                    ['light', 'Light', Sun],
                  ] as [Theme, string, LucideIcon][]
                ).map(([t, label, I]) => (
                  <button key={t} type="button" onClick={() => void update({ general: { theme: t } })} aria-pressed={s.general.theme === t} className={cx('rounded-xl border p-2 text-left transition-colors', s.general.theme === t ? 'accent-ring border-transparent bg-surface-2' : 'border-line bg-surface hover:border-line-strong')}>
                    <ThemePreview theme={t} />
                    <div className="mt-2 flex items-center gap-1.5 px-0.5 text-[13px] font-medium text-fg">
                      <I size={14} className="text-dim" /> {label}
                      {s.general.theme === t && <Check size={14} className="ml-auto text-accent" />}
                    </div>
                  </button>
                ))}
              </div>
            </Row>
            <Row title="Accent colour">
              <div className="flex gap-2.5" role="radiogroup" aria-label="Accent colour">
                {ACCENTS.map((a) => (
                  <button key={a.id} type="button" role="radio" aria-checked={s.general.accent === a.id} aria-label={a.label} title={a.label} onClick={() => void update({ general: { accent: a.id } })} className={cx('flex h-8 w-8 items-center justify-center rounded-full transition-transform hover:scale-110', s.general.accent === a.id && 'ring-2 ring-fg/80 ring-offset-2 ring-offset-[var(--bg)]')} style={{ background: a.color }}>
                    {s.general.accent === a.id && <Check size={15} className="text-white" />}
                  </button>
                ))}
              </div>
            </Row>
            <Row title="Reduce motion" hint="Turns off page transitions, zoom animations and springy effects.">
              <Switch checked={s.general.reducedMotion} onChange={(v) => void update({ general: { reducedMotion: v } })} label="Reduce motion" />
            </Row>
          </Section>

          <Section id="general" title="General">
            <Row title="Launch at Windows sign-in" hint="OmniHub starts in the background so hotkeys and the phone companion are ready.">
              <Switch checked={s.general.launchAtLogin} onChange={(v) => void setAutostart(v)} label="Launch at login" />
            </Row>
            <Row title="Start minimized" hint="Open to the tray instead of showing the window.">
              <Switch checked={s.general.startMinimized} onChange={(v) => void update({ general: { startMinimized: v } })} label="Start minimized" />
            </Row>
            <Row title="Close to tray" hint="The close button hides the window; quit from the tray icon.">
              <Switch checked={s.general.closeToTray} onChange={(v) => void update({ general: { closeToTray: v } })} label="Close to tray" />
            </Row>
          </Section>

          <Section id="storage" title="Storage" description="How drives are scanned and what counts as worth cleaning.">
            <Row title="Default scan mode" hint="Fast reads the NTFS Master File Table and asks for admin approval each time; Standard walks folders.">
              <Segmented<ScanMode>
                size="sm"
                label="Default scan mode"
                value={s.storage.defaultMode}
                onChange={(v) => void update({ storage: { defaultMode: v } })}
                options={[
                  { value: 'auto', label: 'Auto' },
                  { value: 'fast', label: 'Fast (MFT)' },
                  { value: 'standard', label: 'Standard' },
                ]}
              />
            </Row>
            <Row title="Size shown" hint="Logical size, or the space files actually occupy on disk (clusters).">
              <Segmented
                size="sm"
                label="Size metric"
                value={s.storage.sizeMetric}
                onChange={(v) => void update({ storage: { sizeMetric: v } })}
                options={[
                  { value: 'size', label: 'Size' },
                  { value: 'alloc', label: 'On disk' },
                ]}
              />
            </Row>
            <Row title="Include hidden and system files">
              <Switch checked={s.storage.showHidden} onChange={(v) => void update({ storage: { showHidden: v } })} label="Show hidden files" />
            </Row>
            <Row title="Excluded folders" hint="Skipped by standard scans (fast scans read the whole drive and hide these afterwards)." stack>
              <div className="space-y-1.5">
                {s.storage.exclude.map((p) => (
                  <div key={p} className="flex items-center gap-2 rounded-lg border border-line bg-surface py-0.5 pl-3 pr-0.5">
                    <span className="min-w-0 flex-1 truncate font-mono text-[12px] text-dim">{p}</span>
                    <IconButton icon={X} label={`Remove ${p}`} size="sm" onClick={() => void update({ storage: { exclude: s.storage.exclude.filter((x) => x !== p) } })} />
                  </div>
                ))}
                <form
                  className="flex gap-2"
                  onSubmit={(e) => {
                    e.preventDefault();
                    const v = exclude.trim();
                    if (v && !s.storage.exclude.includes(v)) void update({ storage: { exclude: [...s.storage.exclude, v] } });
                    setExclude('');
                  }}
                >
                  <TextInput value={exclude} onChange={(e) => setExclude(e.target.value)} placeholder="C:\path\to\skip or *\node_modules" className="flex-1" inputSize="sm" aria-label="Folder to exclude" />
                  <Button size="sm" type="submit" icon={Plus} disabled={!exclude.trim()}>
                    Add
                  </Button>
                  <Button
                    size="sm"
                    icon={FolderOpen}
                    onClick={async () => {
                      const p = await api.app.pickFolder('Choose a folder to exclude');
                      if (p && !s.storage.exclude.includes(p)) void update({ storage: { exclude: [...s.storage.exclude, p] } });
                    }}
                  >
                    Browse
                  </Button>
                </form>
              </div>
            </Row>
            <Row title="“Large old files” means" hint="Used by the cleanup suggestions.">
              <div className="flex items-center gap-2 text-[12.5px] text-dim">
                over
                <Select value={String(s.storage.cleanup.largeFileMin)} onChange={(e) => void update({ storage: { cleanup: { largeFileMin: Number(e.target.value) } } })} className="w-[100px]" aria-label="Minimum size">
                  {[256, 512, 1024, 2048, 5120, 10240].map((mb) => (
                    <option key={mb} value={mb * 1024 * 1024}>
                      {mb >= 1024 ? `${mb / 1024} GB` : `${mb} MB`}
                    </option>
                  ))}
                </Select>
                untouched for
                <Select value={String(s.storage.cleanup.largeFileAgeDays)} onChange={(e) => void update({ storage: { cleanup: { largeFileAgeDays: Number(e.target.value) } } })} className="w-[120px]" aria-label="Age">
                  {[90, 180, 365, 730].map((d) => (
                    <option key={d} value={d}>
                      {d >= 365 ? `${d / 365} year${d > 365 ? 's' : ''}` : `${d} days`}
                    </option>
                  ))}
                </Select>
              </div>
            </Row>
            <Row title="Old installers in Downloads" hint="Setup files and archives older than this are suggested for cleanup.">
              <Select value={String(s.storage.cleanup.installerAgeDays)} onChange={(e) => void update({ storage: { cleanup: { installerAgeDays: Number(e.target.value) } } })} className="w-[130px]" aria-label="Installer age">
                {[7, 14, 30, 60, 90, 180].map((d) => (
                  <option key={d} value={d}>
                    {d} days
                  </option>
                ))}
              </Select>
            </Row>
          </Section>

          <Section id="notes" title="Notes & ideas for Claude">
            <PathRow title="Claude folder" hint="“Send to Claude” writes ideas here as Markdown. Point it at a folder Claude (e.g. Claude Code) works in." value={s.notes.claudeFolder} pickTitle="Choose the Claude ideas folder" onPick={(p) => void update({ notes: { claudeFolder: p } })} onClear={() => void update({ notes: { claudeFolder: null } })} />
            <Row title="Also write a JSON file" hint="A .json sidecar with the same data, for scripts.">
              <Switch checked={s.notes.sidecarJson} onChange={(v) => void update({ notes: { sidecarJson: v } })} label="JSON sidecar" />
            </Row>
            <Row title="Send ideas automatically" hint="Every save of an idea updates its file in the Claude folder.">
              <Switch checked={s.notes.autoExportIdeas} onChange={(v) => void update({ notes: { autoExportIdeas: v } })} label="Auto-export ideas" />
            </Row>
            <Row title="Keep INDEX.md" hint="A list of every exported idea, newest first.">
              <Switch checked={s.notes.indexFile} onChange={(v) => void update({ notes: { indexFile: v } })} label="INDEX.md" />
            </Row>
          </Section>

          <Section id="vault" title="Vault">
            <Row title="Lock after inactivity">
              <Select value={String(s.vault.autoLockMinutes)} onChange={(e) => void update({ vault: { autoLockMinutes: Number(e.target.value) } })} className="w-[140px]" aria-label="Auto-lock">
                {[1, 2, 5, 10, 15, 30, 60].map((m) => (
                  <option key={m} value={m}>
                    {m} minute{m > 1 ? 's' : ''}
                  </option>
                ))}
              </Select>
            </Row>
            <Row title="Clear copied secrets after">
              <Select value={String(s.vault.clipboardClearSeconds)} onChange={(e) => void update({ vault: { clipboardClearSeconds: Number(e.target.value) } })} className="w-[140px]" aria-label="Clipboard clear">
                {[10, 15, 20, 30, 45, 60, 90].map((n) => (
                  <option key={n} value={n}>
                    {n} seconds
                  </option>
                ))}
              </Select>
            </Row>
            <Row title="Lock when Windows locks">
              <Switch checked={s.vault.lockOnSessionLock} onChange={(v) => void update({ vault: { lockOnSessionLock: v } })} label="Lock with Windows" />
            </Row>
            <Row title="Let paired phones use the vault" hint={s.remote.tls ? 'Phones must type the master password; entries are revealed one at a time.' : 'Requires HTTPS for the phone companion.'}>
              <Switch checked={s.vault.allowPhone && s.remote.tls} disabled={!s.remote.tls} onChange={(v) => void update({ vault: { allowPhone: v } })} label="Allow phones" />
            </Row>
          </Section>

          <Section id="phone" title="Phone companion" description="The local web server your phone connects to.">
            <Row title="Port" hint="Restart the companion after changing it.">
              <CommitInput
                type="number"
                label="Port"
                value={String(s.remote.port)}
                className="w-[110px]"
                validate={(v) => (/^\d+$/.test(v) && Number(v) >= 1024 && Number(v) <= 65535 ? null : '1024–65535')}
                onCommit={(v) => void update({ remote: { port: Number(v) } })}
              />
            </Row>
            <Row title="HTTPS" hint="Encrypts traffic with a certificate this PC creates. Strongly recommended; phones see a one-time warning.">
              <Switch checked={s.remote.tls} onChange={(v) => void update({ remote: { tls: v } })} label="HTTPS" />
            </Row>
            {!s.remote.tls && (
              <div className="px-5 py-3">
                <Callout tone="warn">Without HTTPS anyone on your Wi-Fi can read the traffic. Vault access from phones is disabled.</Callout>
              </div>
            )}
            <Row title="Who can connect">
              <Segmented<Bind>
                size="sm"
                label="Bind"
                value={s.remote.bind}
                onChange={(v) => void update({ remote: { bind: v } })}
                options={[
                  { value: 'lan', label: 'Local network' },
                  { value: 'localhost', label: 'This PC only' },
                ]}
              />
            </Row>
            <Row title="Allow Tailscale" hint="Also accept connections from your tailnet (100.64.0.0/10), e.g. away from home.">
              <Switch checked={s.remote.allowTailscale} onChange={(v) => void update({ remote: { allowTailscale: v } })} label="Tailscale" />
            </Row>
            <Row title="Name shown on phones">
              <CommitInput label="Device name" value={s.remote.deviceName} className="w-[220px]" validate={(v) => (v.trim() ? null : 'Required')} onCommit={(v) => void update({ remote: { deviceName: v.trim() } })} />
            </Row>
            <PathRow title="Incoming folder" hint="Files sent from phones are saved here." value={s.remote.incomingDir} fallback={info?.incomingDir} pickTitle="Choose the incoming folder" onPick={(p) => void update({ remote: { incomingDir: p } })} onClear={() => void update({ remote: { incomingDir: null } })} />
          </Section>

          <Section id="screenshots" title="Screenshots">
            <PathRow title="Save to" value={s.screenshots.dir} fallback={info?.screenshotDir} pickTitle="Choose the screenshot folder" onPick={(p) => void update({ screenshots: { dir: p } })} onClear={() => void update({ screenshots: { dir: null } })} />
            <Row title="Format">
              <Segmented
                size="sm"
                label="Format"
                value={s.screenshots.format}
                onChange={(v) => void update({ screenshots: { format: v } })}
                options={[
                  { value: 'png', label: 'PNG' },
                  { value: 'jpeg', label: 'JPEG' },
                ]}
              />
            </Row>
            <Row title="Copy to the clipboard too">
              <Switch checked={s.screenshots.copyToClipboard} onChange={(v) => void update({ screenshots: { copyToClipboard: v } })} label="Copy to clipboard" />
            </Row>
            <Row title="Hotkeys" hint="Work anywhere while OmniHub runs.">
              <div className="space-y-1.5 text-right text-[12.5px] text-dim">
                <div>
                  Region <Kbd className="ml-2">{s.screenshots.hotkeyRegion}</Kbd>
                </div>
                <div>
                  Screen <Kbd className="ml-2">{s.screenshots.hotkeyFull}</Kbd>
                </div>
                <div>
                  Window <Kbd className="ml-2">{s.screenshots.hotkeyWindow}</Kbd>
                </div>
              </div>
            </Row>
          </Section>

          <Section id="privacy" title="Privacy & security">
            <Row
              title={
                <span className="flex items-center gap-2">
                  Permissions <Badge tone={info?.elevated ? 'warn' : 'good'}>{info?.elevated ? 'Running as administrator' : 'Standard user'}</Badge>
                </span>
              }
              hint="OmniHub runs without admin rights. Fast scans ask Windows for approval each time; some cleanup items (Windows Update files, system temp) need the whole app elevated."
            >
              {!info?.elevated && (
                <Button
                  size="sm"
                  icon={ShieldCheck}
                  onClick={async () => {
                    const ok = await confirm({ title: 'Restart as administrator?', description: 'OmniHub closes and reopens elevated after you approve the Windows prompt. Use it only for cleanup items that need it.', confirmLabel: 'Restart' });
                    if (ok) await api.app.restartElevated().catch((e: unknown) => toast.error('Could not restart', errorText(e)));
                  }}
                >
                  Restart as administrator
                </Button>
              )}
            </Row>
            {info &&
              (
                [
                  ['Data', info.dataDir, 'Database, vault and snapshots'],
                  ['Settings', info.configDir, 'settings.json'],
                  ['Cache', info.cacheDir, 'Thumbnails and icons — safe to delete'],
                ] as const
              ).map(([label, path, hint]) => (
                <Row key={label} title={`${label} folder`} hint={hint}>
                  <div className="flex items-center gap-1 rounded-xl border border-line bg-surface py-0.5 pl-3 pr-0.5">
                    <span className="max-w-[340px] truncate font-mono text-[11.5px] text-dim">{path}</span>
                    <IconButton icon={FolderOpen} label={`Open ${label.toLowerCase()} folder`} size="sm" onClick={() => void api.app.openPath(path)} />
                  </div>
                </Row>
              ))}
            <Row title="Activity log" hint="Pairings, transfers, power actions and vault events. Kept on this PC only.">
              <div className="flex gap-2">
                <Button size="sm" onClick={() => navigate('phone')}>
                  View
                </Button>
                <Button
                  size="sm"
                  variant="danger"
                  icon={Trash}
                  onClick={async () => {
                    const ok = await confirm({ title: 'Clear the activity log?', description: 'All recorded events are deleted.', tone: 'danger', confirmLabel: 'Clear log' });
                    if (!ok) return;
                    await api.app.clearAudit().then(() => toast.success('Activity log cleared'), (e: unknown) => toast.error('Could not clear', errorText(e)));
                  }}
                >
                  Clear
                </Button>
              </div>
            </Row>
          </Section>

          <Section id="about" title="About">
            <div className="flex items-center gap-4 px-5 py-5">
              <Logo size={52} />
              <div className="flex-1">
                <div className="font-display text-[18px] font-semibold text-fg">OmniHub</div>
                <div className="text-[12.5px] text-dim">
                  Version {info?.version} · {info?.platform} · {info?.hostname}
                </div>
                <div className="mt-0.5 text-[12px] text-faint">Storage, apps, screenshots, notes, vault and phone companion — local first.</div>
              </div>
              <Button size="sm" variant="ghost" onClick={() => void update({ general: { onboarded: false } }, { silent: true })}>
                Show the welcome tour
              </Button>
            </div>
          </Section>
        </div>
      </div>
    </Page>
  );
}
