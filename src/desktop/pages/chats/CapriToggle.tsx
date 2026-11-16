// Capri on and off: the desktop companion who narrates what Claude Code,
// Codex or Antigravity are doing (github.com/mochiiwuff/capri-desktop).
// The first time on installs her official release; OmniHub then starts her
// with itself until she is switched off.

import type { CapriStatus, Settings } from '@shared/types';
import { Download, LoaderCircle } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText, on } from '../../api';
import { Button } from '../../components/ui/Button';
import { Switch } from '../../components/ui/Form';
import { useEvent } from '../../lib/hooks';
import { toast } from '../../state/toasts';

let checked = false;

export function CapriToggle() {
  const [enabled, setEnabled] = useState<boolean | null>(null);
  const [status, setStatus] = useState<CapriStatus | null>(null);
  const [busy, setBusy] = useState<string | null>(null);

  useEffect(() => {
    void api.app.settings().then((s) => setEnabled(s.chats.capri), () => undefined);
    // Her newest version once per run (GitHub is asked only when she's installed).
    void api.capri.status(false).then((st) => {
      setStatus(st);
      if (st.installed && !checked) {
        checked = true;
        void api.capri.status(true).then(setStatus, () => undefined);
      }
    }, () => undefined);
    const off = on<{ text: string }>('capri:progress', (p) => setBusy(p.text));
    return () => void off.then((f) => f());
  }, []);
  useEvent<Settings>('settings:changed', (s) => setEnabled(s.chats.capri));

  const toggle = async (next: boolean) => {
    setBusy(next ? (status?.installed ? 'Starting Capri' : 'Getting Capri') : 'Closing Capri');
    try {
      const st = await api.capri.set(next);
      setStatus((s) => ({ ...st, latest: s?.latest ?? null }));
      setEnabled(next);
      if (next && !status?.installed) toast.success('Capri is here', 'Right-click her for her menu. “Plug me in” connects her to Claude Code.');
    } catch (e) {
      toast.error(next ? 'Capri didn’t start' : 'Capri didn’t close', errorText(e));
    } finally {
      setBusy(null);
    }
  };
  const update = async () => {
    setBusy('Updating Capri');
    try {
      setStatus(await api.capri.update());
      toast.success('Capri is up to date');
    } catch (e) {
      toast.error('Capri wasn’t updated', errorText(e));
    } finally {
      setBusy(null);
    }
  };
  const canUpdate = !!(status?.installed && status.latest && status.version && status.latest !== status.version);

  return (
    <div className="flex items-center gap-2.5 rounded-xl border border-line bg-surface px-3 py-1.5" title="Capri, a desktop companion who narrates what Claude Code, Codex or Antigravity are doing. By mochiiwuff, free for non-commercial use. The first time, OmniHub installs her official release from GitHub (about 100 MB).">
      {busy ? <LoaderCircle size={15} className="animate-spin text-accent" /> : <span className={`h-2 w-2 rounded-full ${status?.running ? 'bg-good' : 'bg-faint'}`} aria-hidden />}
      <div className="leading-tight">
        <div className="text-[13px] font-medium text-fg">Capri</div>
        <div className="max-w-[180px] truncate text-[11px] text-faint">{busy ? `${busy}…` : status?.running ? `On${status.version ? ` · ${status.version}` : ''}` : status?.installed ? 'Off' : 'Desktop companion'}</div>
      </div>
      {canUpdate && !busy && (
        <Button size="sm" variant="ghost" icon={Download} onClick={() => void update()} title={`Update Capri to ${status?.latest}`}>
          {status?.latest}
        </Button>
      )}
      <Switch checked={!!enabled} disabled={enabled === null || !!busy} onChange={(v) => void toggle(v)} label="Capri" />
    </div>
  );
}
