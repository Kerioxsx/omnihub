// Game deals across stores, from CheapShark's public price list (US dollars).

import type { DealGame, GameDeals } from '@shared/types';
import { ExternalLink, Tag } from 'lucide-react';
import { useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Card, CardHeader } from '../../components/ui/Card';
import { SearchInput } from '../../components/ui/Form';
import { toast } from '../../state/toasts';

const usd = (n: number) => `$${n.toFixed(2)}`;

function Thumb({ src }: { src: string }) {
  const [failed, setFailed] = useState(false);
  if (!src || failed) return <span className="h-8 w-16 shrink-0 rounded bg-surface-3" />;
  return <img src={src} alt="" className="h-8 w-16 shrink-0 rounded object-cover" onError={() => setFailed(true)} />;
}

export function DealsCard() {
  const [q, setQ] = useState('');
  const [hits, setHits] = useState<DealGame[] | null>(null);
  const [open, setOpen] = useState<GameDeals | null>(null);
  const [busy, setBusy] = useState(false);
  const search = async () => {
    if (!q.trim()) return;
    setBusy(true);
    setOpen(null);
    try {
      setHits(await api.deals.search(q));
    } catch (e) {
      toast.error('Couldn’t search deals', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  const show = async (g: DealGame) => {
    try {
      setOpen(await api.deals.game(g.id));
    } catch (e) {
      toast.error('Couldn’t load the deals', errorText(e));
    }
  };
  return (
    <Card className="p-5">
      <CardHeader icon={Tag} title="Game deals" subtitle="The price of a game in every store CheapShark follows (Steam, GOG, Humble, Fanatical, Green Man Gaming and more), cheapest first. US dollars." />
      <form
        className="mt-4 flex gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          void search();
        }}
      >
        <SearchInput value={q} onChange={setQ} placeholder="A game’s name" className="flex-1" aria-label="Game to look up" />
        <Button type="submit" variant="primary" loading={busy}>
          Find deals
        </Button>
      </form>
      {hits && !open && (
        <div className="mt-3 divide-y divide-line rounded-xl border border-line">
          {hits.length === 0 && <div className="px-3 py-3 text-[12.5px] text-faint">No game by that name.</div>}
          {hits.map((g) => (
            <button key={g.id} type="button" onClick={() => void show(g)} className="flex w-full items-center gap-3 px-3 py-2 text-left text-[13px] hover:bg-surface-2">
              <Thumb src={g.thumb} />
              <span className="min-w-0 flex-1 truncate text-fg">{g.title}</span>
              <span className="tabular text-dim">from {usd(g.cheapest)}</span>
            </button>
          ))}
        </div>
      )}
      {open && (
        <div className="mt-3">
          <div className="mb-2 flex items-center justify-between gap-2">
            <span className="text-[14px] font-semibold text-fg">{open.title}</span>
            <Button size="sm" variant="ghost" onClick={() => setOpen(null)}>
              Back
            </Button>
          </div>
          {open.lowestEver && (
            <div className="mb-2 text-[12px] text-faint">
              Lowest ever: {usd(open.lowestEver[0])} ({new Date(open.lowestEver[1] * 1000).toLocaleDateString()})
            </div>
          )}
          <div className="divide-y divide-line rounded-xl border border-line">
            {open.deals.map((d) => (
              <div key={d.url} className="flex items-center gap-3 px-3 py-2 text-[13px]">
                <span className="w-36 truncate text-fg">{d.store}</span>
                <span className="font-semibold tabular text-fg">{usd(d.price)}</span>
                {d.savings > 0 && <span className="rounded-md bg-good/15 px-1.5 text-[11px] font-semibold text-good">−{Math.round(d.savings)}%</span>}
                {d.savings > 0 && <span className="text-[12px] text-faint line-through">{usd(d.retail)}</span>}
                <button type="button" onClick={() => void api.app.openUrl(d.url)} className="ml-auto inline-flex items-center gap-1 text-[12.5px] text-accent hover:underline">
                  Open <ExternalLink size={11} />
                </button>
              </div>
            ))}
          </div>
        </div>
      )}
    </Card>
  );
}
