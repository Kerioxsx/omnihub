// Aurora's light: what fills the screen behind the lyrics (the music video or
// the cover, see AuroraBackdrop; or the light show built from the cover) and
// the glow around the frame, driven by one animation loop. Sound comes from the core's analysis of
// what the PC plays (see lib/aurora/audio.ts); colours from the cover art or
// the user's palette (lib/aurora/palette.ts). Nothing here re-renders React
// per frame. The loop drops to 30 fps while paused or with reduced motion,
// and the browser stops it while the window is hidden.

import type { VisualSettings, VisualStatus } from '@shared/types';
import { type RefObject, useEffect, useRef, useState } from 'react';
import { LevelFollower, useAudioFeed } from '../../lib/aurora/audio';
import { cx } from '../../lib/cx';
import { DEFAULT_EFFECTS } from '../../lib/aurora/defaults';
import { GlowRenderer, SceneRenderer } from '../../lib/aurora/gl';
import { albumPalette, glowPalette, type Lab, PaletteBlender, rgbToHex, targetPalette } from '../../lib/aurora/palette';
import { AuroraBackdrop, type BackdropInput } from './AuroraBackdrop';

export interface StageProps {
  visuals: VisualSettings;
  art: string | null;
  playing: boolean;
  /** The lyrics' box, kept calm in the scene. */
  calmRef?: RefObject<HTMLElement | null>;
  /** Inside a card (the Music page) rather than the whole window. */
  framed: boolean;
  /** Name of this view's hold on the sound analysis. */
  who: string;
  /** Where the spectrum line sits (0 bottom – 1 top). */
  waveY: number;
  onStatus?: (s: VisualStatus | null) => void;
  /** Receives the palette blender (for the lyrics' accent). */
  blender: PaletteBlender;
  /** The screen glow's see-through window: the edge light only, nothing behind it. */
  overlay?: boolean;
  /** The player: the music video or the cover behind everything (when the backdrop is not the light show). */
  backdrop?: BackdropInput;
  reduced?: boolean;
}

export function AuroraStage({ visuals, art, playing, calmRef, framed, who, waveY, onStatus, blender, overlay = false, backdrop, reduced: appReduced = false }: StageProps) {
  const root = useRef<HTMLDivElement>(null);
  const sceneCanvas = useRef<HTMLCanvasElement>(null);
  const glowCanvas = useRef<HTMLCanvasElement>(null);
  const [failed, setFailed] = useState<string | null>(null);
  const [generation, setGeneration] = useState(0);
  const on = visuals.enabled;
  const status = useAudioFeed(on && visuals.audioReactive, who);
  useEffect(() => onStatus?.(status), [status, onStatus]);

  // The cover or the music video fills the screen (the light show only when
  // asked for, or when a song has no cover at all).
  const cinema = !overlay && !!backdrop && visuals.scene.backdrop !== 'visual';
  const lightShow = !cinema || !art;
  // Live values for the loop.
  const live = useRef({ visuals, playing, waveY, framed, cinema, lightShow });
  live.current = { visuals, playing, waveY, framed, cinema, lightShow };

  // Palette target: cover art or the user's colours.
  const [album, setAlbum] = useState<Lab[] | null>(null);
  useEffect(() => {
    if (!art) return setAlbum(null);
    let alive = true;
    albumPalette(art).then(
      (p) => alive && setAlbum(p),
      () => alive && setAlbum(null),
    );
    return () => {
      alive = false;
    };
  }, [art]);
  const [glowBlender] = useState(() => new PaletteBlender());
  useEffect(() => {
    blender.set(targetPalette(visuals.color, album));
    glowBlender.set(glowPalette(visuals.color, album));
  }, [blender, glowBlender, visuals.color, album]);

  // Renderers and the loop.
  const scene = useRef<SceneRenderer | null>(null);
  useEffect(() => {
    if (!on) return;
    let glow: GlowRenderer | null = null;
    try {
      if (sceneCanvas.current) scene.current = new SceneRenderer(sceneCanvas.current);
      if (glowCanvas.current) glow = new GlowRenderer(glowCanvas.current);
      setFailed(null);
    } catch (e) {
      setFailed(e instanceof Error ? e.message : String(e));
      return;
    }
    const lost = (e: Event) => {
      e.preventDefault();
    };
    const restored = () => setGeneration((g) => g + 1);
    const canvases = [sceneCanvas.current, glowCanvas.current];
    canvases.forEach((c) => {
      c?.addEventListener('webglcontextlost', lost);
      c?.addEventListener('webglcontextrestored', restored);
    });

    const follower = new LevelFollower();
    let raf = 0;
    let last = performance.now();
    let time = Math.random() * 100;
    let idle = 1;
    let show = 0;
    let seed = 1;
    let seeded = 0;
    let glowTime = time;
    const quiet = new LevelFollower().v;
    let calmK = 0;
    let calm: [number, number, number, number] = [0.5, 0.5, 0.3, 0.12];
    let measured = 0;
    let wantCalm = 0;
    let vars = 0;
    const loop = (now: number) => {
      raf = requestAnimationFrame(loop);
      const { visuals: v, playing: isPlaying, waveY: wy, framed: inCard, cinema: cin, lightShow: drawScene } = live.current;
      const reduced = v.reducedMotion;
      if ((reduced || !isPlaying) && now - last < 1000 / 30 - 2) return;
      const dt = Math.min(0.1, (now - last) / 1000);
      last = now;

      // Paused: everything settles; reduced motion: a gentler response.
      const lv = follower.step(dt, isPlaying ? (reduced ? 0.55 : 0) : 0.85);
      idle += ((lv.active ? 0 : 1) - idle) * (1 - Math.exp(-dt * 1.5));
      time += dt * v.speed * (reduced ? 0.2 : 1) * (isPlaying ? 1 : 0.3);
      const colors = blender.step(dt, reduced ? 3 : 1.2);
      const edgeColors = glowBlender.step(dt, reduced ? 3 : 1.2);
      const flash = v.noFlashes || reduced ? 0 : 1;
      const motion = reduced ? 0 : 1;

      // Where the lyrics are, a few times a second.
      if (now - measured > 250) {
        measured = now;
        const box = calmRef?.current?.getBoundingClientRect();
        const stage = root.current?.getBoundingClientRect();
        wantCalm = box && stage && box.width > 4 && box.height > 4 && stage.width > 0 ? 1 : 0;
        if (box && stage && wantCalm) {
          calm = [(box.left + box.width / 2 - stage.left) / stage.width, 1 - (box.top + box.height / 2 - stage.top) / stage.height, (box.width / 2 / stage.width) * 1.1 + 0.04, (box.height / 2 / stage.height) * 1.2 + 0.05];
        }
      }
      calmK += (wantCalm - calmK) * (1 - Math.exp(-dt * 3));

      const dpr = window.devicePixelRatio || 1;
      const sc = scene.current;
      // Paused: the cover shrinks back and the scene dims (the player shows a card).
      show += ((isPlaying ? 1 : 0.16) - show) * (1 - Math.exp(-dt * (isPlaying ? 2.2 : 1.6)));
      // A new glitch pattern a few times a second, more often on beats.
      if (!reduced && now - seeded > 1000 / (3 + 12 * lv.beat)) {
        seeded = now;
        seed = Math.random() * 97;
      }
      if (sc && !sc.lost && v.scene.enabled && drawScene) {
        const w = sc.canvas.getBoundingClientRect().width || 1;
        sc.resize(Math.max(0.3, Math.min(dpr, 1920 / w)));
        const st = v.scene.style;
        const fx = { ...DEFAULT_EFFECTS, ...v.scene.effects };
        // With no cover behind the lyrics: colour fields in the song's palette.
        const coverless = !v.scene.artwork || st === 'ambient' || cin;
        sc.draw(time, colors, lv, flash, motion, {
          intensity: v.scene.intensity,
          blur: v.scene.blur,
          saturation: v.scene.saturation * v.color.vividness,
          opacity: v.scene.opacity,
          // The style is a starting point; the effects set how much of each.
          glitch: fx.glitch * (st === 'fisheye' ? 0.3 : 1),
          lens: st === 'fisheye' ? Math.max(1, fx.fisheye) : st === 'fisheyeVisual' ? Math.max(0.8, fx.fisheye) : fx.fisheye,
          effects: fx,
          ambient: coverless ? 1 : 0,
          minimal: st === 'minimal' ? 1 : 0,
          show,
          seed,
          wave: v.scene.waveform,
          waveY: wy,
          calm,
          calmStrength: calmK,
        });
      }
      if (glow && !glow.lost && v.glow.enabled) {
        glow.resize(dpr);
        const g = v.glow;
        // Music: follows the sound; Idle: drifts on its own; None: still.
        const anim = g.animation;
        if (anim !== 'none') glowTime += dt * v.speed * (reduced ? 0.2 : 1);
        glow.draw(glowTime, edgeColors, anim === 'music' ? lv : quiet, anim === 'music' ? flash : 0, anim === 'none' ? 0 : motion, {
          dpr,
          radius: inCard ? Math.max(16, g.radius) : g.radius,
          inset: 0,
          thickness: g.thickness,
          edge: g.edge,
          highlight: g.highlight,
          glow: g.glow,
          glowSize: g.glowSize,
          bloom: g.bloom,
          bloomSize: g.bloomSize,
          intensity: g.intensity * (isPlaying || anim !== 'music' ? 1 : 0.75),
          idle: anim === 'none' ? 0 : anim === 'idle' ? 1 : idle,
        });
      }
      // The beat for the cover's breathing and the corner light (cheap: one element).
      if (cin && root.current) {
        root.current.style.setProperty('--aurora-beat', lv.beat.toFixed(3));
        root.current.style.setProperty('--aurora-low', lv.low.toFixed(3));
      }
      // The song's colours for everything around (lyrics, dock, corners).
      const host = root.current?.parentElement;
      if (now - vars > 150 && host) {
        vars = now;
        host.style.setProperty('--aurora-accent', glowBlender.accent());
        for (let k = 0; k < 4; k++) host.style.setProperty(`--aurora-c${k + 1}`, rgbToHex([edgeColors[k * 3], edgeColors[k * 3 + 1], edgeColors[k * 3 + 2]]));
      }
    };
    raf = requestAnimationFrame(loop);
    return () => {
      cancelAnimationFrame(raf);
      canvases.forEach((c) => {
        c?.removeEventListener('webglcontextlost', lost);
        c?.removeEventListener('webglcontextrestored', restored);
      });
      scene.current = null;
    };
  }, [on, generation, blender, glowBlender, calmRef]);

  // The cover, for the scene.
  useEffect(() => {
    const sc = scene.current;
    if (!sc) return;
    // The light show only (the full-screen cover is drawn by the backdrop).
    if (!art || !visuals.scene.artwork || !lightShow) return sc.setArt(null);
    const img = new Image();
    img.onload = () => scene.current?.setArt(img);
    img.src = art;
  }, [art, visuals.scene.artwork, generation, on, failed, lightShow]);

  if (!on) return <div ref={root} className={cx('pointer-events-none absolute inset-0', !overlay && 'bg-black')} aria-hidden />;
  return (
    <div ref={root} className={cx('pointer-events-none absolute inset-0 overflow-hidden', !overlay && 'bg-black')} aria-hidden data-aurora-stage>
      {cinema && backdrop && (art || !!backdrop.videos?.length) && <AuroraBackdrop scene={visuals.scene} art={art} reduced={visuals.reducedMotion || appReduced} {...backdrop} />}
      {failed ? (
        // No WebGL: a still gradient in the palette and a CSS glow.
        <div
          className="absolute inset-0"
          style={{
            background: overlay || !lightShow ? undefined : 'radial-gradient(70% 60% at 30% 30%, color-mix(in oklab, var(--aurora-accent, #8b5cf6) 45%, transparent), transparent), #000',
            boxShadow: visuals.glow.enabled ? 'inset 0 0 0 2px color-mix(in oklab, var(--aurora-accent, #8b5cf6) 80%, white), inset 0 0 40px color-mix(in oklab, var(--aurora-accent, #8b5cf6) 70%, transparent)' : undefined,
          }}
        />
      ) : (
        <>
          {!overlay && <canvas key={`s${generation}`} ref={sceneCanvas} className="absolute inset-0 h-full w-full" style={{ opacity: visuals.scene.enabled && lightShow ? 1 : 0, transition: 'opacity .6s ease' }} />}
          <canvas key={`g${generation}`} ref={glowCanvas} className={cx('absolute inset-0 z-30 h-full w-full', !overlay && 'mix-blend-screen')} style={{ opacity: visuals.glow.enabled ? 1 : 0, transition: 'opacity .6s ease' }} />
        </>
      )}
    </div>
  );
}
