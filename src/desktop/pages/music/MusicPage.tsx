// Music: what the PC plays, with its lyrics, in the full player — and one
// click to make it fill the screen. Karaoke Studio is its own tab, with its
// own state, so the player is the same as ever.

import { Maximize2, MicVocal, Music2, PartyPopper } from 'lucide-react';
import { Suspense, lazy, useState } from 'react';
import { Page } from '../../components/Page';
import { FullPlayer } from '../../components/music/FullPlayer';
import { PartyModal } from '../../components/music/PartyModal';
import { Button } from '../../components/ui/Button';
import { type TabItem, Tabs } from '../../components/ui/Form';
import { navigate, useRoute } from '../../lib/router';
import { usePlayer } from '../../lib/nowPlaying';

const KaraokeStudio = lazy(() => import('./karaoke/KaraokeStudio').then((m) => ({ default: m.KaraokeStudio })));

type Tab = 'player' | 'karaoke';
const TABS: TabItem<Tab>[] = [
  { value: 'player', label: 'Player', icon: Music2 },
  { value: 'karaoke', label: 'Karaoke Studio', icon: MicVocal },
];

export function MusicPage() {
  const show = usePlayer((s) => s.show);
  // One player at a time: the full-screen one replaces this one while open.
  const fullOpen = usePlayer((s) => s.open);
  const [party, setParty] = useState(false);
  const tab: Tab = useRoute().params.get('tab') === 'karaoke' ? 'karaoke' : 'player';
  return (
    <Page
      title="Music"
      subtitle={tab === 'karaoke' ? 'Sing along and learn songs in any language, with pronunciation written for you.' : 'What your PC is playing, with time-synced lyrics. Space plays and pauses, the arrows seek and change the volume.'}
      actions={
        tab === 'player' && (
        <div className="flex gap-2">
          <Button icon={PartyPopper} onClick={() => setParty(true)}>
            Party queue
          </Button>
          <Button icon={Maximize2} onClick={show}>
            Full screen
          </Button>
        </div>
        )
      }
    >
      <Tabs items={TABS} value={tab} onChange={(t) => navigate('music', t === 'karaoke' ? { tab: 'karaoke' } : {})} className="mb-4" />
      {tab === 'karaoke' ? (
        <Suspense fallback={<div className="h-[420px]" />}>
          <KaraokeStudio />
        </Suspense>
      ) : (
        <div className="h-[calc(100vh-240px)] min-h-[460px] overflow-hidden rounded-2xl border border-line shadow-lg">
          {fullOpen ? <div className="h-full w-full bg-[#07070b]" /> : <FullPlayer mode="page" />}
        </div>
      )}
      <PartyModal open={party} onClose={() => setParty(false)} />
    </Page>
  );
}
