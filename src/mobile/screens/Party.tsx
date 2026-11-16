// Party: what the PC's party queue is playing, and its guest page (the same
// one friends open from the QR code) to add songs from this phone.

import { ExternalLink, PartyPopper } from 'lucide-react';
import { useEffect, useState } from 'react';
import { client } from '../client';
import { useEvent } from '../lib/events';
import { Button, Empty } from '../ui/common';
import { SubHeader } from './More';

export function PartyPage({ active, onBack }: { active: boolean; onBack: () => void }) {
  const [p, setP] = useState<Awaited<ReturnType<typeof client.party>> | null>(null);
  const load = () => void client.party().then(setP, () => undefined);
  useEffect(() => {
    if (active) load();
  }, [active]);
  useEvent('party:changed', load);
  return (
    <div className="scroller h-full">
      <SubHeader title="Party queue" onBack={onBack} />
      <div className="px-safe pb-tabbar space-y-4">
        {!p?.code ? (
          <Empty icon={<PartyPopper size={30} />} title="No party right now" body="Start one on the PC: Music → Party queue. Friends then scan its QR code to add songs." />
        ) : (
          <>
            <div className="card p-5">
              <div className="text-[12px] uppercase tracking-[0.15em] text-faint">Playing</div>
              <div className="mt-1 text-[18px] font-semibold">{p.playing ? p.playing.title : 'Nothing yet'}</div>
              {p.playing && <div className="text-[14px] text-dim">{p.playing.artist}</div>}
              <div className="mt-3 text-[13px] text-faint">{p.queue} song{p.queue === 1 ? '' : 's'} waiting</div>
            </div>
            <Button icon={<ExternalLink size={18} />} onClick={() => window.open(`/party/${p.code}`, '_blank')}>
              Add songs
            </Button>
          </>
        )}
      </div>
    </div>
  );
}
