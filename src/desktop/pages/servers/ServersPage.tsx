// Servers: Minecraft servers hosted on this PC, around the clock. The list,
// making one (fresh, from a modpack or a mix), adding an existing server
// folder, and one server's page.

import type { SrvFrom, SrvView } from '@shared/types';
import { FolderOpen, Plus, Server } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Page } from '../../components/Page';
import { Button } from '../../components/ui/Button';
import { Badge, Card } from '../../components/ui/Card';
import { EmptyState, ErrorState } from '../../components/ui/States';
import { useEvent } from '../../lib/hooks';
import { navigate, useRoute } from '../../lib/router';
import { toast } from '../../state/toasts';
import { NewServerModal } from './NewServer';
import { ServerView } from './ServerView';
import { ago, flavorLabel, ServerIcon, StateBadge } from './shared';

export function useServers() {
  const [list, setList] = useState<SrvView[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const load = useCallback(() => void api.servers.list().then((l) => (setList(l), setError(null)), (e: unknown) => setError(errorText(e))), []);
  useEffect(load, [load]);
  useEvent('mcserver:changed', load);
  useEvent('mcserver:work', load);
  return { list, error, reload: load };
}

function ServerCard({ s }: { s: SrvView }) {
  return (
    <Card interactive className="flex items-center gap-4 px-5 py-4" onClick={() => navigate('servers', { id: s.id })}>
      <ServerIcon s={s} size={52} />
      <div className="min-w-0 flex-1 space-y-1">
        <div className="flex flex-wrap items-center gap-2">
          <span className="truncate text-[14.5px] font-semibold text-fg">{s.name}</span>
          <StateBadge s={s} />
        </div>
        <div className="flex flex-wrap items-center gap-1.5 text-[12px] text-dim">
          <Badge tone="neutral">{flavorLabel(s.flavor)}</Badge>
          <Badge tone="neutral">Minecraft {s.mc}</Badge>
          {s.from && <Badge tone="accent">{s.from}</Badge>}
          {s.autoStart && <Badge tone="info">24/7</Badge>}
          <span className="truncate">{s.state === 'running' ? `up ${ago(s.since)}` : s.work ? s.work.text : s.lan ?? ''}</span>
        </div>
      </div>
      {s.online.length > 0 && (
        <div className="hidden text-right text-[12px] text-dim sm:block">
          {s.online
            .slice(0, 3)
            .map((o) => o.name)
            .join(', ')}
          {s.online.length > 3 && ` +${s.online.length - 3}`}
        </div>
      )}
    </Card>
  );
}

export function ServersPage() {
  const route = useRoute();
  const id = route.params.get('id');
  const fromParam = route.params.get('from');
  const { list, error, reload } = useServers();
  const [making, setMaking] = useState(false);
  const [preset, setPreset] = useState<SrvFrom | null>(null);

  // "Host as a server" from the modpack mixer.
  useEffect(() => {
    if (fromParam?.startsWith('mix:')) {
      setPreset({ type: 'mix', id: fromParam.slice(4) });
      setMaking(true);
      navigate('servers');
    }
  }, [fromParam]);

  const adopt = async () => {
    const dir = await api.app.pickFolder('Pick a Minecraft server folder (with server.properties)');
    if (!dir) return;
    try {
      const v = await api.servers.adopt(dir);
      toast.success(`Added ${v.name}`, `${flavorLabel(v.flavor)} ${v.mc}`);
      reload();
      navigate('servers', { id: v.id });
    } catch (e) {
      toast.error('Not added', errorText(e));
    }
  };

  const current = id ? list?.find((s) => s.id === id) : null;
  if (id && current) return <ServerView s={current} reload={reload} />;

  return (
    <Page
      title="Servers"
      subtitle="Host Minecraft servers on this PC, up around the clock: players and their inventories, a live map, mods, plugins, modpacks and your mixes, and backups."
      actions={
        <>
          <Button variant="secondary" icon={FolderOpen} onClick={() => void adopt()}>
            Add a server folder
          </Button>
          <Button variant="primary" icon={Plus} onClick={() => (setPreset(null), setMaking(true))}>
            New server
          </Button>
        </>
      }
    >
      <NewServerModal open={making} onClose={() => setMaking(false)} preset={preset} onMade={(nid) => (reload(), navigate('servers', { id: nid }))} />
      {error && <ErrorState error={error} onRetry={reload} />}
      {list && list.length === 0 && (
        <Card>
          <EmptyState
            icon={Server}
            title="No servers yet"
            description="Make one for you and your friends: Paper for plugins, Fabric or NeoForge for mods, or straight from a modpack or one of your mixes. OmniHub gets everything it needs and keeps it running."
            action={
              <Button variant="primary" icon={Plus} onClick={() => setMaking(true)}>
                New server
              </Button>
            }
          />
        </Card>
      )}
      <div className="space-y-3">
        {list?.map((s) => (
          <ServerCard key={s.id} s={s} />
        ))}
      </div>
    </Page>
  );
}
