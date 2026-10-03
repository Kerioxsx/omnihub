import type { BrowseScope } from '@shared/types';
import type { LucideIcon } from 'lucide-react';
import { AppWindow, FolderOpen, FolderPlus, KeyRound, MousePointer2, NotebookPen, Power, RotateCcw, ScreenShare, ShieldAlert, Upload, X } from 'lucide-react';
import type { ReactNode } from 'react';
import { api } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Card, CardHeader } from '../../components/ui/Card';
import { Segmented, Select, Switch } from '../../components/ui/Form';
import { Callout } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { useSettings } from '../../state/settings';

function Toggle({ icon: Icon, title, hint, checked, onChange, disabled, children, tone }: { icon: LucideIcon; title: string; hint: ReactNode; checked: boolean; onChange: (v: boolean) => void; disabled?: boolean; children?: ReactNode; tone?: 'warn' }) {
  return (
    <div className={cx('py-3', disabled && 'opacity-60')}>
      <div className="flex items-center gap-3">
        <span className={cx('flex h-8 w-8 shrink-0 items-center justify-center rounded-[10px]', tone === 'warn' && checked ? 'bg-warn/15 text-warn' : 'bg-surface-3 text-dim')}>
          <Icon size={15} aria-hidden />
        </span>
        <div className="min-w-0 flex-1">
          <div className="text-[13.5px] font-medium text-fg">{title}</div>
          <div className="text-[12px] leading-snug text-faint">{hint}</div>
        </div>
        <Switch checked={checked} onChange={onChange} label={title} disabled={disabled} />
      </div>
      {children && <div className="ml-11 mt-2">{children}</div>}
    </div>
  );
}

export function Permissions() {
  const s = useSettings((st) => st.settings);
  const info = useSettings((st) => st.info);
  const update = useSettings((st) => st.update);
  if (!s) return null;
  const r = s.remote;

  const addRoot = async () => {
    const f = await api.app.pickFolder('Choose a folder phones may browse');
    if (f && !r.customRoots.includes(f)) await update({ remote: { customRoots: [...r.customRoots, f] } });
  };

  return (
    <div className="space-y-3">
      <Card className="p-5">
        <CardHeader icon={ShieldAlert} title="What paired phones may do" subtitle="Changes apply immediately to every paired phone." />
        <div className="mt-2 divide-y divide-line">
          <div className="py-3">
            <div className="flex items-center gap-3">
              <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-[10px] bg-surface-3 text-dim">
                <FolderOpen size={15} aria-hidden />
              </span>
              <div className="min-w-0 flex-1">
                <div className="text-[13.5px] font-medium text-fg">Browse files</div>
                <div className="text-[12px] text-faint">Which folders phones can list and download from</div>
              </div>
            </div>
            <div className="ml-11 mt-2.5 space-y-2">
              <Segmented<BrowseScope>
                size="sm"
                label="Browse scope"
                value={r.browseScope}
                onChange={(v) => void update({ remote: { browseScope: v } })}
                options={[
                  { value: 'userFolders', label: 'Your folders' },
                  { value: 'allDrives', label: 'All drives' },
                  { value: 'custom', label: 'Custom' },
                ]}
              />
              <div className="text-[12px] text-faint">{r.browseScope === 'userFolders' ? 'Desktop, Documents, Downloads, Pictures, Videos and Music.' : r.browseScope === 'allDrives' ? 'Every drive on this PC, except system folders.' : 'Only the folders listed below.'}</div>
              {r.browseScope === 'custom' && (
                <div className="space-y-1.5">
                  {r.customRoots.map((p) => (
                    <div key={p} className="flex items-center gap-2 rounded-lg border border-line bg-surface py-0.5 pl-3 pr-0.5">
                      <span className="min-w-0 flex-1 truncate font-mono text-[12px] text-dim">{p}</span>
                      <IconButton icon={X} label={`Remove ${p}`} size="sm" onClick={() => void update({ remote: { customRoots: r.customRoots.filter((x) => x !== p) } })} />
                    </div>
                  ))}
                  <Button size="xs" icon={FolderPlus} onClick={() => void addRoot()}>
                    Add folder
                  </Button>
                </div>
              )}
            </div>
          </div>
          <Toggle icon={Upload} title="Receive files" hint="Phones can upload photos and files to the incoming folder" checked={r.allowUploads} onChange={(v) => void update({ remote: { allowUploads: v } })} />
          <Toggle icon={Power} title="Power actions" hint="Shut down, restart, sleep or lock — always with a countdown you can cancel here" checked={r.allowPower} onChange={(v) => void update({ remote: { allowPower: v } })}>
            {r.allowPower && (
              <div className="flex items-center gap-2 text-[12.5px] text-dim">
                Countdown
                <Select value={String(r.powerDelaySeconds)} onChange={(e) => void update({ remote: { powerDelaySeconds: Number(e.target.value) } })} className="w-[132px]" aria-label="Power countdown">
                  {[5, 10, 20, 30, 60, 120].map((n) => (
                    <option key={n} value={n}>
                      {n} seconds
                    </option>
                  ))}
                </Select>
              </div>
            )}
          </Toggle>
          <Toggle icon={ScreenShare} title="Watch the screen" hint="Stream the desktop to the phone browser" checked={r.allowScreen} onChange={(v) => void update({ remote: { allowScreen: v } })} />
          <Toggle icon={MousePointer2} title="Remote control" hint="Mouse and keyboard from the phone while watching" checked={r.allowControl} disabled={!r.allowScreen} tone="warn" onChange={(v) => void update({ remote: { allowControl: v } })}>
            {r.allowControl && (
              <Callout tone="warn" className="py-2">
                A paired phone can then do anything you can do at this PC. Only enable it for phones you trust.
              </Callout>
            )}
          </Toggle>
          <Toggle icon={AppWindow} title="Launch apps" hint="Start installed apps from the phone" checked={r.allowAppLaunch} onChange={(v) => void update({ remote: { allowAppLaunch: v } })} />
          <Toggle icon={NotebookPen} title="Notes & ideas" hint="Read and write notes from the phone" checked={r.allowNotes} onChange={(v) => void update({ remote: { allowNotes: v } })} />
          <Toggle icon={KeyRound} title="Vault access" hint={r.tls ? 'List and reveal entries after typing the master password on the phone' : 'Needs HTTPS — turn it on in Settings → Phone'} checked={s.vault.allowPhone && r.tls} disabled={!r.tls} tone="warn" onChange={(v) => void update({ vault: { allowPhone: v } })} />
        </div>
      </Card>
      <Card className="p-5">
        <CardHeader icon={FolderOpen} title="Incoming folder" subtitle="Where files from phones are saved" />
        <div className="mt-3 flex items-center gap-1 rounded-xl border border-line bg-surface py-1 pl-3 pr-1">
          <span className="min-w-0 flex-1 truncate font-mono text-[12px] text-dim">{r.incomingDir ?? info?.incomingDir ?? 'Downloads\\OmniHub'}</span>
          <IconButton icon={FolderOpen} label="Open folder" size="sm" onClick={() => void api.app.openPath(r.incomingDir ?? info?.incomingDir ?? '')} />
          {r.incomingDir && <IconButton icon={RotateCcw} label="Reset to default" size="sm" onClick={() => void update({ remote: { incomingDir: null } })} />}
        </div>
        <Button
          size="sm"
          className="mt-2.5"
          onClick={async () => {
            const f = await api.app.pickFolder('Choose the incoming folder');
            if (f) await update({ remote: { incomingDir: f } });
          }}
        >
          Change…
        </Button>
      </Card>
    </div>
  );
}
