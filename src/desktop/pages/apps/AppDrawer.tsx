import { formatBytes, formatDate, formatRelative } from '@shared/format';
import type { AppInfo } from '@shared/types';
import { Camera, Copy, FolderOpen, Play, Trash, X } from 'lucide-react';
import { useRef, useState } from 'react';
import { api, errorText } from '../../api';
import { AppIcon } from '../../components/AppIcon';
import { ShotThumb } from '../../components/ShotThumb';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Meta, SectionTitle, Skeleton } from '../../components/ui/Card';
import { Drawer } from '../../components/ui/Overlay';
import { useAsync } from '../../lib/hooks';
import { navigate } from '../../lib/router';
import { copyText } from '../../lib/util';
import { confirm } from '../../state/dialogs';
import { toast } from '../../state/toasts';
import { SOURCE_LABEL } from './shared';

function Body({ app, onClose }: { app: AppInfo; onClose: () => void }) {
  const shots = useAsync(() => api.apps.screenshots(app.id), [app.id]);
  const [busy, setBusy] = useState<'launch' | 'uninstall' | null>(null);

  const launch = async () => {
    setBusy('launch');
    try {
      await api.apps.launch(app.id);
      toast.success(`Starting ${app.name}`);
    } catch (e) {
      toast.error(`Could not start ${app.name}`, errorText(e));
    } finally {
      setBusy(null);
    }
  };

  const uninstall = async () => {
    const ok = await confirm({ title: `Uninstall ${app.name}?`, description: `${app.publisher}'s uninstaller will open. Follow its steps to remove the app${app.size ? ` and free about ${formatBytes(app.size)}` : ''}.`, tone: 'danger', confirmLabel: 'Open uninstaller' });
    if (!ok) return;
    setBusy('uninstall');
    try {
      await api.apps.uninstall(app.id);
      toast.info('Uninstaller started', 'Finish removing the app in its own window.');
    } catch (e) {
      toast.error('Could not start the uninstaller', errorText(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <>
      <div className="relative overflow-hidden border-b border-line px-6 pb-5 pt-6">
        <div className="pointer-events-none absolute -right-16 -top-20 h-56 w-56 rounded-full bg-[radial-gradient(circle,var(--accent-soft),transparent_70%)]" />
        <IconButton icon={X} label="Close" size="sm" onClick={onClose} className="absolute right-4 top-4" />
        <div className="relative flex items-center gap-4">
          <AppIcon id={app.id} name={app.name} size={64} />
          <div className="min-w-0">
            <h2 className="font-display text-[19px] font-semibold leading-tight text-fg">{app.name}</h2>
            <div className="mt-0.5 truncate text-[13px] text-dim">{app.publisher}</div>
            <div className="mt-2 flex flex-wrap gap-1.5">
              <Badge tone={app.source === 'store' ? 'info' : 'accent'}>{SOURCE_LABEL[app.source]}</Badge>
              <Badge>v{app.version}</Badge>
            </div>
          </div>
        </div>
        <div className="relative mt-5 flex gap-2">
          <Button variant="primary" icon={Play} loading={busy === 'launch'} disabled={!app.launchable} onClick={launch} className="flex-1" title={app.launchable ? undefined : 'This entry has no program to start'}>
            Launch
          </Button>
          <Button icon={FolderOpen} disabled={!app.installLocation} onClick={() => app.installLocation && void api.app.openPath(app.installLocation).catch((e: unknown) => toast.error('Could not open the folder', errorText(e)))}>
            Folder
          </Button>
          <Button variant="danger" icon={Trash} loading={busy === 'uninstall'} disabled={!app.uninstallable} onClick={uninstall}>
            Uninstall
          </Button>
        </div>
      </div>
      <div className="flex-1 space-y-6 overflow-y-auto px-6 py-5">
        <div className="grid grid-cols-2 gap-4">
          <Meta label="Size">
            {app.size != null ? (
              <span className="inline-flex items-center gap-2">
                {formatBytes(app.size)}
                <Badge tone={app.sizeFromScan ? 'good' : 'neutral'} className="h-[18px] text-[10.5px]">
                  {app.sizeFromScan ? 'from scan' : 'estimated'}
                </Badge>
              </span>
            ) : (
              'Unknown'
            )}
          </Meta>
          <Meta label="Installed">{app.installDate ? `${formatDate(app.installDate)}${Date.now() / 1000 - app.installDate < 30 * 86400 ? ` · ${formatRelative(app.installDate)}` : ''}` : 'Unknown'}</Meta>
          <Meta label="Version" mono>
            {app.version}
          </Meta>
          <Meta label="Publisher">{app.publisher}</Meta>
        </div>
        {app.installLocation && (
          <div>
            <div className="text-[11px] font-medium uppercase tracking-wider text-faint">Location</div>
            <div className="mt-1 flex items-center gap-1 rounded-xl border border-line bg-surface py-1 pl-3 pr-1">
              <span className="min-w-0 flex-1 break-all font-mono text-[11.5px] text-dim">{app.installLocation}</span>
              <IconButton icon={Copy} label="Copy path" size="sm" onClick={() => void copyText(app.installLocation ?? '').then(() => toast.success('Path copied'))} />
              <IconButton icon={FolderOpen} label="Open folder" size="sm" onClick={() => void api.app.openPath(app.installLocation ?? '')} />
            </div>
          </div>
        )}
        {app.aumid && <Meta label="App user model ID" mono>{app.aumid}</Meta>}
        {!app.sizeFromScan && app.size != null && <p className="text-[12px] leading-relaxed text-faint">Sizes marked “estimated” come from the installer's registry entry and are often wrong. Scan the drive on the Storage page to measure the real install folder.</p>}
        <div>
          <SectionTitle>Screenshots in this app</SectionTitle>
          {shots.data ? (
            shots.data.length ? (
              <div className="grid grid-cols-2 gap-2">
                {shots.data.slice(0, 8).map((s) => (
                  <button key={s.id} type="button" onClick={() => navigate('screenshots', { open: s.id })} className="group overflow-hidden rounded-xl border border-line text-left">
                    <ShotThumb id={s.id} alt={s.appTitle ?? 'Screenshot'} className="aspect-video w-full" />
                    <div className="px-2 py-1 text-[11px] text-faint">{formatRelative(s.created)}</div>
                  </button>
                ))}
              </div>
            ) : (
              <div className="flex items-center gap-2 rounded-xl border border-dashed border-line px-3 py-4 text-[12.5px] text-faint">
                <Camera size={15} aria-hidden /> No screenshots taken in {app.name} yet.
              </div>
            )
          ) : (
            <div className="grid grid-cols-2 gap-2">
              <Skeleton className="aspect-video" />
              <Skeleton className="aspect-video" />
            </div>
          )}
        </div>
      </div>
    </>
  );
}

export function AppDrawer({ app, onClose }: { app: AppInfo | null; onClose: () => void }) {
  // Keep showing the last app while the drawer slides out.
  const last = useRef<AppInfo | null>(app);
  if (app) last.current = app;
  const shown = app ?? last.current;
  return (
    <Drawer open={!!app} onClose={onClose} label={shown ? `${shown.name} details` : 'App details'} width={460}>
      {shown && <Body key={shown.id} app={shown} onClose={onClose} />}
    </Drawer>
  );
}
