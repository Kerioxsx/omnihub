// Pieces the Servers page shares: the state badge and the server's picture,
// plus what @shared/minecraft has (the phone uses that too).

import { STATE } from '@shared/minecraft';
import type { SrvView } from '@shared/types';
import { Server, ShieldCheck, Users } from 'lucide-react';
import { Badge } from '../../components/ui/Card';
import { cx } from '../../lib/cx';

export { DIMENSIONS, FLAVORS, Head, ItemTile, STATE, ago, agoText, busy, dimLabel, flavorLabel, up } from '@shared/minecraft';

export function StateBadge({ s }: { s: SrvView }) {
  const st = STATE[s.state];
  return <Badge tone={st.tone}>{s.state === 'running' ? `Online · ${s.online.length}/${s.maxPlayers}` : st.label}</Badge>;
}

export function ServerIcon({ s, size = 48, className }: { s: Pick<SrvView, 'icon' | 'name' | 'state'>; size?: number; className?: string }) {
  return (
    <div className={cx('relative grid shrink-0 place-items-center overflow-hidden rounded-xl bg-surface-2 text-dim', className)} style={{ width: size, height: size }}>
      {s.icon ? <img src={s.icon} alt="" className="h-full w-full object-cover [image-rendering:pixelated]" /> : <Server size={size * 0.45} />}
      <span className={cx('absolute bottom-1 right-1 h-2.5 w-2.5 rounded-full ring-2 ring-surface', s.state === 'running' ? 'bg-good' : s.state === 'crashed' ? 'bg-bad' : s.state === 'stopped' ? 'bg-faint' : 'bg-warn')} />
    </div>
  );
}

/** Who can join: paid Minecraft accounts only, or everyone (no account check, operators guarded). */
export function JoinChoice({ everyone, onChange }: { everyone: boolean; onChange: (everyone: boolean) => void }) {
  const opts = [
    { on: false, icon: ShieldCheck, title: 'Paid Minecraft accounts', hint: 'Mojang checks every player. The safest choice.' },
    { on: true, icon: Users, title: 'Everyone, also without a paid account', hint: 'Friends who play with a free launcher can join too.' },
  ];
  return (
    <div className="grid gap-2 sm:grid-cols-2" role="radiogroup" aria-label="Who can join">
      {opts.map((o) => (
        <button key={String(o.on)} type="button" role="radio" aria-checked={everyone === o.on} onClick={() => onChange(o.on)} className={cx('flex items-start gap-2.5 rounded-xl border px-3 py-2.5 text-left transition', everyone === o.on ? 'border-accent bg-accent-soft' : 'border-line hover:bg-surface-2')}>
          <o.icon size={16} className={cx('mt-0.5 shrink-0', everyone === o.on ? 'text-accent' : 'text-faint')} />
          <span>
            <span className="block text-[13px] font-semibold text-fg">{o.title}</span>
            <span className="block text-[11.5px] leading-snug text-dim">{o.hint}</span>
          </span>
        </button>
      ))}
    </div>
  );
}

/** What “everyone” means, in plain words. */
export const EVERYONE_NOTE = 'Minecraft can’t check names on these servers, so anyone could type yours. OmniHub guards your operators: an operator’s name joining from a new place is kept out until you allow it (here or on your phone). Keep the whitelist on too, so only your friends’ names get in.';
