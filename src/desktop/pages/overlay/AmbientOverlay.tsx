// The glow around the screen, in a see-through, click-through window per
// monitor (src-tauri/src/ambient.rs opens and places them). It lights up
// while music plays and fades a few seconds after it stops; with "Lyrics on
// the desktop" the lyrics float in the lower part of the screen.

import type { LyricsStatus, Settings } from '@shared/types';
import { useCallback, useEffect, useState } from 'react';
import { inTauri } from '../../api';
import { AuroraLyrics, type LyricsState } from '../../components/aurora/AuroraLyrics';
import { AuroraStage } from '../../components/aurora/AuroraStage';
import { PaletteBlender } from '../../lib/aurora/palette';
import { useVisuals } from '../../lib/aurora/settings';
import { useEvent } from '../../lib/hooks';
import { useNowPlaying } from '../../lib/nowPlaying';
import { useSettings } from '../../state/settings';

function lyricsState(l: LyricsStatus | null, lines: number): LyricsState {
  if (!l) return 'loading';
  if (l.status === 'ready') return lines ? 'ready' : l.lyrics.instrumental ? 'instrumental' : 'none';
  return l.status === 'searching' ? 'searching' : 'none';
}

function useWindowLabel(): string {
  const [label, setLabel] = useState('ambient');
  useEffect(() => {
    if (!inTauri) return;
    void import('@tauri-apps/api/window').then(({ getCurrentWindow }) => setLabel(getCurrentWindow().label), () => undefined);
  }, []);
  return label;
}

export function AmbientOverlay() {
  const loaded = useSettings((s) => !!s.settings);
  useEffect(() => {
    void useSettings.getState().load();
  }, []);
  useEvent<Settings>('settings:changed', (s) => useSettings.getState().set(s));
  const np = useNowPlaying();
  const visuals = useVisuals();
  const label = useWindowLabel();
  const [blender] = useState(() => new PaletteBlender());
  const playing = !!np.state?.playing;

  // On while music plays; fades a few seconds after it stops.
  const [on, setOn] = useState(false);
  useEffect(() => {
    if (playing) return setOn(true);
    const t = setTimeout(() => setOn(false), 4000);
    return () => clearTimeout(t);
  }, [playing]);

  // The player's timing nudge applies here too.
  const offset = (() => {
    try {
      return Number(localStorage.getItem('omnihub.desktopLyricsOffset')) || 0;
    } catch {
      return 0;
    }
  })();
  const position = np.position;
  const time = useCallback(() => position() + offset, [position, offset]);

  if (!loaded) return null;
  const showLyrics = visuals.desktop.lyrics && visuals.lyrics.visible && !!np.state;
  return (
    <div className="fixed inset-0 overflow-hidden" style={{ opacity: on ? 1 : 0, transition: 'opacity 1.4s ease' }} data-ambient-overlay>
      {on && <AuroraStage overlay visuals={visuals} art={np.art} playing={playing} framed={false} who={label} waveY={0} blender={blender} />}
      {showLyrics && np.state && (
        <AuroraLyrics
          key={np.state.key}
          lines={np.lines}
          state={lyricsState(np.lyrics, np.lines.length)}
          time={time}
          trackKey={np.state.key}
          // Over other apps: smaller, low on the screen, on a dark backing so it reads on anything.
          settings={{ ...visuals.lyrics, place: 'lower', offsetY: visuals.lyrics.offsetY + 10, size: visuals.lyrics.size * 0.5, backing: Math.max(0.8, visuals.lyrics.backing) }}
          reduced={visuals.reducedMotion}
          full={false}
          shown={playing}
        />
      )}
    </div>
  );
}
