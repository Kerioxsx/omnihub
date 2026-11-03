// Music: what the PC plays, with its lyrics, in the full player — and one
// click to make it fill the screen.

import { Maximize2, PartyPopper } from 'lucide-react';
import { useState } from 'react';
import { Page } from '../../components/Page';
import { FullPlayer } from '../../components/music/FullPlayer';
import { PartyModal } from '../../components/music/PartyModal';
import { Button } from '../../components/ui/Button';
import { usePlayer } from '../../lib/nowPlaying';

export function MusicPage() {
  const show = usePlayer((s) => s.show);
  // One player at a time: the full-screen one replaces this one while open.
  const fullOpen = usePlayer((s) => s.open);
  const [party, setParty] = useState(false);
  return (
    <Page
      title="Music"
      subtitle="What your PC is playing, with time-synced lyrics. Space plays and pauses, the arrows seek and change the volume."
      actions={
        <div className="flex gap-2">
          <Button icon={PartyPopper} onClick={() => setParty(true)}>
            Party queue
          </Button>
          <Button icon={Maximize2} onClick={show}>
            Full screen
          </Button>
        </div>
      }
    >
      <div className="h-[calc(100vh-190px)] min-h-[460px] overflow-hidden rounded-2xl border border-line shadow-lg">
        {fullOpen ? <div className="h-full w-full bg-[#07070b]" /> : <FullPlayer mode="page" />}
      </div>
      <PartyModal open={party} onClose={() => setParty(false)} />
    </Page>
  );
}
