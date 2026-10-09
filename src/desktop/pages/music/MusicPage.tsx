// Music: what the PC plays, with its lyrics, in the full player — and one
// click to make it fill the screen.

import { Maximize2 } from 'lucide-react';
import { Page } from '../../components/Page';
import { FullPlayer } from '../../components/music/FullPlayer';
import { Button } from '../../components/ui/Button';
import { usePlayer } from '../../lib/nowPlaying';

export function MusicPage() {
  const show = usePlayer((s) => s.show);
  return (
    <Page
      title="Music"
      subtitle="What your PC is playing, with time-synced lyrics. Space plays and pauses, the arrows seek and change the volume."
      actions={
        <Button icon={Maximize2} onClick={show}>
          Full screen
        </Button>
      }
    >
      <div className="h-[calc(100vh-190px)] min-h-[460px] overflow-hidden rounded-2xl border border-line shadow-lg">
        <FullPlayer mode="page" />
      </div>
    </Page>
  );
}
