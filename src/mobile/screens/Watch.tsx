// Watch on the PC: Netflix, YouTube, Crunchyroll, Disney+ and the rest
// open full screen on the PC (in their own window, no browser bars).

import { Tv } from 'lucide-react';
import { useEffect, useState } from 'react';
import { client, type Site } from '../client';
import { errorMessage, vibrate } from '../lib/util';
import { toast } from '../state';
import { Empty, Switch } from '../ui/common';
import { SubHeader } from './More';

export function WatchPage({ active, onBack }: { active: boolean; onBack: () => void }) {
  const [sites, setSites] = useState<Site[] | null>(null);
  const [full, setFull] = useState(true);
  useEffect(() => {
    if (active) client.sites().then((r) => setSites(r.sites), () => setSites([]));
  }, [active]);
  const open = async (s: Site) => {
    vibrate(20);
    try {
      await client.openSite(s.id, full);
      toast.success(`${s.name} is opening on the PC`);
    } catch (e) {
      toast.error(errorMessage(e));
    }
  };
  return (
    <div className="scroller h-full">
      <SubHeader title="Watch on the PC" onBack={onBack} />
      <div className="px-safe pb-tabbar space-y-4">
        <div className="card flex items-center gap-3 px-4 py-3">
          <span className="flex-1 text-[15px]">Full screen, no browser bars</span>
          <Switch label="Full screen" on={full} onChange={setFull} />
        </div>
        {sites && sites.length === 0 ? (
          <Empty icon={<Tv size={28} />} title="No sites" body="Add them on the PC in Big Picture." />
        ) : (
          <div className="grid grid-cols-2 gap-3">
            {sites?.map((s) => (
              <button key={s.id} type="button" onClick={() => void open(s)} className="press relative h-24 overflow-hidden rounded-2xl p-3 text-left" style={{ background: `radial-gradient(120% 90% at 15% 0%, ${s.color}, #0b0b10 80%)` }}>
                <span className="absolute bottom-3 left-3 font-display text-[19px] font-extrabold text-white drop-shadow">{s.name}</span>
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
