// The music video in YouTube's own embedded player (youtube-nocookie.com),
// steered the way YouTube's IFrame API does it: JSON messages to and from the
// player's frame. No YouTube script runs in OmniHub's page, and the player
// sets its own cookies only in its own frame.
//
// The player plays muted; the sound is still the user's music app. Aurora
// keeps it at the song's position (plus the user's nudge for videos longer
// than the song), seeking when it drifts more than a second and easing the
// playback rate for smaller drifts.

export const YT_ORIGIN = 'https://www.youtube-nocookie.com';

/** YouTube's errors: 2 bad id, 5 HTML5 error, 100 removed, 101/150 embedding not allowed, 152/153 player blocked. */
export interface PlayerInfo {
  ready: boolean;
  /** -1 unstarted, 0 ended, 1 playing, 2 paused, 3 buffering, 5 cued. */
  state: number;
  /** Seconds, as last reported, and when (performance.now()). */
  time: number;
  at: number;
  rate: number;
  duration: number;
  /** "hd2160", "hd1440", "hd1080", "hd720", "large"… */
  quality: string;
  error: number | null;
}

/** The embed for a video, starting near `startSec`, muted, without YouTube's controls. */
export function embedUrl(id: string, startSec: number): string {
  const origin = typeof location !== 'undefined' ? location.origin : '';
  const q = new URLSearchParams({
    enablejsapi: '1',
    autoplay: '1',
    mute: '1',
    controls: '0',
    disablekb: '1',
    fs: '0',
    iv_load_policy: '3',
    cc_load_policy: '0',
    modestbranding: '1',
    playsinline: '1',
    rel: '0',
    start: String(Math.max(0, Math.floor(startSec))),
    origin,
    widget_referrer: origin,
  });
  return `${YT_ORIGIN}/embed/${encodeURIComponent(id)}?${q}`;
}

/** "hd2160" → "4K (2160p)", honestly: what YouTube says it is playing. */
export function qualityLabel(q: string): string {
  const m = /(\d{3,4})/.exec(q);
  const lines = m ? Number(m[1]) : ({ large: 480, medium: 360, small: 240, tiny: 144 } as Record<string, number>)[q];
  if (!lines) return '';
  return lines >= 2160 ? `4K (${lines}p)` : lines >= 1440 ? `1440p` : `${lines}p`;
}

export class PlayerLink {
  info: PlayerInfo = { ready: false, state: -1, time: 0, at: 0, rate: 1, duration: 0, quality: '', error: null };
  private readonly id = Math.floor(Math.random() * 1e9);
  private hello: ReturnType<typeof setInterval> | undefined;

  constructor(
    private readonly frame: HTMLIFrameElement,
    private readonly onChange: () => void,
  ) {
    window.addEventListener('message', this.onMessage);
    // Until the player answers, keep saying we are listening (as the API does).
    this.hello = setInterval(() => {
      if (this.info.ready) {
        clearInterval(this.hello);
        return;
      }
      this.post({ event: 'listening', id: this.id, channel: 'widget' });
    }, 250);
  }

  private post(msg: object) {
    try {
      this.frame.contentWindow?.postMessage(JSON.stringify(msg), YT_ORIGIN);
    } catch {
      /* the frame is going away */
    }
  }

  command(func: string, args: unknown[] = []) {
    this.post({ event: 'command', func, args, id: this.id, channel: 'widget' });
  }

  /** The video's position now (seconds), from its last report. */
  now(): number {
    const i = this.info;
    return i.state === 1 ? i.time + ((performance.now() - i.at) / 1000) * i.rate : i.time;
  }

  private onMessage = (e: MessageEvent) => {
    if (e.source !== this.frame.contentWindow) return;
    let d: { event?: string; info?: unknown };
    try {
      d = typeof e.data === 'string' ? JSON.parse(e.data) : e.data;
    } catch {
      return;
    }
    const i = this.info;
    switch (d?.event) {
      case 'onReady':
        i.ready = true;
        this.command('mute');
        break;
      case 'initialDelivery':
      case 'infoDelivery': {
        const info = (d.info ?? {}) as Record<string, unknown>;
        i.ready = true;
        if (typeof info.currentTime === 'number') {
          i.time = info.currentTime;
          i.at = performance.now();
        }
        if (typeof info.playerState === 'number') i.state = info.playerState;
        if (typeof info.duration === 'number' && info.duration > 0) i.duration = info.duration;
        if (typeof info.playbackQuality === 'string') i.quality = info.playbackQuality;
        if (typeof info.playbackRate === 'number') i.rate = info.playbackRate;
        break;
      }
      case 'onStateChange':
        if (typeof d.info === 'number') {
          i.state = d.info;
          if (d.info === 1) i.at = performance.now();
        }
        break;
      case 'onPlaybackQualityChange':
        if (typeof d.info === 'string') i.quality = d.info;
        break;
      case 'onError':
        i.error = typeof d.info === 'number' ? d.info : 0;
        break;
      default:
        return;
    }
    this.onChange();
  };

  destroy() {
    clearInterval(this.hello);
    window.removeEventListener('message', this.onMessage);
  }
}
