// What fills the screen behind Aurora's lyrics, edge to edge at any size:
//
//   1. the song's music video (the artist's own upload on YouTube), muted and
//      kept at the song's position, as sharp as the screen allows;
//   2. else the artist's visualizer (usually the cover, animated) — the same
//      player, it is just the next video in the list;
//   3. else the cover itself, full size (3000×3000 for most songs), cropped
//      around its detail and drifting slowly, breathing with the beat;
//   4. with no cover, AuroraStage's light show in the song's colours.
//
// Over it: a scrim that darkens behind the lyrics as much as the picture
// needs, soft light from the corners in the song's colours, and a vignette.
// The video fades in only once it plays at the right moment, so nothing
// half-loaded or out of step is ever shown; the cover stays underneath.

import type { MusicVideo, VisualSettings } from '@shared/types';
import { AnimatePresence, motion } from 'motion/react';
import { memo, useEffect, useRef, useState } from 'react';
import type { CoverLook } from '../../lib/aurora/coverLook';
import { cx } from '../../lib/cx';
import { embedUrl, PlayerLink, qualityLabel } from '../../lib/aurora/youtube';

export interface VideoNow {
  video: MusicVideo;
  /** What YouTube says it is playing ("4K (2160p)", "1080p"…), when known. */
  quality: string;
  /** On screen now (playing, in place). */
  showing: boolean;
  offsetMs: number;
  /** The song went past the end of the video. */
  ended: boolean;
}

export interface BackdropInput {
  look: CoverLook | null;
  /** Candidates, best first (null: none, or not looked up). */
  videos: MusicVideo[] | null;
  /** The song's position, ms. */
  time: () => number;
  playing: boolean;
  /** The user's nudge for a video (ms, + shows later parts of the video). */
  offsetFor: (id: string) => number;
  onVideo?: (v: VideoNow | null) => void;
  /** Where the lyrics sit (0 top – 1 bottom), for the scrim. */
  lyricY: number;
  /** How bright the picture is behind them (0–1): bright needs more darkening. */
  glare: number;
}

interface Props extends BackdropInput {
  scene: VisualSettings['scene'];
  art: string | null;
  reduced: boolean;
}

const STYLE = `
@keyframes aurora-drift {
  0% { transform: translate3d(calc(var(--m) * -1.6%), calc(var(--m) * -1.1%), 0) scale(calc(1 + var(--m) * 0.02)); }
  50% { transform: translate3d(calc(var(--m) * 0.6%), calc(var(--m) * 1.2%), 0) scale(calc(1 + var(--m) * 0.075)); }
  100% { transform: translate3d(calc(var(--m) * 1.6%), calc(var(--m) * -0.4%), 0) scale(calc(1 + var(--m) * 0.045)); }
}`;

const AR = 16 / 9;

/** The video's frame: covering (fill) or whole (fit), within the quality cap. */
function frameFor(W: number, H: number, fit: 'fill' | 'fit', focus: number, quality: VisualSettings['scene']['videoQuality']) {
  let vw = fit === 'fill' ? Math.max(W, H * AR) : Math.min(W, H * AR);
  // A sliver more than the screen keeps YouTube's corner marks off it.
  if (fit === 'fill') vw *= 1.04;
  const vh = vw / AR;
  const dpr = typeof window !== 'undefined' ? window.devicePixelRatio || 1 : 1;
  // YouTube picks the quality from the player's size in device pixels: a
  // smaller frame scaled up caps it.
  const cap = quality === '720' ? 1280 : quality === '1080' ? 1920 : Infinity;
  const cssW = Math.min(vw, cap / dpr);
  return { left: (W - vw) / 2, top: fit === 'fill' ? (H - vh) * focus : (H - vh) / 2, width: cssW, height: cssW / AR, scale: vw / cssW };
}

function VideoLayer({ video, scene, input, size, onFail }: { video: MusicVideo; scene: Props['scene']; input: BackdropInput; size: { W: number; H: number }; onFail: () => void }) {
  const frame = useRef<HTMLIFrameElement>(null);
  const [link, setLink] = useState<PlayerLink | null>(null);
  const [showing, setShowing] = useState(false);
  const live = useRef(input);
  live.current = input;
  // The first frame starts where the song is, so it needs no seek.
  const [start] = useState(() => (input.time() + input.offsetFor(video.id)) / 1000);

  useEffect(() => {
    const f = frame.current;
    if (!f) return;
    const l = new PlayerLink(f, () => undefined);
    setLink(l);
    return () => l.destroy();
  }, [video.id]);

  useEffect(() => {
    if (!link) return;
    const born = performance.now();
    let lastSeek = 0;
    let steadySince = 0;
    let shown = false;
    let lastRate = 1;
    let report = '';
    const tick = () => {
      const { time, playing, offsetFor, onVideo } = live.current;
      const i = link.info;
      const now = performance.now();
      if (i.error !== null || (!i.ready && now - born > 15000)) {
        onFail();
        return;
      }
      if (!i.ready) return;
      const offsetMs = offsetFor(video.id);
      const target = (time() + offsetMs) / 1000;
      const ended = i.duration > 0 && target > i.duration - 0.25;
      if (!playing || ended) {
        if (i.state === 1) link.command('pauseVideo');
      } else if (i.state !== 1 && i.state !== 3) link.command('playVideo');
      const drift = link.now() - target;
      if (playing && !ended && Math.abs(drift) > 1 && now - lastSeek > 1500) {
        link.command('seekTo', [Math.max(0, target), true]);
        lastSeek = now;
      }
      // Small drifts: a touch faster or slower until it lines up.
      const rate = playing && Math.abs(drift) > 0.12 && Math.abs(drift) <= 1 ? (drift > 0 ? 0.94 : 1.06) : 1;
      if (rate !== lastRate) {
        link.command('setPlaybackRate', [rate]);
        lastRate = rate;
      }
      // On screen once it has played in place for a moment (YouTube's title
      // card is gone by then, and so is any half-loaded frame).
      const inPlace = !ended && (i.state === 1 || (!playing && i.state === 2)) && Math.abs(drift) < 1.5;
      if (inPlace) steadySince ||= now;
      else steadySince = 0;
      const show = steadySince > 0 && now - steadySince > (shown ? 0 : 1200);
      if (show !== shown) {
        shown = show;
        setShowing(show);
      }
      const r = JSON.stringify([video.id, i.quality, show, offsetMs, ended]);
      if (r !== report) {
        report = r;
        onVideo?.({ video, quality: qualityLabel(i.quality), showing: show, offsetMs, ended });
      }
    };
    const timer = setInterval(tick, 300);
    return () => clearInterval(timer);
  }, [link, video, onFail]);

  const f = frameFor(size.W, size.H, scene.videoFit, scene.videoFocus, scene.videoQuality);
  return (
    <div className="absolute inset-0 transition-opacity duration-[900ms] ease-out" style={{ opacity: showing ? 1 : 0 }} data-aurora-video={video.id} data-showing={showing || undefined}>
      <iframe
        ref={frame}
        title={`${video.title} — music video`}
        src={embedUrl(video.id, start)}
        allow="autoplay; encrypted-media; picture-in-picture"
        referrerPolicy="strict-origin-when-cross-origin"
        tabIndex={-1}
        className="pointer-events-none absolute origin-top-left border-0"
        style={{ left: f.left, top: f.top, width: f.width, height: f.height, transform: f.scale !== 1 ? `scale(${f.scale})` : undefined }}
      />
    </div>
  );
}

export const AuroraBackdrop = memo(function AuroraBackdrop({ scene, art, reduced, look, videos, time, playing, offsetFor, onVideo, lyricY, glare }: Props) {
  const root = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ W: 1600, H: 900 });
  useEffect(() => {
    const el = root.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setSize({ W: el.clientWidth || 1600, H: el.clientHeight || 900 }));
    ro.observe(el);
    setSize({ W: el.clientWidth || 1600, H: el.clientHeight || 900 });
    return () => ro.disconnect();
  }, []);

  // The next candidate when YouTube will not play one here (embedding off, removed).
  const list = scene.backdrop === 'cover' || !scene.musicVideos ? [] : (videos ?? []);
  const listKey = list.map((v) => v.id).join(',');
  const [failed, setFailed] = useState<string[]>([]);
  useEffect(() => setFailed([]), [listKey]);
  const video = list.find((v) => !failed.includes(v.id)) ?? null;
  const fail = useRef<() => void>(() => undefined);
  fail.current = () => video && setFailed((f) => [...f, video.id]);
  const [onFail] = useState(() => () => fail.current());
  useEffect(() => {
    if (!video) onVideo?.(null);
  }, [video, onVideo]);

  const m = reduced ? 0 : Math.max(0, Math.min(1, scene.coverMotion));
  const input: BackdropInput = { look, videos, time, playing, offsetFor, onVideo, lyricY, glare };
  // The user's amount, more over a bright picture, less over a dark one.
  const dim = Math.max(0, Math.min(1, scene.dim * (0.55 + 1.1 * Math.max(0, Math.min(1, glare)))));
  const focus = look?.focusY ?? 0.45;
  const corners = Math.max(0, Math.min(1, scene.corners));

  return (
    // With no cover (only a video), the light show stays visible underneath.
    <div ref={root} className={cx('absolute inset-0 overflow-hidden', art && 'bg-black')} data-aurora-backdrop>
      <style>{STYLE}</style>
      {/* The cover: under the video too, so a gap or a late video never shows black. */}
      <AnimatePresence initial={false}>
        {art && (
          <motion.div key={art} className="absolute inset-0" initial={{ opacity: 0, scale: 1.04 }} animate={{ opacity: 1, scale: 1 }} exit={{ opacity: 0 }} transition={{ duration: reduced ? 0.4 : 1.4, ease: [0.22, 1, 0.36, 1] }}>
            <div className="absolute inset-0" style={{ transform: `scale(calc(1 + var(--aurora-beat, 0) * ${(0.014 * m).toFixed(4)}))`, willChange: 'transform' }}>
              <img
                src={art}
                alt=""
                draggable={false}
                decoding="async"
                className="absolute left-[-5%] top-[-5%] h-[110%] w-[110%] max-w-none object-cover"
                style={{ objectPosition: `50% ${Math.round(focus * 100)}%`, ['--m' as string]: m, animation: m > 0 ? `aurora-drift ${Math.round(46 / Math.max(0.3, scene.speed))}s ease-in-out infinite alternate` : undefined, animationPlayState: playing ? 'running' : 'paused', willChange: 'transform' }}
                data-aurora-cover
              />
            </div>
          </motion.div>
        )}
      </AnimatePresence>
      {/* "Whole picture": the cover blurred around the video. */}
      {video && scene.videoFit === 'fit' && art && <img src={art} alt="" aria-hidden className="absolute inset-0 h-full w-full scale-110 object-cover opacity-70 blur-3xl" />}
      {video && <VideoLayer key={video.id} video={video} scene={scene} input={input} size={size} onFail={onFail} />}
      {/* Readability: behind the lyrics the picture softens into a feathered
          patch of frosted glass and darkens a little; the rest stays sharp. */}
      {dim > 0.01 && (
        <div
          className="pointer-events-none absolute inset-0"
          style={{
            backdropFilter: `blur(${Math.round(4 + 16 * dim)}px) saturate(1.15)`,
            WebkitBackdropFilter: `blur(${Math.round(4 + 16 * dim)}px) saturate(1.15)`,
            maskImage: `radial-gradient(ellipse 46% 26% at 50% ${Math.round(lyricY * 100)}%, #000 25%, transparent 100%)`,
            WebkitMaskImage: `radial-gradient(ellipse 46% 26% at 50% ${Math.round(lyricY * 100)}%, #000 25%, transparent 100%)`,
          }}
          data-aurora-frost
        />
      )}
      <div
        className="pointer-events-none absolute inset-0"
        style={{
          background: `radial-gradient(ellipse 75% 42% at 50% ${Math.round(lyricY * 100)}%, rgba(0,0,0,${(0.55 * dim).toFixed(3)}), rgba(0,0,0,${(0.18 * dim).toFixed(3)}) 70%, rgba(0,0,0,0) 100%), rgba(0,0,0,${(0.32 * dim).toFixed(3)})`,
          transition: 'background .8s ease',
        }}
      />
      {/* Soft light from the corners, in the song's colours, breathing with the bass. */}
      {corners > 0.01 && (
        <div
          className="pointer-events-none absolute inset-0 mix-blend-screen"
          style={{
            opacity: `calc(${corners.toFixed(3)} * (0.62 + 0.38 * var(--aurora-low, 0)))`,
            background: [
              'radial-gradient(38% 46% at 0% 0%, color-mix(in oklab, var(--aurora-c1, #a855f7) 55%, transparent), transparent 72%)',
              'radial-gradient(38% 46% at 100% 0%, color-mix(in oklab, var(--aurora-c2, #ec4899) 50%, transparent), transparent 72%)',
              'radial-gradient(40% 48% at 100% 100%, color-mix(in oklab, var(--aurora-c3, #3b82f6) 55%, transparent), transparent 72%)',
              'radial-gradient(40% 48% at 0% 100%, color-mix(in oklab, var(--aurora-c4, #22d3ee) 50%, transparent), transparent 72%)',
            ].join(', '),
          }}
          data-aurora-corners
        />
      )}
      {/* A cinema vignette. */}
      <div className="pointer-events-none absolute inset-0" style={{ background: 'radial-gradient(ellipse 85% 80% at 50% 50%, transparent 58%, rgba(0,0,0,.5) 100%)' }} />
    </div>
  );
});
