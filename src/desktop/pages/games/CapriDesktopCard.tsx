// Capri Desktop (capriai.app): a free, fan-made companion by lachydotmcg
// who sits on your windows and chats. OmniHub never ships it: Install
// downloads the developer's own installer from their GitHub releases. Here
// you open and close it, have it start with OmniHub, and get Ollama ready
// for its local brain.

import type { CapriDesktopStatus } from '@shared/types';
import { Bot, Download, ExternalLink, Play, Square } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Badge, Card, CardHeader } from '../../components/ui/Card';
import { Switch } from '../../components/ui/Form';
import { ProgressBar } from '../../components/ui/Progress';
import { useEvent } from '../../lib/hooks';
import { toast } from '../../state/toasts';
import { Row } from './shared';

const MODEL = 'gemma4:e4b';

export function CapriDesktopCard() {
  const [st, setSt] = useState<CapriDesktopStatus | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [dl, setDl] = useState<number | null>(null);
  const [pull, setPull] = useState<number | null>(null);
  const load = useCallback(() => void api.capriDesktop.status().then(setSt, () => undefined), []);
  useEffect(load, [load]);
  useEvent<{ done: number; total: number }>('capri:download', (p) => setDl(p.total ? (p.done / p.total) * 100 : null));
  useEvent<{ status: string; done: number; total: number }>('capri:pull', (p) => setPull(p.total ? (p.done / p.total) * 100 : null));

  const run = async (what: string, f: () => Promise<unknown>, ok?: string) => {
    setBusy(what);
    try {
      await f();
      if (ok) toast.success(ok);
    } catch (e) {
      toast.error('Couldn’t do that', errorText(e));
    } finally {
      setBusy(null);
      setDl(null);
      setPull(null);
      load();
    }
  };

  const o = st?.ollama;
  return (
    <Card className="overflow-hidden">
      <CardHeader
        icon={Bot}
        title="Capri Desktop"
        subtitle="A free, fan-made companion by lachydotmcg (capriai.app) who sits on your windows and chats. OmniHub doesn’t include it: Install downloads the developer’s own installer from their GitHub page."
        className="px-5 pt-4"
        actions={st?.installed ? <Badge tone={st.running ? 'good' : 'neutral'}>{st.running ? 'Open' : 'Closed'}</Badge> : undefined}
      />
      <div className="divide-y divide-line">
        <Row title={st?.installed ? `Installed${st.version ? ` · ${st.version}` : ''}` : 'Not installed'} hint={st?.installed ? 'Open it, or close it for a game.' : 'About 100 MB. Free for non-commercial use (its own licence).'}>
          <div className="flex items-center gap-2">
            {st?.installed ? (
              st.running ? (
                <Button size="sm" variant="secondary" icon={Square} loading={busy === 'close'} onClick={() => void run('close', api.capriDesktop.close)}>
                  Close
                </Button>
              ) : (
                <Button size="sm" variant="primary" icon={Play} loading={busy === 'open'} onClick={() => void run('open', api.capriDesktop.open)}>
                  Open
                </Button>
              )
            ) : (
              <Button size="sm" variant="primary" icon={Download} loading={busy === 'install'} disabled={st?.supported === false} onClick={() => void run('install', api.capriDesktop.install, 'Capri’s installer is open: follow its steps')}>
                Install
              </Button>
            )}
            <Button size="sm" variant="ghost" icon={ExternalLink} onClick={() => void api.app.openUrl('https://capriai.app/')}>
              Website
            </Button>
          </div>
        </Row>
        {dl != null && (
          <div className="px-5 py-3">
            <ProgressBar value={dl} label="Downloading Capri" />
          </div>
        )}
        {st?.installed && (
          <Row title="Start with OmniHub" hint="Opens Capri when OmniHub starts. Your phone and deck can open and close it too.">
            <Switch label="Start Capri with OmniHub" checked={st.settings.startWithOmnihub} onChange={(v) => void run('settings', () => api.capriDesktop.setSettings({ startWithOmnihub: v }))} />
          </Row>
        )}
        <Row
          title="Local brain (Ollama)"
          hint={
            !o?.installed && !o?.running
              ? 'Optional: Capri can think on this PC with Ollama, free and offline. Install Ollama, then come back.'
              : !o?.running
                ? 'Ollama is installed but not running. Open it from the Start menu.'
                : o.hasModel
                  ? `Ready: ${MODEL} is downloaded. Pick Ollama in Capri’s settings.`
                  : `Ollama is running. Capri’s site suggests ${MODEL} (a few GB).`
          }
        >
          {!o?.installed && !o?.running ? (
            <Button size="sm" variant="secondary" icon={ExternalLink} onClick={() => void api.app.openUrl('https://ollama.com/download')}>
              Get Ollama
            </Button>
          ) : o?.running && !o.hasModel ? (
            <Button size="sm" variant="secondary" icon={Download} loading={busy === 'pull'} onClick={() => void run('pull', api.capriDesktop.pullModel, `${MODEL} is ready`)}>
              Download {MODEL}
            </Button>
          ) : o?.hasModel ? (
            <Badge tone="good">Ready</Badge>
          ) : null}
        </Row>
        {pull != null && (
          <div className="px-5 py-3">
            <ProgressBar value={pull} label={`Downloading ${MODEL}`} />
          </div>
        )}
      </div>
    </Card>
  );
}
