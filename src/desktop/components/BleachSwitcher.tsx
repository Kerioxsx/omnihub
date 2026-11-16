// While a Bleach theme is on: switch between Bankai, Hollow, Mugetsu and
// Hakka no Togame from the sidebar. Each switch has its own short
// transformation (an orange slash, an ink flash, a dark fade, a freeze) and
// a soft sound; none with reduced motion. "Picture" puts your own picture
// behind the current mode.

import type { Look } from '@shared/types';
import { ImagePlus, Trash2 } from 'lucide-react';
import { useEffect, useState } from 'react';
import { createPortal } from 'react-dom';
import { api, errorText } from '../api';
import { cx } from '../lib/cx';
import { BLEACH } from '../lib/looks';
import { useSettings } from '../state/settings';
import { toast } from '../state/toasts';
import { type ArtPrefs, artPrefs, setArtPrefs } from './Backdrop';
import { Button } from './ui/Button';
import { Segmented, Switch } from './ui/Form';
import { Modal } from './ui/Overlay';

function reducedMotion(setting: boolean | undefined): boolean {
  return !!setting || window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

const SOUND_KEY = 'omnihub.themeSounds';
const soundsOn = () => {
  try {
    return localStorage.getItem(SOUND_KEY) !== 'off';
  } catch {
    return true;
  }
};

let ctx: AudioContext | null = null;
/** A short synthesized sound for each mode (quiet). */
function playSwitch(mode: Look) {
  if (!soundsOn()) return;
  try {
    ctx ??= new AudioContext();
    const c = ctx;
    const t = c.currentTime;
    const out = c.createGain();
    out.gain.value = 0.05;
    out.connect(c.destination);
    const tone = (type: OscillatorType, f1: number, f2: number, len: number, at = 0, vol = 1) => {
      const o = c.createOscillator();
      const g = c.createGain();
      o.type = type;
      o.frequency.setValueAtTime(f1, t + at);
      o.frequency.exponentialRampToValueAtTime(f2, t + at + len);
      g.gain.setValueAtTime(vol, t + at);
      g.gain.exponentialRampToValueAtTime(0.0001, t + at + len);
      o.connect(g).connect(out);
      o.start(t + at);
      o.stop(t + at + len + 0.02);
    };
    const noise = (len: number, f1: number, f2: number, vol = 1) => {
      const buf = c.createBuffer(1, Math.floor(c.sampleRate * len), c.sampleRate);
      const d = buf.getChannelData(0);
      for (let i = 0; i < d.length; i++) d[i] = Math.random() * 2 - 1;
      const src = c.createBufferSource();
      src.buffer = buf;
      const bp = c.createBiquadFilter();
      bp.type = 'bandpass';
      bp.Q.value = 1.2;
      bp.frequency.setValueAtTime(f1, t);
      bp.frequency.exponentialRampToValueAtTime(f2, t + len);
      const g = c.createGain();
      g.gain.setValueAtTime(vol, t);
      g.gain.exponentialRampToValueAtTime(0.0001, t + len);
      src.connect(bp).connect(g).connect(out);
      src.start(t);
    };
    if (mode === 'bankai') noise(0.45, 600, 4200, 1.6); // a sword whoosh
    else if (mode === 'hollow') [0, 0.06, 0.12].forEach((at, i) => tone('square', 180 + i * 90, 90, 0.07, at, 0.6)); // a sharp glitch
    else if (mode === 'mugetsu') tone('sine', 90, 38, 0.9, 0, 1.4); // a deep muted pulse
    else {
      tone('sine', 1760, 1700, 0.9, 0, 0.7); // an ice chime
      tone('sine', 2637, 2600, 0.7, 0.05, 0.4);
    }
  } catch {
    /* no sound */
  }
}

function ArtDialog({ look, open, onClose }: { look: Look; open: boolean; onClose: () => void }) {
  const mode = BLEACH.find((m) => m.id === look);
  const [url, setUrl] = useState<string | null>(null);
  const [p, setP] = useState<ArtPrefs>(() => artPrefs(look));
  const [sounds, setSounds] = useState(soundsOn);
  useEffect(() => {
    if (!open) return;
    setP(artPrefs(look));
    void api.themeArt.get(look).then(setUrl, () => setUrl(null));
  }, [open, look]);
  const patch = (x: Partial<ArtPrefs>) => {
    setArtPrefs(look, x);
    setP(artPrefs(look));
  };
  const choose = async () => {
    try {
      const [path] = await api.app.pickFiles('Pick a picture for this theme');
      if (!path) return;
      setUrl(await api.themeArt.set(look, path));
      window.dispatchEvent(new Event('omnihub:theme-art'));
    } catch (e) {
      toast.error('Couldn’t use that picture', errorText(e));
    }
  };
  const remove = async () => {
    await api.themeArt.set(look, null).catch(() => null);
    setUrl(null);
    window.dispatchEvent(new Event('omnihub:theme-art'));
  };
  return (
    <Modal open={open} onClose={onClose} icon={ImagePlus} title={`Picture behind ${mode?.name ?? 'the theme'}`} description="Your own picture, shown large and faded behind the cards. It stays on this PC; OmniHub doesn’t include artwork from the series." size="md">
      <div className="space-y-4">
        <div className="flex items-center gap-4">
          <div className="grid h-28 w-24 shrink-0 place-items-center overflow-hidden rounded-lg border border-line bg-surface-2">{url ? <img src={url} alt="" className="h-full w-full object-contain" /> : <ImagePlus size={22} className="text-faint" />}</div>
          <div className="space-y-2">
            <Button variant="primary" icon={ImagePlus} onClick={() => void choose()}>
              {url ? 'Choose another' : 'Choose a picture'}
            </Button>
            {url && (
              <Button variant="ghost" icon={Trash2} onClick={() => void remove()}>
                Remove
              </Button>
            )}
            <p className="text-[12px] text-faint">PNG, JPG, WebP or GIF, up to 25 MB. Black-on-white manga pages work well.</p>
          </div>
        </div>
        <div className="space-y-1.5">
          <div className="text-[12.5px] font-medium">Style</div>
          <Segmented
            label="Style"
            value={p.style}
            onChange={(style) => patch({ style })}
            options={[
              { value: 'tint', label: 'Theme colour' },
              { value: 'ink', label: 'Dark ink' },
              { value: 'color', label: 'Original' },
            ]}
          />
          <p className="text-[12px] text-faint">Theme colour and Dark ink drop a white background and keep the drawing, in the mode’s colour (Bankai’s black turns orange) or as dark ink.</p>
        </div>
        <div className="space-y-1.5">
          <div className="flex justify-between text-[12.5px]">
            <span className="font-medium">Strength</span>
            <span className="tabular-nums text-dim">{Math.round(p.opacity * 100)}%</span>
          </div>
          <input type="range" min={0.1} max={0.65} step={0.01} value={p.opacity} onChange={(e) => patch({ opacity: Number(e.target.value) })} className="w-full accent-[var(--accent)]" aria-label="Strength" />
        </div>
        <div className="space-y-1.5">
          <div className="text-[12.5px] font-medium">Place</div>
          <Segmented
            label="Place"
            value={p.side}
            onChange={(side) => patch({ side })}
            options={[
              { value: 'left', label: 'Left' },
              { value: 'center', label: 'Centre' },
              { value: 'right', label: 'Right' },
            ]}
          />
        </div>
        <label className="flex items-center justify-between gap-3 text-[13px]">
          <span>Sounds when switching modes</span>
          <Switch
            label="Sounds when switching modes"
            checked={sounds}
            onChange={(v) => {
              setSounds(v);
              try {
                localStorage.setItem(SOUND_KEY, v ? 'on' : 'off');
              } catch {
                /* this session only */
              }
            }}
          />
        </label>
      </div>
    </Modal>
  );
}

export function BleachSwitcher({ collapsed }: { collapsed?: boolean }) {
  const look = useSettings((s) => s.settings?.general.look) as Look | undefined;
  const reduced = useSettings((s) => s.settings?.general.reducedMotion);
  const update = useSettings((s) => s.update);
  const [flash, setFlash] = useState<{ mode: Look; key: number } | null>(null);
  const [art, setArt] = useState(false);
  const at = BLEACH.findIndex((m) => m.id === look);
  if (at < 0 || !look) return null;
  const go = (id: Look) => {
    if (id === look) return;
    const still = reducedMotion(reduced);
    if (!still) {
      const key = Date.now();
      setFlash({ mode: id, key });
      setTimeout(() => setFlash((f) => (f?.key === key ? null : f)), 900);
    }
    playSwitch(id);
    // The colours change under the transformation.
    setTimeout(() => void update({ general: { look: id } }, { silent: true }), still ? 0 : id === 'mugetsu' || id === 'hakka' ? 300 : 160);
  };
  const next = BLEACH[(at + 1) % BLEACH.length];
  return (
    <>
      {collapsed ? (
        <button type="button" onClick={() => go(next.id)} title={`Switch to ${next.name}`} aria-label={`Bleach theme: ${BLEACH[at].name}. Switch to ${next.name}`} className="mb-2 grid h-9 w-full place-items-center rounded-[10px] border border-line bg-surface text-[13px] font-bold text-accent hover:border-line-strong">
          {BLEACH[at].kanji.slice(0, 1)}
        </button>
      ) : (
        <div className="mb-2 rounded-[10px] border border-line bg-surface p-1">
          <div role="radiogroup" aria-label="Bleach theme" className="grid grid-cols-2 gap-1">
            {BLEACH.map((m) => (
              <button
                key={m.id}
                type="button"
                role="radio"
                aria-checked={m.id === look}
                onClick={() => go(m.id)}
                title={`${m.name} · ${m.sub}`}
                style={{ '--opt': m.color } as React.CSSProperties}
                className={cx('bleach-opt flex flex-col gap-1 rounded-lg px-1.5 py-1.5 text-left leading-tight', m.id === look ? 'is-on bg-surface-2 text-fg' : 'text-dim hover:bg-surface-2 hover:text-fg')}
              >
                <span className={cx('bleach-chip', `c-${m.id}`)} />
                <span className="flex items-baseline gap-1.5">
                  <span className="text-[12px] font-bold" style={{ color: m.id === look ? m.color : undefined }}>
                    {m.kanji}
                  </span>
                  <span className="truncate font-display text-[10.5px] font-semibold uppercase tracking-[0.06em]">{m.id === 'hakka' ? 'Hakka' : m.name}</span>
                </span>
              </button>
            ))}
          </div>
          <button type="button" onClick={() => setArt(true)} className="mt-1 flex w-full items-center justify-center gap-1.5 rounded-md py-1 text-[11px] text-faint transition-colors hover:bg-surface-2 hover:text-fg">
            <ImagePlus size={12} /> Picture
          </button>
        </div>
      )}
      {flash && createPortal(<div key={flash.key} className="bleach-flash" data-mode={flash.mode} aria-hidden />, document.body)}
      <ArtDialog look={look} open={art} onClose={() => setArt(false)} />
    </>
  );
}
