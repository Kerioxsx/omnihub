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
import { api, on } from '../../api';
import { attachHls } from '../../lib/aurora/motion';
import { DirectLink, embedUrl, PlayerLink, qualityLabel, type VideoControl } from '../../lib/aurora/youtube';

export interface VideoNow {
  video: MusicVideo;
  /** What YouTube says it is playing ("4K (2160p)", "1080p"…), when known. */
  quality: string;
  /** On screen now (playing, in place). */
  showing: boolean;
  offsetMs: number;
  /** The song went past the end of the video. */
  ended: boolean;
  /** YouTube's player, or the video's own stream (one YouTube will not embed). */
  via: 'youtube' | 'direct';
  /** While the stream is being got: what is happening ("Downloading the video helper… 40%"); after giving up, why. */
  note?: string;
}

export interface BackdropInput {
  look: CoverLook | null;
  /** Candidates, best first (null: none, or not looked up). */
  videos: MusicVideo[] | null;
  /** The album's animated cover (an HLS playlist), when it has one. */
  motion: string | null;
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

/** The tallest stream to ask for, from the quality setting. */
function maxHeight(q: VisualSettings['scene']['videoQuality']): number {
  return q === '720' ? 720 : q === '1080' ? 1080 : 2160;
}

/** What YouTube's player error means, in words. */
function youtubeWhy(code: number): string {
  if (code === 101 || code === 150) return 'the artist doesn’t allow it in apps (error ' + code + ')';
  if (code === 152 || code === 153) return 'YouTube blocked its player here (error ' + code + ')';
  if (code === 100) return 'the video was removed or is private';
  if (code === 2) return 'the video id was not accepted';
  if (code === 5) return 'the player could not play it (error 5)';
  return `error ${code}`;
}

function VideoLayer({ video, scene, input, size, onFail, onShowing }: { video: MusicVideo; scene: Props['scene']; input: BackdropInput; size: { W: number; H: number }; onFail: (why: string) => void; onShowing: (on: boolean) => void }) {
  const frame = useRef<HTMLIFrameElement>(null);
  const player = useRef<HTMLVideoElement>(null);
  const [link, setLink] = useState<VideoControl | null>(null);
  const [showing, setShowing] = useState(false);
  // YouTube's player first; when it will not play the video here, the
  // video's own stream (if allowed); when that fails too, the next video.
  const [mode, setMode] = useState<'youtube' | 'fetching' | 'direct'>('youtube');
  const [stream, setStream] = useState<string | null>(null);
  const live = useRef(input);
  live.current = input;
  const allowDirect = useRef(scene.directVideos);
  allowDirect.current = scene.directVideos;
  const cap = maxHeight(scene.videoQuality);
  // The first frame starts where the song is, so it needs no seek.
  const [start] = useState(() => (input.time() + input.offsetFor(video.id)) / 1000);

  // Why YouTube's player could not play it (said again if the stream fails too).
  const youtubeFailed = useRef('');
  const giveUp = useRef((why: string) => onFail(why));
  giveUp.current = (why: string) => {
    if (mode === 'youtube') {
      youtubeFailed.current = why;
      if (allowDirect.current) setMode('fetching');
      else onFail(`YouTube: ${why}`);
    } else onFail(`${youtubeFailed.current ? `YouTube: ${youtubeFailed.current}; ` : ''}directly: ${why}`);
  };

  useEffect(() => {
    if (mode !== 'fetching') return;
    let gone = false;
    const { onVideo } = live.current;
    const note = (n: string) => onVideo?.({ video, quality: '', showing: false, offsetMs: live.current.offsetFor(video.id), ended: false, via: 'direct', note: n });
    note('YouTube won’t play this video inside apps; getting it directly…');
    let off: (() => void) | undefined;
    void on<{ done: number; total: number }>('media:streamInstall', (p) => {
      if (!gone && p.total > 0) note(`Downloading the video helper (yt-dlp), once… ${Math.min(100, Math.round((100 * p.done) / p.total))}%`);
    }).then((u) => {
      if (gone) u();
      else off = u;
    });
    api.media
      .videoStream(video.id, cap)
      .then((url) => {
        if (gone) return;
        setStream(url);
        setMode('direct');
      })
      .catch((e: unknown) => {
        if (gone) return;
        const why = String(e).replace(/^Error: /, '');
        note(why);
        // Leave the reason up a moment, then the next video or the cover.
        setTimeout(() => !gone && giveUp.current(why), 2500);
      });
    return () => {
      gone = true;
      off?.();
    };
  }, [mode, video, cap]);

  // Say at once that a video is on its way (until it plays, the cover shows).
  useEffect(() => {
    live.current.onVideo?.({ video, quality: '', showing: false, offsetMs: live.current.offsetFor(video.id), ended: false, via: 'youtube' });
  }, [video]);

  useEffect(() => {
    if (mode === 'fetching') return;
    const el = mode === 'direct' ? player.current : frame.current;
    if (!el) return;
    const l = el instanceof HTMLVideoElement ? new DirectLink(el, () => undefined) : new PlayerLink(el as HTMLIFrameElement, () => undefined);
    setLink(l);
    return () => {
      l.destroy();
      setLink(null);
    };
  }, [video.id, mode]);

  useEffect(() => {
    if (!link) return;
    const born = performance.now();
    let lastSeek = 0;
    let steadySince = 0;
    let shown = false;
    let lastRate = 1;
    let report = '';
    let everPlayed = false;
    const tick = () => {
      const { time, playing, offsetFor, onVideo } = live.current;
      const i = link.info;
      const now = performance.now();
      // YouTube says no, never answers, or loads and never starts ("Video
      // unavailable" sends no error): the next video, or the cover.
      const direct = link instanceof DirectLink;
      if (i.error !== null || (!i.ready && now - born > 15000)) {
        clearInterval(timer);
        giveUp.current(
          i.error !== null
            ? direct
              ? `the stream stopped (media error ${i.error})`
              : youtubeWhy(i.error)
            : direct
              ? 'the stream never loaded'
              : 'YouTube’s player never loaded (is youtube.com reachable?)',
        );
        return;
      }
      if (!i.ready) return;
      if (i.state === 1) everPlayed = true;
      if (!everPlayed && live.current.playing && now - born > 14000) {
        clearInterval(timer);
        giveUp.current(direct ? 'the stream loaded but never started' : 'the player loaded but the video never started (“Video unavailable”?)');
        return;
      }
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
        onShowing(show);
      }
      const via = link instanceof DirectLink ? 'direct' : 'youtube';
      const r = JSON.stringify([video.id, i.quality, show, offsetMs, ended, via]);
      if (r !== report) {
        report = r;
        onVideo?.({ video, quality: qualityLabel(i.quality), showing: show, offsetMs, ended, via });
      }
    };
    const timer = setInterval(tick, 300);
    return () => clearInterval(timer);
  }, [link, video, onShowing]);

  const f = frameFor(size.W, size.H, scene.videoFit, scene.videoFocus, scene.videoQuality);
  return (
    <div className="absolute inset-0 transition-opacity duration-[900ms] ease-out" style={{ opacity: showing ? 1 : 0 }} data-aurora-video={video.id} data-showing={showing || undefined}>
      {mode === 'youtube' && (
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
      )}
      {mode === 'direct' && stream && (
        <video
          ref={player}
          src={stream}
          muted
          playsInline
          autoPlay
          preload="auto"
          disablePictureInPicture
          tabIndex={-1}
          className="pointer-events-none absolute inset-0 h-full w-full"
          style={{ objectFit: scene.videoFit === 'fit' ? 'contain' : 'cover', objectPosition: `50% ${Math.round(scene.videoFocus * 100)}%` }}
        />
      )}
    </div>
  );
}

/**
 * The album's animated cover, looping, placed like the still cover (all of it,
 * or filling the screen). It fades in once it plays; until then, and if it
 * cannot play, the still cover shows.
 */
function MotionLayer({ url, fit, box, wide, paused }: { url: string; fit: 'whole' | 'fill'; box: { side: number; left: number; top: number }; wide: boolean; paused: boolean }) {
  const el = useRef<HTMLVideoElement>(null);
  const [on, setOn] = useState(false);
  useEffect(() => {
    const v = el.current;
    if (!v) return;
    let stop: (() => void) | undefined;
    let gone = false;
    setOn(false);
    const playing = () => setOn(true);
    v.addEventListener('playing', playing);
    attachHls(v, url).then(
      (s) => (gone ? s() : (stop = s)),
      () => undefined,
    );
    return () => {
      gone = true;
      v.removeEventListener('playing', playing);
      stop?.();
    };
  }, [url]);
  useEffect(() => {
    const v = el.current;
    if (!v) return;
    if (paused) v.pause();
    else void v.play().catch(() => undefined);
  }, [paused]);
  const mask = wide ? 'linear-gradient(to right, transparent, #000 9%, #000 91%, transparent)' : 'linear-gradient(to bottom, transparent, #000 9%, #000 91%, transparent)';
  return (
    <div className="pointer-events-none absolute inset-0 transition-opacity duration-[1200ms] ease-out" style={{ opacity: on ? 1 : 0 }} data-aurora-motion data-playing={on || undefined}>
      <video
        ref={el}
        muted
        loop
        playsInline
        disablePictureInPicture
        tabIndex={-1}
        className={fit === 'whole' ? 'absolute object-cover' : 'absolute inset-0 h-full w-full object-cover'}
        style={fit === 'whole' ? { width: box.side, height: box.side, left: box.left, top: box.top, maskImage: mask, WebkitMaskImage: mask } : undefined}
      />
    </div>
  );
}

export const AuroraBackdrop = memo(function AuroraBackdrop({ scene, art, reduced, look, videos, motion: motionUrl, time, playing, offsetFor, onVideo, lyricY, glare }: Props) {
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
  // An animated cover from Apple beats the artist's YouTube visualizer.
  const list = scene.backdrop === 'cover' || !scene.musicVideos ? [] : (videos ?? []).filter((v) => !(motionUrl && v.kind === 'visualizer'));
  const listKey = list.map((v) => v.id).join(',');
  const [failed, setFailed] = useState<{ id: string; why: string }[]>([]);
  useEffect(() => setFailed([]), [listKey]);
  const video = list.find((v) => !failed.some((f) => f.id === v.id)) ?? null;
  const fail = useRef<(why: string) => void>(() => undefined);
  fail.current = (why) => video && setFailed((f) => [...f, { id: video.id, why }]);
  const [onFail] = useState(() => (why: string) => fail.current(why));
  // None played: say why (the first one's reason; the rest are usually the same).
  const gaveUp = !video && failed.length > 0 ? failed : null;
  const lastFailed = gaveUp ? list.find((v) => v.id === gaveUp[gaveUp.length - 1].id) : undefined;
  const whyNot = gaveUp ? `Couldn’t play ${gaveUp.length === 1 ? 'the music video' : `any of the ${gaveUp.length} videos`} — ${gaveUp[0].why}. Showing ${motionUrl ? 'the animated cover' : 'the cover'}.` : null;
  useEffect(() => {
    if (video) return;
    onVideo?.(whyNot && lastFailed ? { video: lastFailed, quality: '', showing: false, offsetMs: 0, ended: false, via: 'youtube', note: whyNot } : null);
  }, [video, onVideo, whyNot, lastFailed]);

  const m = reduced ? 0 : Math.max(0, Math.min(1, scene.coverMotion));
  const input: BackdropInput = { look, videos, motion: motionUrl, time, playing, offsetFor, onVideo, lyricY, glare };
  const [videoShowing, setVideoShowing] = useState(false);
  useEffect(() => setVideoShowing(false), [video?.id]);
  // The user's amount, more over a bright picture, less over a dark one.
  const dim = Math.max(0, Math.min(1, scene.dim * (0.55 + 1.1 * Math.max(0, Math.min(1, glare)))));
  const focus = look?.focusY ?? 0.45;
  const corners = Math.max(0, Math.min(1, scene.corners));
  // The whole cover: a square as big as the screen's shorter side.
  const side = Math.min(size.W, size.H);
  const wide = size.W >= size.H;

  return (
    // With no cover (only a video), the light show stays visible underneath.
    <div ref={root} className={cx('absolute inset-0 overflow-hidden', art && 'bg-black')} data-aurora-backdrop>
      <style>{STYLE}</style>
      {/* The cover: under the video too, so a gap or a late video never shows black. */}
      <AnimatePresence initial={false}>
        {art && (
          <motion.div key={art} className="absolute inset-0" initial={{ opacity: 0, scale: 1.04 }} animate={{ opacity: 1, scale: 1 }} exit={{ opacity: 0 }} transition={{ duration: reduced ? 0.4 : 1.4, ease: [0.22, 1, 0.36, 1] }}>
            <div className="absolute inset-0" style={{ transform: `scale(calc(1 + var(--aurora-beat, 0) * ${(0.014 * m).toFixed(4)}))`, willChange: 'transform' }}>
              {scene.coverFit === 'whole' ? (
                <>
                  {/* The screen around the cover: the cover itself, enlarged and
                      softened, so it reads as one picture reaching the edges. */}
                  <img
                    src={art}
                    alt=""
                    aria-hidden
                    draggable={false}
                    decoding="async"
                    className="absolute left-[-12%] top-[-12%] h-[124%] w-[124%] max-w-none object-cover"
                    style={{ filter: 'blur(48px) saturate(1.35) brightness(0.78)', ['--m' as string]: m, animation: m > 0 ? `aurora-drift ${Math.round(60 / Math.max(0.3, scene.speed))}s ease-in-out infinite alternate-reverse` : undefined, animationPlayState: playing ? 'running' : 'paused' }}
                    data-aurora-cover-fill
                  />
                  {/* All of the cover, as large as the screen allows, its sides
                      melting into that light. */}
                  <div
                    className="absolute"
                    style={{
                      width: side,
                      height: side,
                      left: (size.W - side) / 2,
                      top: (size.H - side) / 2,
                      maskImage: wide ? 'linear-gradient(to right, transparent, #000 9%, #000 91%, transparent)' : 'linear-gradient(to bottom, transparent, #000 9%, #000 91%, transparent)',
                      WebkitMaskImage: wide ? 'linear-gradient(to right, transparent, #000 9%, #000 91%, transparent)' : 'linear-gradient(to bottom, transparent, #000 9%, #000 91%, transparent)',
                    }}
                  >
                    <img
                      src={art}
                      alt=""
                      draggable={false}
                      decoding="async"
                      className="h-full w-full object-cover"
                      style={{ ['--m' as string]: m * 0.5, animation: m > 0 ? `aurora-drift ${Math.round(46 / Math.max(0.3, scene.speed))}s ease-in-out infinite alternate` : undefined, animationPlayState: playing ? 'running' : 'paused', willChange: 'transform' }}
                      data-aurora-cover
                    />
                  </div>
                </>
              ) : (
                <img
                  src={art}
                  alt=""
                  draggable={false}
                  decoding="async"
                  className="absolute left-[-5%] top-[-5%] h-[110%] w-[110%] max-w-none object-cover"
                  style={{ objectPosition: `50% ${Math.round(focus * 100)}%`, ['--m' as string]: m, animation: m > 0 ? `aurora-drift ${Math.round(46 / Math.max(0.3, scene.speed))}s ease-in-out infinite alternate` : undefined, animationPlayState: playing ? 'running' : 'paused', willChange: 'transform' }}
                  data-aurora-cover
                />
              )}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
      {/* The album's animated cover over the still one (it rests while a music video shows). */}
      {motionUrl && art && <MotionLayer url={motionUrl} fit={scene.coverFit} box={{ side, left: (size.W - side) / 2, top: (size.H - side) / 2 }} wide={wide} paused={!playing || videoShowing} />}
      {/* "Whole picture": the cover blurred around the video. */}
      {video && scene.videoFit === 'fit' && art && <img src={art} alt="" aria-hidden className="absolute inset-0 h-full w-full scale-110 object-cover opacity-70 blur-3xl" />}
      {video && <VideoLayer key={video.id} video={video} scene={scene} input={input} size={size} onFail={onFail} onShowing={setVideoShowing} />}
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
