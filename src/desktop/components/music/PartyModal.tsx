// Party queue: friends scan the QR code with their phone camera and add
// songs, no pairing. Songs play one after another in your music app (the
// next starts when one ends, with "Play the next by itself" on).

import type { PartyView } from '@shared/types';
import { ArrowUp, Copy, PartyPopper, Play, Square, X } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../ui/Button';
import { Switch } from '../ui/Form';
import { Modal } from '../ui/Overlay';
import { EmptyState } from '../ui/States';
import { useEvent } from '../../lib/hooks';
import { toast } from '../../state/toasts';

export function PartyModal({ open, onClose }: { open: boolean; onClose: () => void }) {
  const [v, setV] = useState<PartyView | null>(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    if (open) void api.party.get().then(setV, () => undefined);
  }, [open]);
  useEvent('party:changed', () => void api.party.get().then(setV, () => undefined));
  const run = async (f: () => Promise<PartyView>) => {
    setBusy(true);
    try {
      setV(await f());
    } catch (e) {
      toast.error('Couldn’t do that', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  const st = v?.state;
  const on = !!st?.code;
  return (
    <Modal open={open} onClose={onClose} icon={PartyPopper} title="Party queue" description="Friends scan the code with their phone camera and add songs. No app or pairing; they need to be on your Wi-Fi." size="lg">
      {!on ? (
        <div className="space-y-4">
          <p className="text-[13px] text-dim">Songs play one after another in your music app, the way the music hub plays them (YouTube Music opens the song itself; Spotify and others open on its search). Each friend can have 15 songs waiting.</p>
          <Button variant="primary" icon={PartyPopper} loading={busy} onClick={() => void run(api.party.start)}>
            Start a party
          </Button>
        </div>
      ) : (
        <div className="grid gap-5 sm:grid-cols-[200px_minmax(0,1fr)]">
          <div className="space-y-2">
            {v?.qrSvgs[0] && <div className="aspect-square w-full rounded-xl bg-white p-2 [&_svg]:h-full [&_svg]:w-full" role="img" aria-label="Party QR code" dangerouslySetInnerHTML={{ __html: v.qrSvgs[0] }} />}
            {v?.urls[0] && (
              <button type="button" className="flex w-full items-center gap-1.5 truncate rounded-lg bg-surface-2 px-2 py-1.5 text-left text-[11.5px] text-dim hover:text-fg" onClick={() => void navigator.clipboard.writeText(v.urls[0]).then(() => toast.success('Link copied'))}>
                <Copy size={12} className="shrink-0" />
                <span className="truncate">{v.urls[0]}</span>
              </button>
            )}
            <p className="text-[11.5px] text-faint">The phone may warn about the page’s certificate (it’s this PC’s own); that’s expected.</p>
          </div>
          <div className="min-w-0 space-y-3">
            <div className="flex flex-wrap items-center gap-2">
              <Button size="sm" variant="primary" icon={Play} loading={busy} disabled={!st?.queue.length} onClick={() => void run(api.party.next)}>
                {st?.playing ? 'Next song' : 'Play the first'}
              </Button>
              <label className="flex items-center gap-2 text-[12.5px] text-dim">
                <Switch label="Play the next by itself" checked={!!st?.auto} onChange={(a) => void run(() => api.party.auto(a))} />
                Play the next by itself
              </label>
              <Button size="sm" variant="ghost" icon={Square} className="ml-auto" onClick={() => void run(api.party.stop)}>
                End party
              </Button>
            </div>
            {st?.playing && (
              <div className="rounded-xl border border-accent/40 bg-accent/10 px-3 py-2">
                <div className="text-[11px] font-semibold uppercase tracking-wider text-accent">Playing</div>
                <div className="truncate text-[13.5px] font-medium">
                  {st.playing.title} · <span className="text-dim">{st.playing.artist}</span>
                </div>
                <div className="text-[11.5px] text-faint">Added by {st.playing.by}</div>
              </div>
            )}
            <div className="max-h-[320px] space-y-1 overflow-y-auto pr-1">
              {st?.queue.length === 0 ? (
                <EmptyState compact icon={PartyPopper} title="Nothing queued yet" description="Songs your friends add show up here." />
              ) : (
                st?.queue.map((s, i) => (
                  <div key={s.id} className="group flex items-center gap-3 rounded-lg px-2 py-1.5 hover:bg-surface-2">
                    <span className="w-5 text-right text-[12px] tabular-nums text-faint">{i + 1}</span>
                    {s.cover ? <img src={s.cover} alt="" className="h-9 w-9 rounded-md object-cover" /> : <span className="h-9 w-9 rounded-md bg-surface-2" />}
                    <div className="min-w-0 flex-1">
                      <div className="truncate text-[13px] font-medium">{s.title}</div>
                      <div className="truncate text-[11.5px] text-faint">
                        {s.artist} · {s.by}
                      </div>
                    </div>
                    <div className="flex opacity-60 group-hover:opacity-100">
                      {i > 0 && <IconButton icon={ArrowUp} label="Play next" size="sm" onClick={() => void run(() => api.party.bump(s.id))} />}
                      <IconButton icon={X} label="Remove" size="sm" variant="danger" onClick={() => void run(() => api.party.remove(s.id))} />
                    </div>
                  </div>
                ))
              )}
            </div>
            <div className="text-[11.5px] text-faint">{st?.played ? `${st.played} played so far` : ''}</div>
          </div>
        </div>
      )}
    </Modal>
  );
}
