// Animated album covers (Apple Music's "motion artwork"): an HLS stream of a
// short loop, square, up to 3840×3840. WebView2 has no HLS of its own, so
// hls.js feeds the <video> (loaded only when a song has one). The sharpest
// version the PC can decode (H.264 up to 1080×1080, HEVC beyond where the
// PC supports it) is used, up to what the screen can show.

/** Play `url` (an HLS playlist) in `el`; returns a function that stops it. */
export async function attachHls(el: HTMLVideoElement, url: string): Promise<() => void> {
  el.muted = true;
  el.loop = true;
  el.playsInline = true;
  if (el.canPlayType('application/vnd.apple.mpegurl')) {
    el.src = url;
    void el.play().catch(() => undefined);
    return () => {
      el.removeAttribute('src');
      el.load();
    };
  }
  const { default: Hls } = await import('hls.js');
  if (!Hls.isSupported()) throw new Error('This PC cannot play animated covers.');
  // Workers would need their own content-security rule; the stream is small.
  const hls = new Hls({ enableWorker: false, maxBufferLength: 20, capLevelOnFPSDrop: true });
  const cap = Math.max(1080, Math.round((window.screen?.height || 1080) * (window.devicePixelRatio || 1)));
  hls.on(Hls.Events.MANIFEST_PARSED, () => {
    // Levels the PC cannot decode are already left out; take the sharpest up to the cap.
    let best = -1;
    hls.levels.forEach((l, i) => {
      if (l.height <= cap && (best < 0 || l.height > hls.levels[best].height || (l.height === hls.levels[best].height && l.bitrate > hls.levels[best].bitrate))) best = i;
    });
    if (best >= 0) {
      hls.startLevel = best;
      hls.currentLevel = best;
    }
    void el.play().catch(() => undefined);
  });
  hls.loadSource(url);
  hls.attachMedia(el);
  return () => hls.destroy();
}
