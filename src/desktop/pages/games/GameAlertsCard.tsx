// Game alerts and the ready check: games free to keep (Epic, and Steam
// giveaways), patch notes for your Steam games, and what could get in the
// way before you play.

import type { ReadyItem, WatchStore } from '@shared/types';
import { ArrowRight, CheckCircle2, Gift, ShieldCheck, TriangleAlert } from 'lucide-react';
import { useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Card, CardHeader } from '../../components/ui/Card';
import { Switch } from '../../components/ui/Form';
import { navigate } from '../../lib/router';
import { toast } from '../../state/toasts';

export function GameAlertsCard({ w, setW }: { w: WatchStore; setW: (w: WatchStore) => void }) {
  const set = (free: boolean, patches: boolean, fortnite = w.fortniteAlerts) => void api.watch.setGameAlerts(free, patches, fortnite).then(setW, (e: unknown) => toast.error('Couldn’t save', errorText(e)));
  return (
    <Card className="p-5">
      <CardHeader icon={Gift} title="Free games and patch notes" subtitle="Checked every 6 hours (Fortnite every 10 minutes); alerts here and on your phone." />
      <div className="mt-4 space-y-3">
        <div className="flex items-center justify-between gap-3 text-[13.5px] text-fg">
          <span>
            Games free to keep <span className="text-faint">(Epic Games, and Steam giveaways listed by GamerPower)</span>
          </span>
          <Switch checked={w.freeAlerts} onChange={(v) => set(v, w.patchAlerts)} label="Free game alerts" />
        </div>
        <div className="flex items-center justify-between gap-3 text-[13.5px] text-fg">
          <span>
            Patch notes for your Steam games <span className="text-faint">(from Steam’s news)</span>
          </span>
          <Switch checked={w.patchAlerts} onChange={(v) => set(w.freeAlerts, v)} label="Patch note alerts" />
        </div>
        <div className="flex items-center justify-between gap-3 text-[13.5px] text-fg">
          <span>
            Fortnite updates and server downtime <span className="text-faint">(your installed version, and Epic’s status page)</span>
          </span>
          <Switch checked={w.fortniteAlerts} onChange={(v) => set(w.freeAlerts, w.patchAlerts, v)} label="Fortnite alerts" />
        </div>
      </div>
    </Card>
  );
}

export function ReadyCard() {
  const [items, setItems] = useState<ReadyItem[] | null>(null);
  const [busy, setBusy] = useState(false);
  const check = async () => {
    setBusy(true);
    try {
      setItems(await api.pcCheck.ready());
    } catch (e) {
      toast.error('Couldn’t check', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  const warnings = items?.filter((i) => i.status !== 'ok').length ?? 0;
  return (
    <Card className="p-5">
      <CardHeader icon={ShieldCheck} title="Ready to play?" subtitle="What could get in the way: a restart Windows is waiting for, low space, busy background programs, memory, heat." actions={<Button size="sm" variant="primary" loading={busy} onClick={() => void check()}>{items ? 'Check again' : 'Check'}</Button>} />
      {items && (
        <div className="mt-3 space-y-1.5">
          <div className="text-[13px] font-medium text-fg">{warnings === 0 ? 'All good. Have fun!' : `${warnings} thing${warnings === 1 ? '' : 's'} to look at`}</div>
          {items.map((i) => (
            <div key={i.title} className="flex items-center gap-2.5 rounded-lg px-1 py-1 text-[13px]">
              {i.status === 'ok' ? <CheckCircle2 size={15} className="shrink-0 text-good" /> : <TriangleAlert size={15} className="shrink-0 text-warn" />}
              <span className="w-[150px] shrink-0 text-dim">{i.title}</span>
              <span className="min-w-0 flex-1 text-fg">{i.detail}</span>
              {i.route && (
                <Button size="xs" variant="ghost" iconRight={ArrowRight} onClick={() => navigate(i.route as string)}>
                  Open
                </Button>
              )}
            </div>
          ))}
        </div>
      )}
    </Card>
  );
}
