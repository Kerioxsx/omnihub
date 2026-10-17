// Subscribes once to app-wide backend events and keeps the shared stores fresh.

import { useEffect } from 'react';
import type { Device, JobProgress, PendingPower, Screenshot, ScanSummary, ServerStatus, Settings, TransferEvent, ViewerInfo } from '@shared/types';
import { formatBytes } from '@shared/format';
import { api } from '../api';
import { useEvent } from '../lib/hooks';
import { currentRoute, navigate } from '../lib/router';
import { useLive } from '../state/live';
import { useSend } from '../state/send';
import { useSettings } from '../state/settings';
import { useStorage } from '../state/storage';
import { toast } from '../state/toasts';

export function GlobalEvents() {
  useEffect(() => {
    const live = useLive.getState();
    void Promise.allSettled([live.refreshRemote(), live.refreshDevices(), live.refreshVault(), live.refreshPower(), useStorage.getState().loadVolumes()]);
    // Started from Explorer's "Send to → OmniHub (phone)".
    void api.app.takePendingSend().then((paths) => paths.length && useSend.getState().openFiles(paths), () => undefined);
    // The previous run ended without a normal exit: offer what is known.
    void api.app.lastCrash().then((report) => {
      if (!report) return;
      toast.warn('OmniHub closed unexpectedly last time', 'Copy the details and send them along with what you were doing, so it can be fixed.', {
        key: 'last-crash',
        duration: 60_000,
        action: { label: 'Copy details', run: () => void navigator.clipboard?.writeText(report).then(() => toast.success('Details copied')) },
      });
    }, () => undefined);
  }, []);
  useEvent<string[]>('send:files', () => void api.app.takePendingSend().then((paths) => paths.length && useSend.getState().openFiles(paths), () => undefined));
  useEvent<{ device: string; chars: number }>('clipboard:from-phone', (p) => toast.success(`Text from ${p.device} copied`, `${p.chars} characters are on the clipboard — paste with Ctrl+V.`));

  // storage
  useEvent<JobProgress>('storage:progress', (p) => useStorage.getState().onProgress(p));
  useEvent<{ jobId: string; summary: ScanSummary }>('storage:done', (p) => useStorage.getState().onDone(p.jobId, p.summary));
  useEvent<{ jobId: string; error: string; cancelled: boolean }>('storage:failed', (p) => useStorage.getState().onFailed(p.jobId, p.error, p.cancelled));
  useEvent('storage:deleted', () => useStorage.getState().bump());

  // phone companion
  useEvent<ServerStatus>('remote:status', (s) => useLive.getState().setRemote(s));
  useEvent<Device[]>('remote:devices', (d) => useLive.getState().setDevices(d));
  useEvent<ViewerInfo[]>('screen:viewers', (v) => useLive.getState().setViewers(v));
  useEvent<Device>('remote:paired', (d) => {
    toast.success(`${d.name} is paired`, `It can now reach this PC from ${d.lastIp}.`);
    void useLive.getState().refreshDevices();
  });
  useEvent<TransferEvent>('transfer:started', (e) => useLive.getState().transferEvent('started', e));
  useEvent<TransferEvent>('transfer:progress', (e) => useLive.getState().transferEvent('progress', e));
  useEvent<TransferEvent>('transfer:cancelled', (e) => useLive.getState().transferEvent('cancelled', e));
  useEvent<TransferEvent>('transfer:done', (e) => {
    useLive.getState().transferEvent('done', e);
    if (e.direction === 'upload') {
      const path = e.path;
      toast.success(`Received ${e.name}`, `${formatBytes(e.size ?? 0)} from ${e.device ?? 'your phone'}`, path ? { action: { label: 'Show in folder', run: () => void api.app.revealPath(path) } } : {});
    }
  });

  // power
  useEvent<PendingPower>('power:pending', (p) => useLive.getState().setPower(p));
  useEvent<{ id: string; reason: string }>('power:cancelled', (p) => {
    useLive.getState().setPower(null);
    toast.info('Power action cancelled', p.reason);
  });
  useEvent<{ action: string; ok: boolean; error: string | null; dryRun: boolean }>('power:executed', (p) => {
    useLive.getState().setPower(null);
    if (!p.ok) toast.error('Power action failed', p.error ?? undefined);
    else if (p.dryRun) toast.info('Power action ran in dry-run mode', 'Nothing was actually shut down.');
  });

  // vault
  useEvent<{ reason: string }>('vault:locked', (p) => {
    void useLive.getState().refreshVault();
    if (p.reason !== 'manual') toast.info('Vault locked', p.reason === 'idle' ? 'Locked automatically after inactivity.' : p.reason === 'session' ? 'Windows was locked.' : undefined);
  });
  useEvent('vault:unlocked', () => void useLive.getState().refreshVault());
  useEvent<{ id: string; title: string; snippet: string }>('notes:reminder', (p) =>
    toast.info(`Reminder: ${p.title || 'Untitled note'}`, p.snippet || undefined, { action: { label: 'Open', run: () => navigate('notes', { id: p.id }) } }),
  );
  useEvent<{ browser: string }>('browser:unlock-request', (p) => {
    navigate('vault');
    toast.info(`${p.browser} wants to fill a login`, 'Unlock the vault here, then go back to the browser.');
  });

  // misc
  useEvent<Settings>('settings:changed', (s) => useSettings.getState().set(s));
  useEvent<string>('app:navigate', (route) => navigate(route));
  useEvent<Screenshot>('screenshots:new', (s) => {
    if (currentRoute().id === 'screenshots') return;
    toast.success('Screenshot saved', `${s.width} × ${s.height}${s.appTitle ? ` · ${s.appTitle}` : ''}`, { action: { label: 'View', run: () => navigate('screenshots', { open: s.id }) } });
  });
  return null;
}

export function ThemeController() {
  const theme = useSettings((s) => s.settings?.general.theme);
  const accent = useSettings((s) => s.settings?.general.accent);
  const reduced = useSettings((s) => s.settings?.general.reducedMotion);
  useEffect(() => {
    if (!theme) return;
    const root = document.documentElement;
    const mq = window.matchMedia('(prefers-color-scheme: light)');
    const apply = () => {
      const light = theme === 'light' || (theme === 'system' && mq.matches);
      root.classList.toggle('light', light);
      root.style.colorScheme = light ? 'light' : 'dark';
    };
    apply();
    if (theme !== 'system') return;
    mq.addEventListener('change', apply);
    return () => mq.removeEventListener('change', apply);
  }, [theme]);
  useEffect(() => {
    if (accent) document.documentElement.dataset.accent = accent;
  }, [accent]);
  useEffect(() => {
    document.documentElement.classList.toggle('reduce-motion', !!reduced);
  }, [reduced]);
  // Interface size: CSS zoom on the root scales layout like browser zoom.
  const scale = useSettings((s) => s.settings?.general.uiScale ?? 100);
  useEffect(() => {
    const z = Math.min(150, Math.max(80, scale)) / 100;
    document.documentElement.style.zoom = z === 1 ? '' : String(z);
  }, [scale]);
  // Ctrl+= / Ctrl+- / Ctrl+0 change it from anywhere.
  useEffect(() => {
    const steps = [80, 90, 100, 110, 125, 150];
    const onKey = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey) || e.altKey) return;
      const cur = useSettings.getState().settings?.general.uiScale ?? 100;
      const i = steps.indexOf(cur) < 0 ? 2 : steps.indexOf(cur);
      const next = e.key === '=' || e.key === '+' ? steps[Math.min(steps.length - 1, i + 1)] : e.key === '-' ? steps[Math.max(0, i - 1)] : e.key === '0' ? 100 : null;
      if (next == null) return;
      e.preventDefault();
      if (next !== cur) void useSettings.getState().update({ general: { uiScale: next } });
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);
  return null;
}
