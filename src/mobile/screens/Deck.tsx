// The macro deck: big buttons made on the PC (Quick actions → Deck). A tap
// runs the button on the PC; the phone never sees what a button types or opens.

import { quickIcon } from '@shared/quickIcons';
import { Grid3x3 } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { client, type DeckButton } from '../client';
import { errorMessage, vibrate } from '../lib/util';
import { toast } from '../state';
import { Empty } from '../ui/common';
import { SubHeader } from './More';

export function DeckPage({ active, onBack }: { active: boolean; onBack: () => void }) {
  const [deck, setDeck] = useState<{ columns: number; buttons: DeckButton[] } | null>(null);
  const [hit, setHit] = useState<string | null>(null);
  const load = useCallback(() => {
    client.deck().then(setDeck, (e) => toast.error(errorMessage(e)));
  }, []);
  useEffect(() => {
    if (active) load();
  }, [active, load]);
  const run = async (b: DeckButton) => {
    vibrate(20);
    setHit(b.id);
    setTimeout(() => setHit((h) => (h === b.id ? null : h)), 350);
    try {
      await client.runDeck(b.id);
    } catch (e) {
      vibrate([40, 60, 40]);
      toast.error(errorMessage(e));
    }
  };
  const cols = Math.min(deck?.columns ?? 3, 4);
  return (
    <div className="flex min-h-full flex-col">
      <SubHeader title="Deck" onBack={onBack} />
      {deck && deck.buttons.length === 0 && <Empty icon={<Grid3x3 size={28} />} title="No buttons yet" body="Make some on the PC: Quick actions → Deck." />}
      {deck && deck.buttons.length > 0 && (
        <div className="grid gap-3 px-4 pb-6" style={{ gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` }}>
          {deck.buttons.map((b) => {
            const Icon = quickIcon(b.icon);
            const color = b.color || '#a78bfa';
            return (
              <button
                key={b.id}
                type="button"
                onClick={() => void run(b)}
                className={`flex aspect-square select-none flex-col items-center justify-center gap-2 rounded-2xl border border-white/10 bg-white/[0.04] p-2 text-center transition-transform active:scale-90 ${hit === b.id ? 'scale-95' : ''}`}
                style={{ boxShadow: hit === b.id ? `0 0 0 2px ${color}, 0 0 24px ${color}66` : `inset 0 -3px 0 ${color}55` }}
              >
                <span className="grid h-12 w-12 place-items-center rounded-xl" style={{ background: `${color}26`, color }}>
                  <Icon size={24} />
                </span>
                <span className="line-clamp-2 text-[12.5px] font-medium leading-tight text-fg">{b.label}</span>
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}
