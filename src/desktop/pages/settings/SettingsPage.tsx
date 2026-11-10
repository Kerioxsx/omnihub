import type { Bind, EqStatus, PresenceSettings, PresenceStatus, PrivacyUse, ScanMode, Theme } from '@shared/types';
import { formatRelative } from '@shared/format';
import { motion } from 'motion/react';
import type { LucideIcon } from 'lucide-react';
import { Camera, Check, Download, ExternalLink, FolderOpen, HardDrive, Info, KeyRound, Laptop, Moon, Music2, NotebookPen, Palette, Plus, RotateCcw, Settings as SettingsIcon, ShieldCheck, Siren, Smartphone, Sun, Trash, Upload, X } from 'lucide-react';
import { type ReactNode, useEffect, useRef, useState } from 'react';
import { api, errorText } from '../../api';
import { Backdrop } from '../../components/Backdrop';
import { Logo } from '../../components/Logo';
import { updateText, useUpdate } from '../../components/UpdateCard';
import { Page } from '../../components/Page';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Kbd } from '../../components/ui/Card';
import { Segmented, Select, Switch, TextInput } from '../../components/ui/Form';
import { Callout } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { forgetPictures, LOOK_CHANGED, useAuroraLook } from '../../lib/aurora/textures';
import { wallSummary } from '@shared/palette';
import { type PcWall, setPhotoPrefs, setWallpaperLocal, usePhotoPrefs, useWallpaper } from '../../lib/wallpaper';
import { LOOKS, PICTURE_LOOKS } from '../../lib/looks';
import { artPrefs, setArtPrefs } from '../../components/Backdrop';
import { controllerModeOn, setControllerMode } from '../../lib/padNav';
import { navigate, useRoute } from '../../lib/router';
import { confirm } from '../../state/dialogs';
import { useLive } from '../../state/live';
import { useSettings } from '../../state/settings';
import { toast } from '../../state/toasts';

const SECTIONS: { id: string; label: string; icon: LucideIcon }[] = [
  { id: 'appearance', label: 'Appearance', icon: Palette },
  { id: 'general', label: 'General', icon: SettingsIcon },
  { id: 'storage', label: 'Storage', icon: HardDrive },
  { id: 'notes', label: 'Notes', icon: NotebookPen },
  { id: 'vault', label: 'Vault', icon: KeyRound },
  { id: 'phone', label: 'Phone', icon: Smartphone },
  { id: 'music', label: 'Music', icon: Music2 },
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
function CommitInput({ value, onCommit, type = 'text', className, label, validate, placeholder }: { value: string; onCommit: (v: string) => void; type?: string; className?: string; label: string; validate?: (v: string) => string | null; placeholder?: string }) {
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
      <TextInput type={type} value={v} onChange={(e) => setV(e.target.value)} onBlur={commit} onKeyDown={(e) => e.key === 'Enter' && (e.currentTarget as HTMLInputElement).blur()} aria-label={label} aria-invalid={!!err} inputSize="sm" placeholder={placeholder} />
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
  return <div className="flex h-full min-h-16 overflow-hidden rounded-lg">{theme === 'system' ? <>{half(false)}{half(true)}</> : half(theme === 'light')}</div>;
}

export function SettingsPage() {
  const route = useRoute();
  const s = useSettings((st) => st.settings);
  const info = useSettings((st) => st.info);
  const update = useSettings((st) => st.update);
  const [active, setActive] = useState('appearance');
  const [exclude, setExclude] = useState('');
  const vaultUnlocked = useLive((l) => l.vault?.unlocked ?? false);
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
              <div className="grid grid-cols-4 gap-3" role="radiogroup" aria-label="Theme">
                {LOOKS.map((l) => (
                  <button key={l.id} type="button" role="radio" aria-checked={s.general.look === l.id} onClick={() => void update({ general: { look: l.id } })} className={cx('rounded-xl border p-2 text-left transition-colors', s.general.look === l.id ? 'accent-ring border-transparent bg-surface-2' : 'border-line bg-surface hover:border-line-strong')}>
                    <div className="relative h-20 overflow-hidden rounded-lg border border-line">{l.id === 'classic' ? <ThemePreview theme={s.general.theme} /> : <Backdrop look={l.id} contained />}</div>
                    <div className="mt-2 flex items-center gap-1.5 px-0.5 text-[13px] font-medium text-fg">
                      {l.name}
                      {s.general.look === l.id && <Check size={14} className="ml-auto text-accent" />}
                    </div>
                    <div className="px-0.5 text-[11.5px] leading-snug text-faint">{l.blurb}</div>
                  </button>
                ))}
              </div>
            </Row>
            {s.general.look === 'classic' ? (
              <>
                <Row title="Light or dark" stack>
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
              </>
            ) : (
              <>
                <Row title="Animate the background" hint="Moves only while OmniHub is on screen, and holds still when Reduce motion is on. Off shows a still picture of the theme.">
                  <Switch checked={s.general.animateBackdrop} onChange={(v) => void update({ general: { animateBackdrop: v } })} label="Animate the background" />
                </Row>
                {s.general.look === 'minecraft' && <ItemPackRow />}
                {s.general.look === 'photo' && <PhotoRow />}
                {PICTURE_LOOKS.includes(s.general.look) && <ThemePictureRow look={s.general.look} />}
              </>
            )}
            <Row title="Glass" hint="Frosted, see-through cards and sidebar over the background. Over a moving theme it uses a little more graphics power.">
              <Switch checked={s.general.glass} onChange={(v) => void update({ general: { glass: v } })} label="Glass" />
            </Row>
            <Row title="Reduce motion" hint="Turns off page transitions, zoom animations and springy effects.">
              <Switch checked={s.general.reducedMotion} onChange={(v) => void update({ general: { reducedMotion: v } })} label="Reduce motion" />
            </Row>
            <Row title="Interface size" hint="Makes text and controls larger or smaller. Ctrl+= and Ctrl+- work too.">
              <Select value={String(s.general.uiScale)} onChange={(e) => void update({ general: { uiScale: Number(e.target.value) } })} className="w-[140px]" aria-label="Interface size">
                {[80, 90, 100, 110, 125, 150].map((z) => (
                  <option key={z} value={z}>
                    {z}%{z === 100 ? ' (default)' : ''}
                  </option>
                ))}
              </Select>
            </Row>
          </Section>

          <Section id="general" title="General">
            <Row title="Your name" hint="Used in the greeting on Home. Leave empty to use your Windows name.">
              <CommitInput label="Your name" value={s.general.displayName} placeholder={info?.userName || 'Your name'} className="w-[220px]" onCommit={(v) => void update({ general: { displayName: v.trim().slice(0, 40) } })} />
            </Row>
            <Row title="Launch at Windows sign-in" hint="OmniHub starts in the background so hotkeys and the phone companion are ready.">
              <Switch checked={s.general.launchAtLogin} onChange={(v) => void setAutostart(v)} label="Launch at login" />
            </Row>
            <Row title="Start minimized" hint="Open to the tray instead of showing the window.">
              <Switch checked={s.general.startMinimized} onChange={(v) => void update({ general: { startMinimized: v } })} label="Start minimized" />
            </Row>
            <Row title="Close to tray" hint="The close button hides the window; quit from the tray icon.">
              <Switch checked={s.general.closeToTray} onChange={(v) => void update({ general: { closeToTray: v } })} label="Close to tray" />
            </Row>
            <Row title="“Send to → OmniHub (phone)” in Explorer" hint="Right-click files or folders in Explorer, choose Send to, and they open here ready to send to your phone.">
              <Switch checked={s.remote.sendToMenu} onChange={(v) => void update({ remote: { sendToMenu: v } })} label="Explorer Send to menu" />
            </Row>
            <ControllerRow />
            <DiscordRows />
          </Section>

          <Section id="storage" title="Storage" description="How drives are scanned and what counts as worth cleaning.">
            <ScanApprovalRow />
            <Row title="Default scan mode" hint="Fast reads the NTFS Master File Table (it needs administrator rights); Standard walks folders.">
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
            <Row title="Explorer view" hint="How the Explorer tab shows a folder. You can also switch on the tab itself.">
              <Select value={s.storage.explorerView} onChange={(e) => void update({ storage: { explorerView: e.target.value as typeof s.storage.explorerView } })} className="w-[200px]" aria-label="Explorer view">
                <option value="split">Treemap + list</option>
                <option value="list">Details list</option>
                <option value="grid">Grid with previews</option>
                <option value="treemap">Treemap only</option>
                <option value="sunburst">Rings (sunburst)</option>
              </Select>
            </Row>
            <Row title="Image previews in the grid" hint="Reads image files to make small previews; turn off on slow or network drives.">
              <Switch checked={s.storage.gridPreviews} onChange={(v) => void update({ storage: { gridPreviews: v } })} label="Image previews" />
            </Row>
            <Row title="Warn when a drive is almost full" hint="A banner on Home, and a Windows notification when OmniHub is in the tray.">
              <div className="flex items-center gap-2">
                <Select value={String(s.storage.lowSpacePercent)} onChange={(e) => void update({ storage: { lowSpacePercent: Number(e.target.value) } })} disabled={!s.storage.lowSpaceAlert} className="w-[150px]" aria-label="Low space threshold">
                  {[5, 10, 15, 20].map((p) => (
                    <option key={p} value={p}>
                      Under {p}% free
                    </option>
                  ))}
                </Select>
                <Switch checked={s.storage.lowSpaceAlert} onChange={(v) => void update({ storage: { lowSpaceAlert: v } })} label="Low space warning" />
              </div>
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
            <Row title="Master password" hint={vaultUnlocked ? 'The password that unlocks the vault.' : 'Unlock the vault first, then change it here or in the vault.'}>
              <Button size="sm" icon={KeyRound} onClick={() => navigate('vault', { password: '1' })}>
                {vaultUnlocked ? 'Change…' : 'Open the vault'}
              </Button>
            </Row>
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
            <PathRow title="Scans folder" hint="Pages scanned with the phone (More → Scan) are saved here, with their text." value={s.remote.scansDir} fallback="Documents\\OmniHub Scans" pickTitle="Choose the scans folder" onPick={(p) => void update({ remote: { scansDir: p } })} onClear={() => void update({ remote: { scansDir: null } })} />
          </Section>

          <MusicSection />

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
            <Row title="Panic button" hint="Locks the vault, cuts every phone off, stops screen sharing, hides OmniHub and locks the PC. Also in the tray menu, and from a paired phone.">
              <Button
                size="sm"
                variant="danger"
                icon={Siren}
                onClick={async () => {
                  const ok = await confirm({ title: 'Lock everything now?', description: 'The vault locks, phones are cut off (phone access stays off until you turn it on in Phone) and the PC locks.', confirmLabel: 'Panic', tone: 'danger' });
                  if (ok) await api.privacy.panic().catch((e: unknown) => toast.error('Couldn’t lock everything', errorText(e)));
                }}
              >
                Panic
              </Button>
            </Row>
            <Row title="Alert when a phone connects" hint="A Windows notification when a paired phone connects after being away for 10 minutes, so you know if someone else uses it.">
              <Switch checked={s.remote.connectAlerts} onChange={(v) => void update({ remote: { connectAlerts: v } })} label="Connect alerts" />
            </Row>
            <Row title="Approve on your phone" hint="Before these happen on the PC, OmniHub asks your paired phone and waits up to a minute for Allow (open OmniHub on the phone). Turning this off needs the phone too. It slows down someone at your PC; it isn’t a lock against someone with your Windows account and time.">
              <Switch checked={s.approvals.enabled} onChange={(v) => void update({ approvals: { enabled: v } }).catch((e: unknown) => toast.error('Not changed', errorText(e)))} label="Approve on your phone" />
            </Row>
            {s.approvals.enabled && (
              <Row title="Ask for" stack>
                <div className="grid grid-cols-2 gap-2 text-[13px]">
                  {(
                    [
                      ['vault', 'Opening a vault login or copying its password'],
                      ['admin', 'Restarting OmniHub as administrator'],
                      ['shred', 'Shredding files'],
                      ['environment', 'Changing PATH and environment variables'],
                    ] as const
                  ).map(([k, label]) => (
                    <label key={k} className="flex items-center gap-2 rounded-lg border border-line px-3 py-2">
                      <input type="checkbox" className="accent-[var(--accent)]" checked={s.approvals[k]} onChange={(e) => void update({ approvals: { [k]: e.target.checked } }).catch((err: unknown) => toast.error('Not changed', errorText(err)))} />
                      {label}
                    </label>
                  ))}
                </div>
              </Row>
            )}
            <PrivacyDashboard />
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
            <Row title="Back up settings" hint="Save every setting and window layout to a file, or restore them on this or another PC. The vault, notes, keys and paired phones are not included.">
              <div className="flex gap-2">
                <Button
                  size="sm"
                  icon={Download}
                  onClick={async () => {
                    const path = await api.app.saveFile('Save OmniHub settings', 'OmniHub settings.json');
                    if (!path) return;
                    await api.app.exportSettings(path).then(() => toast.success('Settings saved', path), (e: unknown) => toast.error('Could not save', errorText(e)));
                  }}
                >
                  Save to file
                </Button>
                <Button
                  size="sm"
                  icon={Upload}
                  onClick={async () => {
                    const [path] = await api.app.pickFiles('Choose an OmniHub settings file');
                    if (!path) return;
                    const ok = await confirm({ title: 'Restore these settings?', description: 'Your current settings are replaced by the ones in the file.', confirmLabel: 'Restore' });
                    if (!ok) return;
                    await api.app.importSettings(path).then(() => toast.success('Settings restored'), (e: unknown) => toast.error('Could not restore', errorText(e)));
                  }}
                >
                  Restore…
                </Button>
              </div>
            </Row>
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
            <UpdateRows />
          </Section>
        </div>
      </div>
    </Page>
  );
}

/** "Don't ask again" for the administrator prompt of fast scans. */
function ScanApprovalRow() {
  const [on, setOn] = useState<boolean | null>(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    void api.storage.scanTaskStatus().then(setOn, () => setOn(false));
  }, []);
  const toggle = async (v: boolean) => {
    setBusy(true);
    try {
      setOn(await api.storage.scanTaskSet(v));
      toast.success(v ? 'Fast scans won’t ask again' : 'Fast scans will ask each time');
    } catch (e) {
      const msg = errorText(e);
      toast.error(/declined/i.test(msg) ? 'Administrator approval was declined' : 'That did not work', msg);
    } finally {
      setBusy(false);
    }
  };
  return (
    <Row
      title="Approve fast scans once"
      hint="Windows asks for administrator approval before every fast scan. Approve once and OmniHub keeps a Windows scheduled task that only runs its drive scan, so later scans start straight away. Best on a PC only you use."
    >
      <Switch checked={!!on} disabled={on === null || busy} onChange={(v) => void toggle(v)} label="Approve fast scans once" />
    </Row>
  );
}

/** Updates: status, check now, automatic checks and installs. */
function UpdateRows() {
  const s = useSettings((x) => x.settings);
  const update = useSettings((x) => x.update);
  const { info, check, install } = useUpdate();
  if (!s) return null;
  const st = info?.state;
  const busy = st?.state === 'checking' || st?.state === 'downloading' || st?.state === 'installing';
  return (
    <>
      <Row title="Updates" hint={st ? updateText(st) : 'Checking…'}>
        <div className="flex gap-2">
          {st?.state === 'available' && (
            <Button size="sm" variant="primary" icon={Download} onClick={() => void install()}>
              Update to {st.release.version}
            </Button>
          )}
          <Button size="sm" variant="secondary" icon={RotateCcw} loading={busy} onClick={() => void check()}>
            Check now
          </Button>
        </div>
      </Row>
      <Row title="Check for updates automatically" hint="When OmniHub starts and every six hours, from OmniHub's GitHub releases.">
        <Switch checked={s.updates.check} onChange={(v) => void update({ updates: { check: v } })} label="Check for updates automatically" />
      </Row>
      <Row title="Install updates automatically" hint="Downloads the new version, checks it against its published checksum and installs it — but never during a game boost, screen sharing or a phone transfer. Settings and data stay.">
        <Switch checked={s.updates.autoInstall} disabled={!s.updates.check} onChange={(v) => void update({ updates: { autoInstall: v } })} label="Install updates automatically" />
      </Row>
    </>
  );
}

/** Music: phone control, online lyrics, volume, bass and treble. */
function MusicSection() {
  const s = useSettings((x) => x.settings);
  const update = useSettings((x) => x.update);
  const [eq, setEq] = useState<EqStatus | null>(null);
  useEffect(() => {
    void api.media.audio().then((a) => setEq(a.eq), () => undefined);
  }, []);
  if (!s) return null;
  const m = s.media;
  return (
    <Section id="music" title="Music" description="What the PC plays, on your phone: cover, controls and time-synced lyrics.">
      <Row title="Let phones control music" hint="Play, pause, skip, seek and volume from the phone's Music tab. Works with Spotify, Apple Music, browsers and any app in Windows' media controls.">
        <Switch checked={m.allowPhone} onChange={(v) => void update({ media: { allowPhone: v } })} label="Phone music control" />
      </Row>
      <Row title="Find lyrics online" hint="Looks the song up in LRCLIB, a free lyrics database (only the title, artist and album are sent). Found lyrics are kept on this PC.">
        <Switch checked={m.lyricsOnline} onChange={(v) => void update({ media: { lyricsOnline: v } })} label="Online lyrics" />
      </Row>
      <Row title="Bass and treble" hint={eq?.available ? `Through Equalizer APO${eq.hooked ? '' : ' (OmniHub adds one line to its config the first time)'}. Also adjustable from the phone.` : 'Needs the free Equalizer APO — Windows has no system-wide bass control of its own.'} stack>
        {eq?.available ? (
          <div className="flex flex-col gap-3">
            <label className="flex items-center gap-2 text-[13px] text-dim">
              <Switch checked={m.eqEnabled} onChange={(v) => void update({ media: { eqEnabled: v } })} label="Equaliser on" /> Equaliser on
            </label>
            {(['bassDb', 'trebleDb'] as const).map((k) => (
              <div key={k} className="flex items-center gap-3">
                <span className="w-14 text-[13px] text-dim">{k === 'bassDb' ? 'Bass' : 'Treble'}</span>
                <input type="range" min={-12} max={12} step={0.5} value={m[k]} onChange={(e) => void update({ media: { [k]: Number(e.target.value), eqEnabled: true } }, { silent: true })} className="flex-1 accent-[var(--accent)]" aria-label={k === 'bassDb' ? 'Bass' : 'Treble'} />
                <span className="w-16 text-right font-mono text-[12px] tabular text-fg">{m[k] > 0 ? '+' : ''}{m[k].toFixed(1)} dB</span>
              </div>
            ))}
          </div>
        ) : (
          <Button size="sm" icon={ExternalLink} onClick={() => void api.app.openUrl('https://sourceforge.net/projects/equalizerapo/')}>
            Get Equalizer APO
          </Button>
        )}
      </Row>
    </Section>
  );
}

const PRIVACY_WHAT: Record<PrivacyUse['what'], string> = { microphone: 'Microphone', webcam: 'Camera', location: 'Location' };

/** Which programs used the microphone, camera or location, from Windows' own record. */
function PrivacyDashboard() {
  const [list, setList] = useState<PrivacyUse[] | null>(null);
  const [open, setOpen] = useState(false);
  useEffect(() => {
    if (open) void api.privacy.list().then(setList, () => setList([]));
  }, [open]);
  const live = (list ?? []).filter((u) => u.inUse);
  return (
    <Row
      stack
      title={
        <span className="flex items-center gap-2">
          Privacy dashboard {live.length > 0 && <Badge tone="warn">{live.length} in use now</Badge>}
        </span>
      }
      hint="Which programs used your microphone, camera or location, and when. Read from Windows on this PC; nothing is sent anywhere."
    >
      {!open ? (
        <Button size="sm" onClick={() => setOpen(true)}>
          Show
        </Button>
      ) : !list ? (
        <span className="text-[12.5px] text-faint">Reading…</span>
      ) : !list.length ? (
        <span className="text-[12.5px] text-faint">Windows hasn’t recorded any use yet.</span>
      ) : (
        <div className="mt-2 max-h-72 w-full overflow-auto rounded-xl border border-line">
          {list.map((u) => (
            <div key={`${u.what}-${u.id}`} className="flex items-center gap-3 border-b border-line px-3 py-1.5 text-[12.5px] last:border-0">
              <Badge tone={u.inUse ? 'warn' : 'neutral'}>{PRIVACY_WHAT[u.what]}</Badge>
              <span className="min-w-0 flex-1 truncate text-fg" title={u.id}>
                {u.app}
              </span>
              <span className="shrink-0 text-faint">{u.inUse ? 'using it now' : `last ${formatRelative(u.started)}`}</span>
            </div>
          ))}
        </div>
      )}
    </Row>
  );
}

/** Minecraft theme: the items that float by come from a pack of pictures on this PC (shared with Aurora's lyrics). */
/** Your own picture behind one of the Bleach-inspired looks. */
function ThemePictureRow({ look }: { look: string }) {
  const [url, setUrl] = useState<string | null>(null);
  const [prefs, setPrefs] = useState(() => artPrefs(look));
  useEffect(() => {
    setPrefs(artPrefs(look));
    void api.themeArt.get(look).then(setUrl, () => setUrl(null));
  }, [look]);
  const changed = () => window.dispatchEvent(new Event('omnihub:theme-art'));
  const choose = async () => {
    try {
      const [path] = await api.app.pickFiles('Pick a picture for this theme');
      if (!path) return;
      setUrl(await api.themeArt.set(look, path));
      changed();
    } catch (e) {
      toast.error('Couldn’t use that picture', errorText(e));
    }
  };
  const patch = (x: Partial<ReturnType<typeof artPrefs>>) => {
    setArtPrefs(look, x);
    setPrefs(artPrefs(look));
  };
  return (
    <Row title="Your picture" stack hint="Optional: a picture of your own, shown faded behind the cards. OmniHub ships no artwork from any series, so use a picture you have the right to use (your own, or one its maker lets you download). It stays on this PC.">
      <div className="flex items-center gap-4">
        <div className="relative h-24 w-24 shrink-0 overflow-hidden rounded-xl border border-line bg-surface-2">{url ? <img src={url} alt="" className="h-full w-full object-cover" /> : <div className="grid h-full place-items-center px-2 text-center text-[12px] text-faint">No picture</div>}</div>
        <div className="min-w-0 flex-1 space-y-2.5">
          <div className="flex gap-2">
            <Button variant="primary" onClick={() => void choose()}>
              {url ? 'Change picture' : 'Choose a picture'}
            </Button>
            {url && (
              <Button
                variant="ghost"
                onClick={() =>
                  void api.themeArt.set(look, null).then(() => {
                    setUrl(null);
                    changed();
                  })
                }
              >
                Remove
              </Button>
            )}
          </div>
          {url && (
            <div className="flex flex-wrap items-center gap-x-5 gap-y-2 text-[12.5px] text-dim">
              <label className="flex items-center gap-2">
                Strength
                <input type="range" min={10} max={80} value={Math.round(prefs.opacity * 100)} onChange={(e) => patch({ opacity: Number(e.target.value) / 100 })} className="w-32 accent-[var(--accent)]" />
              </label>
              <label className="flex items-center gap-2">
                Side
                <Select value={prefs.side} onChange={(e) => patch({ side: e.target.value as 'left' | 'center' | 'right' })} className="w-28" aria-label="Side">
                  <option value="left">Left</option>
                  <option value="center">Centre</option>
                  <option value="right">Right</option>
                </Select>
              </label>
              <label className="flex items-center gap-2">
                Style
                <Select value={prefs.style} onChange={(e) => patch({ style: e.target.value as 'tint' | 'ink' | 'color' })} className="w-36" aria-label="Style">
                  <option value="color">Its own colours</option>
                  <option value="tint">Theme colour</option>
                  <option value="ink">Ink lines</option>
                </Select>
              </label>
            </div>
          )}
        </div>
      </div>
    </Row>
  );
}

function PhotoRow() {
  const wall = useWallpaper();
  const prefs = usePhotoPrefs();
  const [gallery, setGallery] = useState<PcWall[]>([]);
  const loadGallery = () => void api.wallpaper.gallery().then(setGallery, () => undefined);
  useEffect(loadGallery, [wall?.version]);
  const [busy, setBusy] = useState(false);
  const choose = async () => {
    try {
      const [path] = await api.app.pickFiles('Pick a photo for the wallpaper');
      if (!path) return;
      setBusy(true);
      const w = await api.wallpaper.set('pc', path);
      setWallpaperLocal(w);
      if (w) toast.success('Wallpaper ready', wallSummary(w));
    } catch (e) {
      toast.error('Couldn’t use that photo', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  const remove = async () => {
    setWallpaperLocal(await api.wallpaper.set('pc', null).catch(() => null));
  };
  return (
    <Row title="Your photo" stack hint="Any photo: it fills the whole window and is sized for 4K (small photos are enlarged and sharpened, which keeps edges clean but can’t add detail that isn’t there). The theme’s colours come from it, and your phone can use it too. It stays on this PC.">
      <div className="flex items-center gap-4">
        <div className="relative h-24 w-40 shrink-0 overflow-hidden rounded-xl border border-line bg-surface-2">{wall ? <img src={wall.url} alt="" className="h-full w-full object-cover" /> : <div className="grid h-full place-items-center text-[12px] text-faint">No photo yet</div>}</div>
        <div className="min-w-0 flex-1 space-y-2">
          {wall && (
            <>
              <div className="text-[12.5px] text-dim">{wallSummary(wall)}</div>
              <div className="flex items-center gap-2 text-[12px] text-faint">
                Colours from it:
                {[wall.palette.accent, wall.palette.accent2, wall.palette.tint].map((c) => (
                  <span key={c} className="h-5 w-5 rounded-full border border-line" style={{ background: c }} title={c} />
                ))}
                <span>{wall.palette.light ? 'light' : 'dark'}</span>
              </div>
            </>
          )}
          <div className="flex gap-2">
            <Button variant="primary" onClick={() => void choose()} disabled={busy}>
              {busy ? 'Preparing…' : wall ? 'Change photo' : 'Choose a photo'}
            </Button>
            {wall && (
              <Button variant="ghost" onClick={() => void remove()}>
                No photo
              </Button>
            )}
          </div>
        </div>
      </div>
      {gallery.length > 0 && (
        <div className="mt-4">
          <div className="mb-2 text-[12.5px] font-medium text-dim">Your photos ({gallery.length})</div>
          <div className="flex flex-wrap gap-2.5">
            {gallery.map((g) => {
              const on = !!g.source && g.source === wall?.source;
              return (
                <div key={g.source} className="group relative">
                  <button type="button" aria-pressed={on} aria-label="Use this photo" onClick={() => g.source && void api.wallpaper.select(g.source).then(setWallpaperLocal, (e: unknown) => toast.error('Couldn’t use that photo', errorText(e)))} className={cx('block h-16 w-28 overflow-hidden rounded-lg border transition', on ? 'accent-ring border-transparent' : 'border-line hover:border-line-strong')}>
                    <img src={g.url} alt="" loading="lazy" className="h-full w-full object-cover" />
                  </button>
                  {!on && (
                    <button type="button" aria-label="Remove this photo" onClick={() => g.source && void api.wallpaper.remove(g.source).then(loadGallery)} className="absolute -right-1.5 -top-1.5 hidden h-5 w-5 place-items-center rounded-full border border-line-strong bg-elev text-[11px] text-dim hover:text-fg group-hover:grid">
                      ×
                    </button>
                  )}
                </div>
              );
            })}
          </div>
        </div>
      )}
      {wall && (
        <div className="mt-4 flex flex-wrap items-center gap-x-6 gap-y-3 text-[12.5px] text-dim">
          <label className="flex items-center gap-2">
            Darken
            <input type="range" min={0} max={70} value={prefs.dim} onChange={(e) => setPhotoPrefs({ dim: Number(e.target.value) })} className="w-32 accent-[var(--accent)]" />
          </label>
          <label className="flex items-center gap-2">
            Blur
            <input type="range" min={0} max={24} value={prefs.blur} onChange={(e) => setPhotoPrefs({ blur: Number(e.target.value) })} className="w-32 accent-[var(--accent)]" />
          </label>
          <label className="flex items-center gap-2">
            Slow drift
            <Switch checked={prefs.drift} onChange={(v) => setPhotoPrefs({ drift: v })} label="Slow drift" />
          </label>
          <label className="flex items-center gap-2">
            Change photo
            <Select value={String(prefs.rotate)} onChange={(e) => setPhotoPrefs({ rotate: Number(e.target.value) })} className="w-36" aria-label="Change photo" disabled={gallery.length < 2}>
              <option value="0">Never</option>
              <option value="15">Every 15 min</option>
              <option value="60">Every hour</option>
              <option value="1440">Every day</option>
            </Select>
          </label>
        </div>
      )}
    </Row>
  );
}

function ItemPackRow() {
  const { look, textures } = useAuroraLook();
  const [busy, setBusy] = useState(false);
  const count = look?.textures.length ?? 0;
  const choose = async () => {
    setBusy(true);
    try {
      const pack = await api.media.chooseEmojiPack();
      if (pack) {
        forgetPictures();
        window.dispatchEvent(new Event(LOOK_CHANGED));
        toast.success('Item pack ready', pack.kind === 'font' ? 'That’s an emoji font: the theme needs a zip of pictures.' : `${pack.count.toLocaleString()} pictures`);
      }
    } catch (e) {
      toast.error('Could not use that pack', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  const has = textures ? ['diamond', 'emerald', 'golden_apple', 'apple', 'diamond_sword'].some((n) => textures.names.has(n)) : false;
  return (
    <Row
      title="Item pack"
      hint={
        count
          ? has
            ? `${count.toLocaleString()} pictures. Its items float by like dropped items (the same pack Aurora's lyrics use).`
            : `${count.toLocaleString()} pictures, but none named like Minecraft items (diamond.png, apple.png…), so XP orbs float instead.`
          : 'Pick a zip of Minecraft item pictures (diamond.png, apple.png…) and they float by like dropped items. It stays on this PC; without one, XP orbs float instead.'
      }
    >
      <Button size="sm" icon={FolderOpen} loading={busy} onClick={() => void choose()}>
        {count ? 'Change…' : 'Choose a zip…'}
      </Button>
    </Row>
  );
}

/** The Discord status: the game you play and your shiny hunt, under your name in Discord. */
function DiscordRows() {
  const [st, setSt] = useState<PresenceStatus | null>(null);
  const [id, setId] = useState('');
  useEffect(() => {
    void api.presence.status().then((x) => {
      setSt(x);
      setId(x.settings.clientId);
    }, () => undefined);
  }, []);
  if (!st) return null;
  const save = async (p: Partial<PresenceSettings>) => {
    try {
      const next = await api.presence.save({ ...st.settings, clientId: id, ...p });
      setSt(next);
      toast.saved();
    } catch (e) {
      toast.error('Not saved', errorText(e));
    }
  };
  const s = st.settings;
  return (
    <>
      <Row title="Discord status" hint={st.showing ? `Showing: ${st.showing.details}${st.showing.state ? ` — ${st.showing.state}` : ''}` : st.error && s.enabled ? st.error : 'Show the game you’re playing and your shiny hunt under your name in Discord (the desktop app).'}>
        <Switch checked={s.enabled} onChange={(v) => void save({ enabled: v })} label="Discord status" />
      </Row>
      <Row
        title="Discord Application ID"
        stack
        hint={
          <>
            Discord only shows statuses for an app, so make your own (free): open discord.com/developers/applications, click New Application, name it what Discord should show after “Playing” (like “OmniHub”), and copy its Application ID here.
          </>
        }
      >
        <div className="flex flex-wrap items-center gap-2">
          <TextInput value={id} onChange={(e) => setId(e.target.value.replace(/[^0-9]/g, ''))} placeholder="1234567890123456789" className="w-[260px]" inputMode="numeric" aria-label="Discord Application ID" />
          <Button size="sm" variant="secondary" onClick={() => void save({})} disabled={id === s.clientId}>
            Save
          </Button>
          <Button size="sm" variant="ghost" icon={ExternalLink} onClick={() => void api.app.openUrl('https://discord.com/developers/applications')}>
            Developer Portal
          </Button>
        </div>
        <div className="flex flex-wrap gap-4 pt-1 text-[13px]">
          <label className="flex items-center gap-2">
            <Switch checked={s.showGame} onChange={(v) => void save({ showGame: v })} label="Show the game" /> The game
          </label>
          <label className="flex items-center gap-2">
            <Switch checked={s.showHunt} onChange={(v) => void save({ showHunt: v })} label="Show the shiny hunt" /> The shiny hunt
          </label>
        </div>
      </Row>
    </>
  );
}

/** Controller mode: the whole app with a gamepad. */
function ControllerRow() {
  const [on, setOn] = useState(controllerModeOn);
  return (
    <Row title="Controller mode" hint="Move around OmniHub with an Xbox or other controller: D-pad or stick to move, A to press, B to go back, LB / RB to switch pages, Start to search. Only while OmniHub is in front.">
      <Switch
        checked={on}
        onChange={(v) => {
          setControllerMode(v);
          setOn(v);
        }}
        label="Controller mode"
      />
    </Row>
  );
}
