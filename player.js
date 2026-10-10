// OmniHub's music-video relay.
//
// YouTube refuses to play official music videos inside apps: a player embedded
// by a page on a local address (OmniHub's window is http://tauri.localhost)
// answers with error 150 for nearly every one. So Aurora shows this page, and
// this page embeds YouTube's player the way any website does. It only passes
// the player's own messages (YouTube's IFrame API protocol) between OmniHub and
// the player, both ways: nothing is stored, nothing is tracked.
(() => {
  const YT = 'https://www.youtube-nocookie.com';
  // The windows that may drive the player: OmniHub, and its dev server.
  const APP = ['http://tauri.localhost', 'https://tauri.localhost', 'tauri://localhost', 'http://127.0.0.1:1420', 'http://localhost:1420'];
  const q = new URLSearchParams(location.search);
  const id = q.get('v') || '';
  if (!/^[\w-]{11}$/.test(id) || window.parent === window) return;
  const start = Math.max(0, parseInt(q.get('start') || '0', 10) || 0);
  const p = new URLSearchParams({
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
    start: String(start),
    origin: location.origin,
    widget_referrer: location.origin + location.pathname,
  });
  const frame = document.createElement('iframe');
  frame.src = `${YT}/embed/${id}?${p}`;
  frame.allow = 'autoplay; encrypted-media; picture-in-picture';
  frame.referrerPolicy = 'strict-origin-when-cross-origin';
  frame.title = 'Music video';
  document.body.append(frame);
  // The app's origin, from its first message (YouTube only talks once asked).
  let app = null;
  addEventListener('message', (e) => {
    if (e.source === window.parent && APP.includes(e.origin)) {
      app = e.origin;
      frame.contentWindow?.postMessage(e.data, YT);
    } else if (app && e.source === frame.contentWindow && e.origin === YT) {
      window.parent.postMessage(e.data, app);
    }
  });
})();
