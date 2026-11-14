import type { ConfigGame, GameKind, GameProfile } from '@shared/types';
import type { ReactNode } from 'react';
import { ShieldCheck } from 'lucide-react';
import { cx } from '../../lib/cx';

export const GAMES: Record<GameKind, { label: string; short: string; from: string; to: string; blurb: string }> = {
  fortnite: { label: 'Fortnite', short: 'FN', from: '#7c3aed', to: '#0ea5e9', blurb: 'Epic Games launcher' },
  roblox: { label: 'Roblox', short: 'RB', from: '#334155', to: '#0f172a', blurb: 'With Fast Flags' },
  valorant: { label: 'VALORANT', short: 'VA', from: '#ff4655', to: '#7f1d1d', blurb: 'Riot Client' },
  cs2: { label: 'Counter-Strike 2', short: 'CS', from: '#f59e0b', to: '#78350f', blurb: 'Steam' },
  apex: { label: 'Apex Legends', short: 'AP', from: '#dc2626', to: '#450a0a', blurb: 'Steam' },
  overwatch: { label: 'Overwatch 2', short: 'OW', from: '#f97316', to: '#1e293b', blurb: 'Battle.net' },
  rocketLeague: { label: 'Rocket League', short: 'RL', from: '#2563eb', to: '#ea580c', blurb: 'Epic Games launcher' },
  gta5: { label: 'GTA V', short: 'V', from: '#16a34a', to: '#14532d', blurb: 'Steam' },
  callOfDuty: { label: 'Call of Duty', short: 'CoD', from: '#57534e', to: '#1c1917', blurb: 'Steam' },
  league: { label: 'League of Legends', short: 'LoL', from: '#c8aa6e', to: '#0a1428', blurb: 'Riot Client' },
  minecraft: { label: 'Minecraft', short: 'MC', from: '#65a30d', to: '#3f2a14', blurb: 'Java edition' },
  cyberpunk: { label: 'Cyberpunk 2077', short: '77', from: '#facc15', to: '#0e7490', blurb: 'Steam · plays in Quality mode' },
  custom: { label: 'Any game', short: '★', from: '#8b5cf6', to: '#22d3ee', blurb: 'Pick its program or link' },
};

/** Games whose own settings OmniHub can set. */
export const CONFIG_GAME: Partial<Record<GameKind, ConfigGame>> = { fortnite: 'fortnite', valorant: 'valorant', cs2: 'cs2', apex: 'apex', overwatch: 'overwatch', roblox: 'roblox', minecraft: 'minecraft' };

export function GameTile({ kind, size = 44, className }: { kind: GameKind; size?: number; className?: string }) {
  const g = GAMES[kind];
  return (
    <div
      className={cx('grid shrink-0 place-items-center rounded-[12px] font-display font-bold text-white shadow-[inset_0_1px_0_rgba(255,255,255,.18)]', className)}
      style={{ width: size, height: size, fontSize: size * (g.short.length > 2 ? 0.28 : 0.36), background: `linear-gradient(135deg, ${g.from}, ${g.to})` }}
      aria-hidden
    >
      {g.short}
    </div>
  );
}

export function launchSummary(p: GameProfile): string {
  const l = p.launch;
  switch (l.type) {
    case 'none':
      return 'Boost only — you start the game';
    case 'exe':
      return l.path.split(/[\\/]/).pop() ?? l.path;
    case 'url':
      return l.url;
    case 'steam':
      return `Steam app ${l.appId}`;
    case 'epic':
      return 'Epic Games launcher';
    case 'riot':
      return 'Riot Client';
    case 'roblox':
      return l.placeId ? `Roblox experience ${l.placeId}` : 'Roblox app';
  }
}

export function Row({ title, hint, children, badge, stack }: { title: ReactNode; hint?: ReactNode; children?: ReactNode; badge?: ReactNode; stack?: boolean }) {
  return (
    <div className={cx('px-5 py-3.5', stack ? 'space-y-2.5' : 'flex items-center justify-between gap-6')}>
      <div className="min-w-0">
        <div className="flex items-center gap-2 text-[13.5px] font-medium text-fg">
          {title}
          {badge}
        </div>
        {hint && <div className="mt-0.5 text-[12.5px] leading-snug text-faint">{hint}</div>}
      </div>
      {children && <div className={cx(stack ? '' : 'shrink-0')}>{children}</div>}
    </div>
  );
}

export function AdminBadge({ on }: { on?: boolean }) {
  return (
    <span className={cx('inline-flex items-center gap-1 rounded-full px-1.5 py-px text-[10.5px] font-semibold', on ? 'bg-good/15 text-good' : 'bg-warn/15 text-warn')} title={on ? 'Already set on this PC' : 'Windows asks for administrator approval once'}>
      <ShieldCheck size={11} /> {on ? 'On' : 'Admin once'}
    </span>
  );
}

export function PerGameBadge() {
  return <span className="rounded-full bg-surface-3 px-1.5 py-px text-[10.5px] font-semibold text-dim" title="Stays set for this game until turned off here">Per game</span>;
}
