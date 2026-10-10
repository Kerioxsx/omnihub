// OmniHub website: download links from the latest GitHub release, the
// animated background, and the interactive demos. No libraries.

(() => {
  'use strict';

  const REPO = 'Kerioxsx/omnihub';
  const $ = (s, root = document) => root.querySelector(s);
  const $$ = (s, root = document) => Array.from(root.querySelectorAll(s));
  const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;
  const finePointer = matchMedia('(pointer: fine)').matches;
  const clamp = (v, a, b) => Math.min(b, Math.max(a, v));
  const store = {
    get(k) {
      try {
        return JSON.parse(sessionStorage.getItem(k) || 'null');
      } catch {
        return null;
      }
    },
    set(k, v) {
      try {
        sessionStorage.setItem(k, JSON.stringify(v));
      } catch {
        /* private mode */
      }
    },
  };
  const svg = (paths, size = 18) => `<svg viewBox="0 0 24 24" width="${size}" height="${size}" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${paths}</svg>`;

  const ICONS = {
    home: '<path d="m3 10 9-7 9 7v10a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><path d="M9 22V12h6v10"/>',
    drive: '<path d="M22 12H2"/><path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"/><path d="M6 16h.01M10 16h.01"/>',
    treemap: '<rect x="3" y="3" width="18" height="18" rx="2"/><path d="M3 12h9M12 3v18M12 15h9"/>',
    grew: '<path d="m22 7-8.5 8.5-5-5L2 17"/><path d="M16 7h6v6"/>',
    apps: '<rect x="3" y="3" width="7" height="7" rx="1.5"/><rect x="14" y="3" width="7" height="7" rx="1.5"/><rect x="3" y="14" width="7" height="7" rx="1.5"/><rect x="14" y="14" width="7" height="7" rx="1.5"/>',
    tasks: '<path d="M22 12h-4l-3 9L9 3l-3 9H2"/>',
    camera: '<path d="M14.5 4h-5L7 7H4a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-3l-2.5-3z"/><circle cx="12" cy="13" r="3"/>',
    idea: '<path d="M15 14c.2-1 .7-1.7 1.5-2.5 1-.9 1.5-2.2 1.5-3.5A6 6 0 0 0 6 8c0 1 .2 2.2 1.5 3.5.7.7 1.3 1.5 1.5 2.5"/><path d="M9 18h6M10 22h4"/>',
    lock: '<rect x="3" y="11" width="18" height="11" rx="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/>',
    key: '<circle cx="7.5" cy="15.5" r="5.5"/><path d="m21 2-9.6 9.6M15.5 7.5l3 3L22 7l-3-3"/>',
    screen: '<rect x="2" y="3" width="20" height="14" rx="2"/><path d="M8 21h8M12 17v4"/>',
    phone: '<rect x="5" y="2" width="14" height="20" rx="2.5"/><path d="M12 18h.01"/>',
    power: '<path d="M12 2v10"/><path d="M18.4 6.6a9 9 0 1 1-12.77.04"/>',
    music: '<path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/>',
    game: '<path d="M6 11h4M8 9v4M15 12h.01M18 10h.01"/><rect x="2" y="6" width="20" height="12" rx="4"/>',
    update: '<path d="M21 12a9 9 0 1 1-3-6.7L21 8"/><path d="M21 3v5h-5"/>',
    command: '<path d="M15 6v12a3 3 0 1 0 3-3H6a3 3 0 1 0 3 3V6a3 3 0 1 0-3 3h12a3 3 0 1 0-3-3"/>',
    send: '<path d="m22 2-7 20-4-9-9-4z"/><path d="M22 2 11 13"/>',
    volume: '<path d="M11 5 6 9H2v6h4l5 4V5z"/><path d="M15.5 8.5a5 5 0 0 1 0 7M19 5a10 10 0 0 1 0 14"/>',
    mic: '<rect x="9" y="2" width="6" height="12" rx="3"/><path d="M19 10v1a7 7 0 0 1-14 0v-1M12 18v4M8 22h8"/>',
    shield: '<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><path d="m9 12 2 2 4-4"/>',
    close: '<rect x="3" y="3" width="18" height="18" rx="2"/><path d="m15 9-6 6M9 9l6 6"/>',
  };

  /* ---------------- Download links from the latest release ---------------- */

  const fmtSize = (b) => (b >= 1048576 ? `${(b / 1048576).toFixed(1)} MB` : `${Math.max(1, Math.round(b / 1024))} KB`);
  let release = null;

  function applyRelease(r) {
    release = r;
    for (const a of $$('[data-download]')) a.href = r.url;
    for (const m of $$('[data-download-meta]')) m.textContent = `Version ${r.version} · ${r.size ? `${fmtSize(r.size)} · ` : ''}Windows 10 & 11`;
    const name = $('[data-asset-name]');
    if (name) name.textContent = r.name;
    const pill = $('[data-version-text]');
    if (pill) pill.textContent = `OmniHub ${r.version} is out · free for Windows`;
  }

  async function loadRelease() {
    const cached = store.get('omnihub-release');
    if (cached && Date.now() - cached.at < 10 * 60 * 1000) return applyRelease(cached);
    try {
      const res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, { headers: { Accept: 'application/vnd.github+json' } });
      if (!res.ok) return;
      const json = await res.json();
      const assets = json.assets || [];
      const asset = assets.find((a) => /x64-setup\.exe$/i.test(a.name)) || assets.find((a) => /\.exe$/i.test(a.name)) || assets.find((a) => /\.msi$/i.test(a.name));
      if (!asset) return;
      const r = { version: String(json.tag_name || '').replace(/^v/, ''), url: asset.browser_download_url, name: asset.name, size: asset.size, at: Date.now() };
      store.set('omnihub-release', r);
      applyRelease(r);
    } catch {
      /* Offline or rate-limited: the links still open the Releases page. */
    }
  }
  loadRelease();

  // A short how-to after the download starts.
  const toast = document.createElement('div');
  toast.className = 'toast';
  toast.setAttribute('role', 'status');
  document.body.append(toast);
  let toastTimer = 0;
  function showToast(html) {
    toast.innerHTML = html;
    toast.classList.add('show');
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => toast.classList.remove('show'), 9000);
  }
  toast.addEventListener('click', () => toast.classList.remove('show'));
  for (const a of $$('[data-download]')) {
    a.addEventListener('click', () => {
      if (!release) return;
      showToast(`<b>Downloading OmniHub ${release.version}…</b><span>Open it when it finishes. If Windows says it protected your PC, choose <i>More info → Run anyway</i>.</span>`);
    });
  }

  /* ---------------- Navigation, progress, cursor light ---------------- */

  const nav = $('.nav');
  const bar = $('.progress span');
  const menu = $('.menu');
  const links = $('.links');
  let scrollQueued = false;

  function onScroll() {
    scrollQueued = false;
    const y = scrollY;
    nav.classList.toggle('scrolled', y > 16);
    const max = document.documentElement.scrollHeight - innerHeight;
    bar.style.transform = `scaleX(${max > 0 ? clamp(y / max, 0, 1) : 0})`;
  }
  addEventListener('scroll', () => {
    if (!scrollQueued) {
      scrollQueued = true;
      requestAnimationFrame(onScroll);
    }
  }, { passive: true });
  onScroll();

  menu.addEventListener('click', () => {
    const open = menu.getAttribute('aria-expanded') !== 'true';
    menu.setAttribute('aria-expanded', String(open));
    links.classList.toggle('open', open);
  });
  links.addEventListener('click', (e) => {
    if (e.target.closest('a')) {
      menu.setAttribute('aria-expanded', 'false');
      links.classList.remove('open');
    }
  });

  // Highlight the section in view.
  const navLinks = $$('.links a');
  const sectionObserver = new IntersectionObserver((entries) => {
    for (const e of entries) {
      if (!e.isIntersecting) continue;
      for (const a of navLinks) a.classList.toggle('active', a.getAttribute('href') === `#${e.target.id}`);
    }
  }, { rootMargin: '-45% 0px -50% 0px' });
  for (const a of navLinks) {
    const s = $(a.getAttribute('href'));
    if (s) sectionObserver.observe(s);
  }

  const pointer = { x: innerWidth / 2, y: innerHeight / 3, active: false };
  let lightQueued = false;
  addEventListener('pointermove', (e) => {
    pointer.x = e.clientX;
    pointer.y = e.clientY;
    pointer.active = true;
    if (!lightQueued) {
      lightQueued = true;
      requestAnimationFrame(() => {
        lightQueued = false;
        document.documentElement.style.setProperty('--mx', `${pointer.x}px`);
        document.documentElement.style.setProperty('--my', `${pointer.y}px`);
      });
    }
  }, { passive: true });
  document.addEventListener('pointerleave', () => (pointer.active = false));

  /* ---------------- Reveal on scroll + count-up ---------------- */

  const groups = new Map();
  for (const el of $$('.reveal')) {
    const list = groups.get(el.parentElement) || [];
    list.push(el);
    groups.set(el.parentElement, list);
  }
  for (const list of groups.values()) list.forEach((el, i) => el.style.setProperty('--d', `${Math.min(i * 0.08, 0.48)}s`));

  const revealObserver = new IntersectionObserver((entries) => {
    for (const e of entries) {
      if (!e.isIntersecting) continue;
      e.target.classList.add('in');
      revealObserver.unobserve(e.target);
      const n = e.target.querySelector('[data-count]');
      if (n) countUp(n);
    }
  }, { rootMargin: '0px 0px -8% 0px', threshold: 0.08 });
  for (const el of $$('.reveal')) revealObserver.observe(el);

  function countUp(el) {
    const to = parseFloat(el.dataset.count);
    const dec = parseInt(el.dataset.decimals || '0', 10);
    const suffix = el.dataset.suffix || '';
    const render = (v) => (el.textContent = `${v.toLocaleString('en-US', { minimumFractionDigits: dec, maximumFractionDigits: dec })}${suffix}`);
    if (reduced || to === 0) return render(to);
    const start = performance.now();
    const dur = 1600;
    const step = (t) => {
      const p = clamp((t - start) / dur, 0, 1);
      render(to * (1 - Math.pow(1 - p, 4)));
      if (p < 1) requestAnimationFrame(step);
    };
    requestAnimationFrame(step);
  }

  /* ---------------- Hero parallax ---------------- */

  const scene = $('[data-tilt-scene]');
  const layers = $$('[data-depth]', scene);
  const heroWindow = $('.hero-window');
  let tiltQueued = false;
  function tilt() {
    tiltQueued = false;
    const r = scene.getBoundingClientRect();
    if (r.bottom < 0 || r.top > innerHeight) return;
    const dx = finePointer && pointer.active ? clamp((pointer.x - (r.left + r.width / 2)) / innerWidth, -0.6, 0.6) : 0;
    const dy = finePointer && pointer.active ? clamp((pointer.y - (r.top + r.height / 2)) / innerHeight, -0.6, 0.6) : 0;
    const sy = clamp(-r.top / innerHeight, -1, 1);
    for (const el of layers) {
      const d = parseFloat(el.dataset.depth);
      const x = -dx * d * 22;
      const y = -dy * d * 18 - sy * d * 46;
      el.style.transform = el === heroWindow ? `translate3d(${x}px, ${y}px, 0) rotateY(${-9 + dx * 10}deg) rotateX(${4 - dy * 8}deg)` : `translate3d(${x}px, ${y}px, 0)`;
    }
  }
  if (!reduced) {
    const queueTilt = () => {
      if (!tiltQueued) {
        tiltQueued = true;
        requestAnimationFrame(tilt);
      }
    };
    addEventListener('pointermove', queueTilt, { passive: true });
    addEventListener('scroll', queueTilt, { passive: true });
  }

  /* ---------------- Magnetic buttons + card spotlight ---------------- */

  if (finePointer && !reduced) {
    for (const b of $$('.magnetic')) {
      b.addEventListener('pointermove', (e) => {
        const r = b.getBoundingClientRect();
        const x = (e.clientX - r.left - r.width / 2) * 0.22;
        const y = (e.clientY - r.top - r.height / 2) * 0.32;
        b.style.transform = `translate(${x}px, ${y}px)`;
      });
      b.addEventListener('pointerleave', () => (b.style.transform = ''));
    }
  }
  document.addEventListener('pointermove', (e) => {
    const card = e.target.closest && e.target.closest('.feature, .card');
    if (!card) return;
    const r = card.getBoundingClientRect();
    card.style.setProperty('--x', `${e.clientX - r.left}px`);
    card.style.setProperty('--y', `${e.clientY - r.top}px`);
  }, { passive: true });

  /* ---------------- Lightbox ---------------- */

  const lb = $('.lightbox');
  const lbImg = $('img', lb);
  let lbReturn = null;
  function openLightbox(img) {
    lbReturn = document.activeElement;
    lbImg.src = img.currentSrc || img.src;
    lbImg.alt = img.alt;
    lb.hidden = false;
    requestAnimationFrame(() => lb.classList.add('open'));
    $('.lb-close', lb).focus();
    document.body.style.overflow = 'hidden';
  }
  function closeLightbox() {
    lb.classList.remove('open');
    document.body.style.overflow = '';
    setTimeout(() => (lb.hidden = true), 300);
    if (lbReturn) lbReturn.focus?.();
  }
  lb.addEventListener('click', closeLightbox);
  addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && !lb.hidden) closeLightbox();
  });
  document.addEventListener('click', (e) => {
    const img = e.target.closest('.window img, .phone:not(.side) img, .landscape-phone img');
    if (img && !img.closest('.lightbox')) openLightbox(img);
  });

  /* ---------------- Feature tour ---------------- */

  const TOUR = [
    { id: 'home', icon: 'home', label: 'Home', title: 'Home', heading: 'Everything at a glance', text: "Your drives, what's playing, the busiest programs and one-click actions — the moment you open OmniHub. Press Ctrl K to jump anywhere.", tags: ['Now playing', 'Drives', 'Top apps', 'Quick actions'] },
    { id: 'storage', icon: 'drive', label: 'Storage', title: 'Storage', heading: 'See what fills your drives — in seconds', text: "OmniHub reads the NTFS file table directly, like WizTree and Everything, and keeps it current from Windows' change journal. Treemap, largest files, file types, what grew, duplicates and cleanup.", tags: ['1.15M files in 3.3 s', 'Treemap', 'Duplicates', 'Cleanup'] },
    { id: 'apps', icon: 'apps', label: 'Apps', title: 'Apps', heading: 'Every program, with its real size', text: 'Desktop programs, Store apps and Start menu entries with their icons and the size from your last scan. Launch, uninstall, manage startup apps, or see the screenshots you took in each.', tags: ['Store apps', 'Uninstall', 'Startup apps', 'Favourites'] },
    { id: 'tasks', icon: 'tasks', label: 'Tasks', title: 'Tasks', heading: 'What each program really uses', text: 'CPU, memory, GPU, video memory and disk for every program, live — with End task, priority and file location. Same view on your phone.', tags: ['GPU per app', 'Video memory', 'Priority', 'End task'] },
    { id: 'games', icon: 'game', label: 'Games', title: 'Games', heading: 'Max FPS where it counts — and every frame measured', text: 'Competitive mode sets Fortnite, VALORANT, CS2, Apex, Overwatch 2, Roblox or Minecraft to their fastest settings, closes background junk, gives the game the processor and a 0.5 ms timer; Quality mode does the same for Windows but never touches the game’s graphics. The FPS meter shows live FPS and 1% lows — and everything comes back when you quit.', tags: ['Competitive & Quality', '7 games tuned', 'FPS meter', 'Lag under load'] },
    { id: 'aurora', icon: 'music', label: 'Aurora', title: 'Music — Aurora', heading: 'Aurora: the song becomes the show', text: 'The playing song’s cover, big and alive on black — swelling with the bass and glitching on the beats — inside a thin neon light in its colours, with huge lyrics one word at a time. It follows the real sound through Windows’ loopback, never the microphone.', tags: ['Cover visual', 'Neon edge light', 'Word by word', 'Follows the beat'] },
    { id: 'music', icon: 'music', label: 'Lyrics', title: 'Music — Lyrics', heading: 'Or lyrics, Apple Music style', text: 'Prefer reading along? Switch to Lyrics: the cover art blurred into drifting light, big lyrics that glide up line by line and fill word by word, and three dots through the breaks. Click any line to jump there.', tags: ['Aurora or Lyrics', 'Word by word', 'Click to seek', 'Spotify & more'] },
    { id: 'vault', icon: 'lock', label: 'Vault', title: 'Vault', heading: 'Passwords, properly locked', text: 'Argon2id and AES-256-GCM, bound to your Windows account. Two-factor codes, a password health report, an optional breach check — and autofill in Brave, Chrome and Edge.', tags: ['Windows Hello', '2FA codes', 'Autofill', 'Breach check'] },
    { id: 'notes', icon: 'idea', label: 'Notes', title: 'Notes', heading: 'Notes, and ideas for Claude', text: 'Markdown notes with templates and reminders. Ideas are written as .md files into a folder Claude can read — and replies Claude writes there show up next to your note.', tags: ['Markdown', 'Templates', 'Reminders', 'Claude folder'] },
    { id: 'screenshots', icon: 'camera', label: 'Screenshots', title: 'Screenshots', heading: 'Capture it, mark it up, find it again', text: 'Region, window or every screen with global hotkeys. A searchable library with tags and favourites, markup and text recognition — saved as normal files in your Pictures.', tags: ['Hotkeys', 'Markup', 'Text recognition', 'Tags'] },
    { id: 'screen', icon: 'screen', label: 'Screen share', title: 'Screen share', heading: 'Your PC on your phone — and back', text: 'Watch the PC from any phone browser; pause or share a single window for privacy. Mirror an iPhone over AirPlay or an Android with scrcpy, or go 4K60 with Sunshine + Moonlight.', tags: ['AirPlay', 'scrcpy', 'Sunshine', 'One-window share'] },
    { id: 'phone', icon: 'phone', label: 'Phone', title: 'Phone', heading: 'You decide what the phone may do', text: 'Pair with a QR code or PIN. Then switch on exactly what paired phones may do — folders, files, power, screen, remote control, vault — with every action in an activity log.', tags: ['QR + PIN', 'Per-phone access', 'Activity log', 'Wi-Fi only'] },
  ];

  const tour = $('.tour');
  const tabs = $('.tour-tabs');
  const images = $('.tour-images');
  const caption = $('.tour-caption');
  const tourWindow = $('.tour-window');
  const TOUR_MS = 6500;
  tour.style.setProperty('--tour-ms', `${TOUR_MS}ms`);
  let tourIndex = 0;

  TOUR.forEach((t, i) => {
    const b = document.createElement('button');
    b.className = 'tour-tab';
    b.type = 'button';
    b.setAttribute('role', 'tab');
    b.id = `tab-${t.id}`;
    b.setAttribute('aria-selected', 'false');
    b.setAttribute('tabindex', '-1');
    b.innerHTML = `${svg(ICONS[t.icon])}<span>${t.label}</span><i class="bar"></i>`;
    b.addEventListener('click', () => selectTour(i, true));
    tabs.append(b);
    const img = document.createElement('img');
    img.alt = `OmniHub's ${t.label} page`;
    img.width = 2880;
    img.height = 1800;
    img.decoding = 'async';
    img.dataset.src = `assets/shots/desktop-${t.id}.webp`;
    images.append(img);
    $('.bar', b).addEventListener('animationend', () => {
      if (!reduced) selectTour((i + 1) % TOUR.length, false);
    });
  });

  const tabButtons = $$('.tour-tab', tabs);
  const tabImages = $$('img', images);
  const load = (img) => {
    if (!img.src) img.src = img.dataset.src;
  };

  function selectTour(i, user) {
    tourIndex = i;
    const t = TOUR[i];
    tabButtons.forEach((b, j) => {
      const on = j === i;
      b.setAttribute('aria-selected', String(on));
      b.setAttribute('tabindex', on ? '0' : '-1');
      const barEl = $('.bar', b);
      barEl.style.animation = 'none';
      void barEl.offsetWidth;
      barEl.style.animation = '';
    });
    load(tabImages[i]);
    load(tabImages[(i + 1) % TOUR.length]);
    tabImages.forEach((img, j) => img.classList.toggle('on', j === i));
    if (user) {
      tabButtons[i].focus({ preventScroll: true });
    }
    // Keep the active tab visible when the row scrolls (phones).
    const b = tabButtons[i];
    if (tabs.scrollWidth > tabs.clientWidth) tabs.scrollTo({ left: b.offsetLeft - tabs.clientWidth / 2 + b.offsetWidth / 2, behavior: reduced ? 'auto' : 'smooth' });
    caption.classList.add('swap');
    setTimeout(() => {
      $('[data-tour-title]').textContent = `OmniHub — ${t.title}`;
      $('[data-tour-heading]').textContent = t.heading;
      $('[data-tour-text]').textContent = t.text;
      $('[data-tour-tags]').innerHTML = t.tags.map((x) => `<li>${x}</li>`).join('');
      caption.classList.remove('swap');
    }, reduced ? 0 : 220);
  }

  tabs.addEventListener('keydown', (e) => {
    const k = e.key;
    if (k !== 'ArrowRight' && k !== 'ArrowLeft' && k !== 'Home' && k !== 'End') return;
    e.preventDefault();
    const n = TOUR.length;
    const i = k === 'Home' ? 0 : k === 'End' ? n - 1 : (tourIndex + (k === 'ArrowRight' ? 1 : -1) + n) % n;
    selectTour(i, true);
  });

  // Pause the auto-advance while the tour is off screen, hovered or focused.
  let tourVisible = false;
  let tourHover = false;
  const syncTourPause = () => tour.classList.toggle('paused', !tourVisible || tourHover || reduced || document.hidden);
  new IntersectionObserver(([e]) => {
    tourVisible = e.isIntersecting;
    syncTourPause();
  }, { threshold: 0.35 }).observe(tour);
  $('.tour-stage').addEventListener('pointerenter', () => {
    tourHover = true;
    syncTourPause();
  });
  $('.tour-stage').addEventListener('pointerleave', () => {
    tourHover = false;
    syncTourPause();
    tourWindow.style.setProperty('--rx', '0deg');
    tourWindow.style.setProperty('--ry', '0deg');
  });
  document.addEventListener('visibilitychange', syncTourPause);
  if (finePointer && !reduced) {
    tourWindow.addEventListener('pointermove', (e) => {
      const r = tourWindow.getBoundingClientRect();
      tourWindow.style.setProperty('--ry', `${((e.clientX - r.left) / r.width - 0.5) * 5}deg`);
      tourWindow.style.setProperty('--rx', `${-((e.clientY - r.top) / r.height - 0.5) * 4}deg`);
    });
  }
  selectTour(0, false);
  syncTourPause();
  // Fetch the rest once the page has settled, so tab switches are instant.
  addEventListener('load', () => setTimeout(() => tabImages.forEach(load), 1200));

  /* ---------------- Everything grid ---------------- */

  const FEATURES = [
    ['drive', 'Storage in seconds', 'Reads the NTFS file table directly, then stays current from the change journal. No more waiting for a folder walk.'],
    ['treemap', 'Treemap, duplicates, cleanup', 'Spot the big folders, find identical files, and clear caches and leftovers with a preview of what goes.'],
    ['grew', 'What grew', 'See which folders grew since last time — the fastest way to find what ate your free space.'],
    ['apps', 'Apps with real sizes', 'Every desktop and Store app with its size. Uninstall, launch, and turn off startup apps.'],
    ['tasks', 'GPU per program', 'CPU, memory, GPU, video memory and disk for every program — End task and priority included.'],
    ['game', 'Game boost', 'Power plan, background apps, notifications, GPU, Wi-Fi and priority — tuned for the game, then put back.'],
    ['game', 'Competitive or Quality', 'Competitive goes for every frame, game graphics included. Quality leaves the game’s graphics alone — 4K Ultra stays 4K Ultra.'],
    ['game', 'Max FPS in 7 games', 'Fortnite, VALORANT, CS2, Apex, Overwatch 2, Roblox and Minecraft set for frames — written into their own settings, with yours kept to put back.'],
    ['tasks', 'FPS meter', 'Live FPS, 1% lows and stutters while you play, on the PC and your phone — measured with Intel PresentMon, results kept per game.'],
    ['send', 'Lag under load', 'Pings while the line is busy, grades it A+ to F and tells you what fixes it — usually the router, a cable, or a paused download.'],
    ['tasks', 'Optimize this PC', 'Monitor at its full refresh rate, Ultimate power plan, no background game recording, GPU scheduling and more — each one checked and undoable.'],
    ['apps', 'Finds your games', 'Epic, Steam, Riot, Roblox and Minecraft games on this PC, ready to add with one click.'],
    ['music', 'Aurora', 'The song’s cover as a music-reactive visual, a neon edge light in its colours, and huge lyrics word by word — full screen or on the Music page.'],
    ['screen', 'Glow around your screen', 'The same neon light around your monitor over every app — click-through, and out of the way of full-screen games.'],
    ['music', 'Full-screen lyrics', 'Apple Music–style lyrics on the PC: drifting cover-art light, lines that glide up and fill word by word.'],
    ['music', 'Music with lyrics', 'Spotify, Apple Music or a browser — controls, cover art, volume, bass and synced lyrics on your phone, upright or sideways.'],
    ['volume', 'Volume mixer', 'Every app’s volume and mute from your phone — turn the game down, Spotify up.'],
    ['mic', 'Call controls', 'In a Discord, WhatsApp or Nyxen call? Mute your mic or deafen from the couch.'],
    ['close', 'Close any app', 'Close a program like clicking ×, or quit the ones that hide in the tray.'],
    ['screen', 'Screen share', 'PC to phone in any browser, iPhone to PC over AirPlay, Android with scrcpy, 4K60 with Sunshine.'],
    ['send', 'Files both ways', 'Big uploads that resume, downloads with resume, and "Send to phone" from anywhere on the PC.'],
    ['power', 'Power from the couch', 'Lock, sleep, restart or shut down from your phone — with a countdown anyone can cancel.'],
    ['lock', 'Encrypted vault', 'Passwords, Wi-Fi keys and notes with Windows Hello unlock and a clipboard that clears itself.'],
    ['key', 'Browser autofill', 'A small extension for Brave, Chrome and Edge that talks only to OmniHub on your PC.'],
    ['camera', 'Screenshots + text', 'Capture with hotkeys, mark up, and copy the text out of any screenshot.'],
    ['idea', 'Ideas for Claude', 'Write ideas into a folder Claude reads; its replies appear right next to your note.'],
    ['command', 'Jump anywhere', 'Ctrl K opens a command bar for every page, drive, app and action.'],
    ['shield', 'Approve scans once', 'One Windows prompt for fast drive scans — not one every time.'],
    ['update', 'Updates itself', 'New versions install from inside the app — checked against their published SHA-256 first.'],
  ];
  const grid = $('[data-features]');
  grid.innerHTML = FEATURES.map(([icon, title, text]) => `<article class="feature reveal"><div class="f-icon">${svg(ICONS[icon], 22)}</div><h3>${title}</h3><p>${text}</p></article>`).join('');
  $$('.feature', grid).forEach((el, i) => {
    el.style.setProperty('--d', `${(i % 4) * 0.07}s`);
    revealObserver.observe(el);
  });

  /* ---------------- Phone carousel ---------------- */

  const PHONES = [
    ['phone-home', 'Home', 'CPU, memory, drives and what’s playing, live.'],
    ['phone-music', 'Music', 'Cover art and controls for whatever plays on the PC.'],
    ['phone-lyrics', 'Lyrics', 'Full-screen lyrics in time — tap a line to jump there.'],
    ['phone-sound', 'Volume & calls', 'Every app’s volume, mute your mic, Mute mic and Deafen for a Discord call.'],
    ['phone-open-apps', 'Open apps', 'Close any program on the PC — or quit it.'],
    ['phone-files', 'Files', 'Browse folders and photos, download or send to the PC.'],
    ['phone-games', 'Games', 'Start a game on the PC with its boost, watch its FPS, or test your ping.'],
    ['phone-power', 'Power', 'Lock, sleep or shut down, with a countdown you can cancel.'],
  ];
  const carousel = $('[data-carousel]');
  const track = $('.phones-track', carousel);
  const dots = $('.dots', carousel);
  const capEl = $('.phones-caption', carousel);
  let current = 0;
  PHONES.forEach(([file, name], i) => {
    const f = document.createElement('figure');
    f.className = 'phone';
    f.innerHTML = `<img src="assets/shots/${file}.webp" alt="Phone: ${name}" width="900" height="1948" loading="lazy" decoding="async" />`;
    f.addEventListener('click', (e) => {
      if (i !== current) {
        e.stopPropagation();
        go(i);
      }
    });
    track.append(f);
    const d = document.createElement('button');
    d.setAttribute('role', 'tab');
    d.setAttribute('aria-label', name);
    d.addEventListener('click', () => go(i));
    dots.append(d);
  });
  const phoneEls = $$('.phone', track);
  const dotEls = $$('button', dots);

  function layoutPhones() {
    const n = PHONES.length;
    phoneEls.forEach((el, i) => {
      let o = i - current;
      if (o > n / 2) o -= n;
      if (o < -n / 2) o += n;
      const a = Math.abs(o);
      el.classList.toggle('center', o === 0);
      el.classList.toggle('side', o !== 0);
      el.style.zIndex = String(10 - a);
      el.style.opacity = a > 2 ? '0' : a === 2 ? '0.35' : '1';
      el.style.filter = o === 0 ? 'none' : `brightness(${a === 1 ? 0.55 : 0.35})`;
      el.style.transform = `translateX(${o * 64}%) translateZ(${-a * 160}px) rotateY(${-o * 22}deg) scale(${1 - a * 0.08})`;
      el.style.pointerEvents = a > 1 ? 'none' : 'auto';
    });
    dotEls.forEach((d, i) => d.setAttribute('aria-selected', String(i === current)));
    const [, name, text] = PHONES[current];
    capEl.innerHTML = `<b>${name}</b> — ${text}`;
  }
  function go(i) {
    current = (i + PHONES.length) % PHONES.length;
    layoutPhones();
    restartAuto();
  }
  $('.prev', carousel).addEventListener('click', () => go(current - 1));
  $('.next', carousel).addEventListener('click', () => go(current + 1));
  carousel.addEventListener('keydown', (e) => {
    if (e.key === 'ArrowLeft') go(current - 1);
    if (e.key === 'ArrowRight') go(current + 1);
  });

  // Swipe.
  let downX = null;
  let swiped = false;
  track.addEventListener('pointerdown', (e) => {
    downX = e.clientX;
    swiped = false;
  });
  addEventListener('pointerup', (e) => {
    if (downX === null) return;
    const dx = e.clientX - downX;
    downX = null;
    if (Math.abs(dx) > 40) {
      swiped = true;
      go(current + (dx < 0 ? 1 : -1));
    }
  });
  addEventListener('pointercancel', () => (downX = null));
  track.addEventListener('click', (e) => {
    if (swiped) {
      swiped = false;
      e.stopPropagation();
      e.preventDefault();
    }
  }, true);

  let autoTimer = 0;
  let carouselVisible = false;
  let carouselHover = false;
  function restartAuto() {
    clearInterval(autoTimer);
    if (reduced) return;
    autoTimer = setInterval(() => {
      if (carouselVisible && !carouselHover && !document.hidden) {
        current = (current + 1) % PHONES.length;
        layoutPhones();
      }
    }, 3800);
  }
  carousel.addEventListener('pointerenter', () => (carouselHover = true));
  carousel.addEventListener('pointerleave', () => (carouselHover = false));
  new IntersectionObserver(([e]) => (carouselVisible = e.isIntersecting), { threshold: 0.4 }).observe(carousel);
  layoutPhones();
  restartAuto();

  /* ---------------- Aurora demo ---------------- */

  const aurora = $('[data-aurora]');
  if (aurora) {
    const stage = $('[data-aurora-stage]', aurora);
    const coverBox = $('[data-aurora-cover]', aurora);
    const wordsBox = $('[data-aurora-words]', aurora);
    const playBtn = $('[data-aurora-play]', aurora);
    // One colour channel each, recombined with "screen" so they can split apart.
    const m = (r, g, b) => `<feColorMatrix values="${r} 0 0 0 0  0 ${g} 0 0 0  0 0 ${b} 0 0  0 0 0 1 0"/>`;
    document.body.insertAdjacentHTML('beforeend', `<svg width="0" height="0" style="position:absolute" aria-hidden="true"><filter id="aurora-r">${m(1, 0, 0)}</filter><filter id="aurora-g">${m(0, 1, 0)}</filter><filter id="aurora-b">${m(0, 0, 1)}</filter></svg>`);

    // Made-up covers for the demo, neon on black.
    const rays = (n, colors, inner, outer, width) =>
      Array.from({ length: n }, (_, k) => {
        const a = (k / n) * Math.PI * 2;
        const len = outer * (0.75 + 0.25 * Math.sin(k * 2.3));
        return `<line x1="${150 + Math.cos(a) * inner}" y1="${150 + Math.sin(a) * inner}" x2="${150 + Math.cos(a) * len}" y2="${150 + Math.sin(a) * len}" stroke="${colors[k % colors.length]}" stroke-width="${width}" stroke-linecap="round"/>`;
      }).join('');
    const COVERS = [
      { name: 'Sunburst', c1: '#ff4fa3', c2: '#ff9a3c', body: `${rays(22, ['#ff5ea8', '#ff9a3c', '#ffe14d', '#c13cff'], 34, 150, 9)}<circle cx="150" cy="150" r="46" fill="#ff7a3c"/><circle cx="150" cy="150" r="20" fill="#120616"/>` },
      { name: 'Rings', c1: '#a855f7', c2: '#3b82f6', body: [0, 1, 2, 3, 4].map((k) => `<circle cx="${150 + k * 6}" cy="${150 - k * 4}" r="${28 + k * 22}" fill="none" stroke="${['#3b82f6', '#22d3ee', '#a855f7', '#ec4899', '#60a5fa'][k]}" stroke-width="${7 - k}"/>`).join('') + '<circle cx="150" cy="150" r="18" fill="#465aff"/>' },
      { name: 'Petals', c1: '#22d3a0', c2: '#a3e635', body: Array.from({ length: 8 }, (_, k) => `<ellipse cx="150" cy="88" rx="22" ry="62" fill="${['#22c79a', '#a3e635', '#22d3ee', '#10b981'][k % 4]}" opacity=".85" transform="rotate(${k * 45} 150 150)"/>`).join('') + '<circle cx="150" cy="150" r="26" fill="#fde047"/>' },
    ].map((c) => ({ ...c, url: `data:image/svg+xml,${encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 300 300"><rect width="300" height="300" fill="#050307"/>${c.body}</svg>`)}` }));

    const art = document.createElement('div');
    art.className = 'art';
    art.innerHTML = '<div class="layer r"></div><div class="layer g"></div><div class="layer b"></div>' + '<div class="slice"></div>'.repeat(5);
    coverBox.append(art);
    const layers = $$('.layer', art);
    const slices = $$('.slice', art);
    const pickers = $('[data-aurora-covers]', aurora);
    let cover = COVERS[0];
    function setCover(c) {
      cover = c;
      layers.forEach((l) => (l.style.backgroundImage = `url("${c.url}")`));
      slices.forEach((s) => (s.style.backgroundImage = `url("${c.url}")`));
      stage.style.setProperty('--c1', c.c1);
      stage.style.setProperty('--c2', c.c2);
      $$('button', pickers).forEach((b) => b.setAttribute('aria-checked', String(b.dataset.v === c.name)));
    }
    pickers.innerHTML = COVERS.map((c) => `<button role="radio" aria-checked="false" data-v="${c.name}"><img src="${c.url}" alt="" />${c.name}</button>`).join('');
    pickers.addEventListener('click', (e) => {
      const b = e.target.closest('button');
      const c = b && COVERS.find((x) => x.name === b.dataset.v);
      if (c) setCover(c);
    });
    for (const [attr, key] of [['data-aurora-style', 'style'], ['data-aurora-emphasis', 'emphasis']]) {
      const group = $(`[${attr}]`, aurora);
      group.addEventListener('click', (e) => {
        const b = e.target.closest('button');
        if (!b) return;
        $$('button', group).forEach((x) => x.setAttribute('aria-checked', String(x === b)));
        stage.dataset[key] = b.dataset.v;
      });
    }
    stage.dataset.style = 'visual';
    stage.dataset.emphasis = 'glow';
    setCover(cover);

    // A made-up song, word by word (half a beat each, a beat between lines).
    const BPM = 112;
    const BEAT = 60 / BPM;
    const LINES = ['Turn it up, the night is ours to keep', 'Echoes on the highway, we don’t sleep', 'Hold the moment, let the chorus fall', 'Daylight’s coming, but we’ve got it all'];
    const WORDS = [];
    let at = BEAT * 2;
    for (const line of LINES) {
      for (const w of line.split(' ')) {
        WORDS.push({ t: at, w });
        at += BEAT / 2;
      }
      at += BEAT;
    }
    const LOOP = at + BEAT;
    let shown = -2;
    function showWord(i) {
      if (i === shown) return;
      shown = i;
      const esc = (s) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;');
      const prev = WORDS[i - 1];
      const next = WORDS[i + 1];
      wordsBox.innerHTML = i < 0 ? '' : `${prev ? `<span class="prev">${esc(prev.w)}</span>` : ''}<span class="cur${reduced ? '' : ' enter'}">${esc(WORDS[i].w)}</span>${next ? `<span class="next">${esc(next.w)}</span>` : ''}`;
    }

    let playing = false;
    let start = 0;
    let lastBeat = -1;
    let glitchUntil = 0;
    function glitch(now) {
      const h = art.clientHeight || 1;
      slices.forEach((s) => {
        if (Math.random() < 0.55) {
          s.style.display = 'none';
          return;
        }
        const top = Math.random() * 0.85;
        const height = 0.03 + Math.random() * 0.12;
        const shift = (Math.random() - 0.5) * h * 0.18;
        Object.assign(s.style, { display: 'block', top: `${top * 100}%`, height: `${height * 100}%`, backgroundPosition: `${shift}px ${-top * h}px`, transform: `translateX(${shift * 0.3}px)` });
      });
      glitchUntil = now + 90 + Math.random() * 90;
    }
    function frame(now) {
      if (!playing) return;
      const t = ((now - start) / 1000) % LOOP;
      const phase = (t % BEAT) / BEAT;
      const beatN = Math.floor(t / BEAT);
      const kick = reduced ? 0 : Math.exp(-phase * 6);
      stage.style.setProperty('--kick', kick.toFixed(3));
      stage.style.setProperty('--split', `${(2 + kick * 7).toFixed(1)}px`);
      stage.style.setProperty('--a', `${((now / 60) % 360).toFixed(1)}deg`);
      if (beatN !== lastBeat) {
        lastBeat = beatN;
        if (!reduced && stage.dataset.style === 'visual' && Math.random() < 0.6) glitch(now);
      }
      if (now > glitchUntil) slices.forEach((s) => (s.style.display = 'none'));
      let i = -1;
      for (let k = 0; k < WORDS.length && WORDS[k].t <= t; k++) i = k;
      showWord(i);
      requestAnimationFrame(frame);
    }
    function play(on) {
      if (on === playing) return;
      playing = on;
      stage.classList.toggle('playing', on);
      playBtn.textContent = on ? '❚❚' : '▶';
      playBtn.setAttribute('aria-label', on ? 'Pause' : 'Play');
      if (on) {
        start = performance.now() - Math.max(0, WORDS[Math.max(0, shown)]?.t ?? 0) * 1000;
        requestAnimationFrame(frame);
      }
    }
    playBtn.addEventListener('click', () => play(!playing));
    showWord(0);
    if (!reduced) {
      new IntersectionObserver((entries) => entries.forEach((e) => play(e.isIntersecting)), { threshold: 0.35 }).observe(stage);
    }
  }

  /* ---------------- Lyrics demo ---------------- */

  const LYRICS = [
    'City lights are fading into blue',
    'I keep the engine running just for you',
    'Every mile a little closer to the sun',
    'We were never made to be the only one',
    'Turn it up, the night is ours to keep',
    "Echoes on the highway, we don't sleep",
    'Hold the moment, let the chorus fall',
    "Daylight's coming, but we've got it all",
    'Paper planes above the parking lot',
    'Every promise that we never bought',
    'Radio is singing what we mean',
    'Somewhere in the static, in between',
  ];
  const LINE_MS = 4000;
  const FIRST_MS = 600;
  const DUR_MS = FIRST_MS + LYRICS.length * LINE_MS;
  const player = $('[data-player]');
  const lyricsBox = $('[data-lyrics]', player);
  const inner = document.createElement('div');
  inner.className = 'lyrics-inner';
  inner.setAttribute('role', 'list');
  inner.innerHTML = LYRICS.map((l, i) => `<button class="lyric" role="listitem" data-i="${i}">${l}</button>`).join('');
  lyricsBox.append(inner);
  const lines = $$('.lyric', inner);
  const playBtn = $('[data-play]', player);
  const timeEl = $('[data-time]', player);
  const durEl = $('[data-dur]', player);
  const trackBar = $('[data-track]', player);
  const fill = $('.fill', trackBar);
  const knob = $('.knob', trackBar);
  const fmt = (ms) => {
    const s = Math.floor(ms / 1000);
    return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
  };
  durEl.textContent = fmt(DUR_MS);
  let pos = 0;
  let playing = !reduced;
  let playerVisible = false;
  let last = performance.now();
  let lastLine = -2;

  function renderPlayer() {
    const pct = (pos / DUR_MS) * 100;
    fill.style.width = `${pct}%`;
    knob.style.left = `${pct}%`;
    timeEl.textContent = fmt(pos);
    const idx = pos < FIRST_MS ? -1 : Math.min(LYRICS.length - 1, Math.floor((pos - FIRST_MS) / LINE_MS));
    if (idx !== lastLine) {
      lastLine = idx;
      lines.forEach((l, i) => {
        l.classList.toggle('now', i === idx);
        l.classList.toggle('past', i < idx);
      });
      const target = lines[Math.max(0, idx)];
      const offset = target.offsetTop - lyricsBox.clientHeight * 0.36;
      inner.style.transform = `translateY(${-offset}px)`;
    }
  }
  function setPlaying(p) {
    playing = p;
    player.classList.toggle('paused', !p);
    playBtn.textContent = p ? '❚❚' : '▶';
    playBtn.setAttribute('aria-label', p ? 'Pause' : 'Play');
    last = performance.now();
  }
  function tick(t) {
    const dt = t - last;
    last = t;
    if (playing && playerVisible && !document.hidden) {
      pos += Math.min(dt, 100);
      if (pos >= DUR_MS) pos = 0;
      renderPlayer();
    }
    requestAnimationFrame(tick);
  }
  playBtn.addEventListener('click', () => setPlaying(!playing));
  inner.addEventListener('click', (e) => {
    const l = e.target.closest('.lyric');
    if (!l) return;
    pos = FIRST_MS + Number(l.dataset.i) * LINE_MS + 1;
    renderPlayer();
    setPlaying(true);
  });
  const seekTo = (clientX) => {
    const r = trackBar.getBoundingClientRect();
    pos = clamp((clientX - r.left) / r.width, 0, 1) * (DUR_MS - 1);
    renderPlayer();
  };
  trackBar.addEventListener('pointerdown', (e) => {
    trackBar.setPointerCapture(e.pointerId);
    seekTo(e.clientX);
  });
  trackBar.addEventListener('pointermove', (e) => {
    if (trackBar.hasPointerCapture(e.pointerId)) seekTo(e.clientX);
  });
  new IntersectionObserver(([e]) => (playerVisible = e.isIntersecting), { threshold: 0.25 }).observe(player);
  setPlaying(playing);
  renderPlayer();
  addEventListener('resize', () => {
    lastLine = -2;
    renderPlayer();
  });
  requestAnimationFrame(tick);

  /* ---------------- Game boost demo ---------------- */

  const boostBtn = $('[data-boost]');
  const boostStatus = $('[data-boost-status]');
  const stepsEl = $('[data-steps]');
  const fpsEl = $('[data-fps]');
  const fpsLine = $('[data-fps-line]');
  let steps = [];
  let boosted = false;
  let boosting = false;
  const wait = (ms) => new Promise((r) => setTimeout(r, reduced ? 0 : ms));

  // What each mode does (the app's Boost::with_mode), for a fast game and a pretty one.
  const MODES = {
    competitive: { game: 'Fortnite', tile: 'FN', fps: 470, steps: ['Fortnite: unlimited FPS, Performance mode, lowest settings', 'Close OneDrive, Widgets and updaters (reopened later)', 'Browsers and launchers to Below normal', 'Ultimate Performance power plan', 'Precise 0.5 ms timer', 'Wi-Fi low-latency mode', 'Game priority: High, full speed', 'Start Fortnite — FPS meter on'] },
    quality: { game: 'Cyberpunk 2077', tile: '77', fps: 88, steps: ['Game graphics untouched — 4K Ultra stays', 'Close OneDrive, Widgets and updaters (reopened later)', 'Browsers and launchers to Below normal', 'Ultimate Performance power plan', 'Precise 0.5 ms timer', 'Pause notification pop-ups', 'Game priority: Above normal, full speed', 'Start Cyberpunk 2077 — FPS meter on'] },
  };
  let mode = 'competitive';
  let fpsTimer = 0;
  const trail = [];
  function renderMode() {
    const m = MODES[mode];
    $('[data-boost-game]').textContent = m.game;
    $('[data-boost-tile]').textContent = m.tile;
    stepsEl.innerHTML = m.steps.map((s) => `<li>${s}</li>`).join('');
    steps = $$('li', stepsEl);
    $$('[data-mode]').forEach((b) => b.setAttribute('aria-selected', String(b.dataset.mode === mode)));
  }
  function startFps() {
    const m = MODES[mode];
    trail.length = 0;
    fpsEl.hidden = false;
    const tickFps = () => {
      const fps = m.fps * (0.95 + Math.random() * 0.08);
      trail.push(fps);
      if (trail.length > 24) trail.shift();
      $('[data-fps-now]').textContent = Math.round(fps);
      $('[data-fps-low]').textContent = Math.round(fps * (0.7 + Math.random() * 0.05));
      $('[data-fps-ms]').textContent = `${(1000 / fps).toFixed(2)} ms`;
      const max = Math.max(...trail) * 1.05;
      const min = Math.min(...trail) * 0.95;
      fpsLine.setAttribute('points', trail.map((v, i) => `${(i / Math.max(1, trail.length - 1)) * 100},${30 - ((v - min) / Math.max(1, max - min)) * 28 - 1}`).join(' '));
    };
    tickFps();
    fpsTimer = setInterval(tickFps, reduced ? 2000 : 900);
  }
  function stopFps() {
    clearInterval(fpsTimer);
    fpsEl.hidden = true;
  }
  $$('[data-mode]').forEach((b) =>
    b.addEventListener('click', () => {
      if (boosting || b.dataset.mode === mode) return;
      if (boosted) {
        boosted = false;
        stopFps();
        boostBtn.textContent = 'Play with boost';
        boostStatus.classList.remove('live');
      }
      mode = b.dataset.mode;
      boostStatus.textContent = mode === 'quality' ? 'Ready — graphics stay as you set them' : 'Ready';
      renderMode();
    }),
  );
  renderMode();

  async function runBoost() {
    if (boosting) return;
    boosting = true;
    boostBtn.disabled = true;
    if (!boosted) {
      boostStatus.textContent = 'Boosting…';
      boostStatus.classList.remove('live');
      for (const s of steps) {
        s.classList.add('doing');
        await wait(420);
        s.classList.remove('doing');
        s.classList.add('done');
      }
      boostStatus.textContent = 'Playing — boosted';
      boostStatus.classList.add('live');
      boostBtn.textContent = 'Stop boost';
      boosted = true;
      startFps();
    } else {
      stopFps();
      boostStatus.textContent = 'Putting everything back…';
      boostStatus.classList.remove('live');
      for (const s of [...steps].reverse()) {
        await wait(140);
        s.classList.remove('done');
      }
      boostStatus.textContent = 'Ready — your settings are back';
      boostBtn.textContent = 'Play with boost';
      boosted = false;
    }
    boostBtn.disabled = false;
    boosting = false;
  }
  boostBtn.addEventListener('click', runBoost);
  const boostCard = boostBtn.closest('.card');
  const boostObserver = new IntersectionObserver(([e]) => {
    if (!e.isIntersecting) return;
    boostObserver.disconnect();
    setTimeout(() => {
      if (!boosted && !boosting) runBoost();
    }, 700);
  }, { threshold: 0.6 });
  boostObserver.observe(boostCard);

  /* ---------------- Ping demo ---------------- */

  const REGIONS = [
    ['NA-East', 28],
    ['NA-Central', 44],
    ['NA-West', 71],
    ['Europe', 98],
    ['Brazil', 132],
    ['Middle East', 158],
    ['Asia', 186],
    ['Oceania', 212],
  ];
  const pingRows = $('[data-ping-rows]');
  const pingBtn = $('[data-ping]');
  pingRows.innerHTML = REGIONS.map(([name]) => `<div class="ping-row"><span class="name">${name}</span><span class="meter"><i></i></span><span class="ms">—</span></div>`).join('');
  const rowEls = $$('.ping-row', pingRows);
  const pingColor = (ms) => (ms < 60 ? 'var(--green)' : ms < 120 ? 'var(--amber)' : 'var(--red)');
  let pinging = false;
  async function runPing() {
    if (pinging) return;
    pinging = true;
    pingBtn.disabled = true;
    pingBtn.textContent = 'Testing…';
    rowEls.forEach((r) => {
      r.classList.remove('best');
      $('i', r).style.width = '0';
      $('.ms', r).textContent = '…';
    });
    const results = REGIONS.map(([, base]) => Math.round(base + (Math.random() - 0.5) * base * 0.18));
    const best = results.indexOf(Math.min(...results));
    for (let i = 0; i < rowEls.length; i++) {
      await wait(180);
      const ms = results[i];
      const fillEl = $('i', rowEls[i]);
      fillEl.style.background = pingColor(ms);
      fillEl.style.width = `${clamp(ms / 240, 0.06, 1) * 100}%`;
      $('.ms', rowEls[i]).textContent = `${ms} ms`;
    }
    rowEls[best].classList.add('best');
    await wait(500);
    const extra = Math.round(38 + Math.random() * 14);
    $('[data-bloat-grade]').textContent = extra < 30 ? 'A' : extra < 60 ? 'B' : 'C';
    $('[data-bloat-text]').textContent = `+${extra} ms while downloading`;
    $('[data-bloat]').hidden = false;
    const sub = pingBtn.closest('.card-head').querySelector('span');
    sub.textContent = 'Sample numbers — the app measures from your PC';
    pingBtn.textContent = 'Test again';
    pingBtn.disabled = false;
    pinging = false;
  }
  pingBtn.addEventListener('click', runPing);
  const pingObserver = new IntersectionObserver(([e]) => {
    if (!e.isIntersecting) return;
    pingObserver.disconnect();
    setTimeout(runPing, 500);
  }, { threshold: 0.6 });
  pingObserver.observe(pingRows);

  /* ---------------- Roblox flags demo ---------------- */

  // The same presets the app writes (crates/omnihub-core/src/games/roblox.rs).
  const PRESETS = {
    maxFps: {
      FFlagDebugGraphicsPreferD3D11: 'True',
      FIntDebugForceMSAASamples: '0',
      DFFlagTextureQualityOverrideEnabled: 'True',
      DFIntTextureQualityOverride: '0',
      FIntFRMMinGrassDistance: '0',
      FIntFRMMaxGrassDistance: '0',
      DFIntCSGLevelOfDetailSwitchingDistance: '0',
      DFIntCSGLevelOfDetailSwitchingDistanceL12: '0',
      DFIntCSGLevelOfDetailSwitchingDistanceL23: '0',
      DFIntCSGLevelOfDetailSwitchingDistanceL34: '0',
      DFIntDebugFRMQualityLevelOverride: '1',
      FFlagHandleAltEnterFullscreenManually: 'False',
    },
    balanced: {
      FFlagDebugGraphicsPreferD3D11: 'True',
      FIntFRMMinGrassDistance: '0',
      FIntFRMMaxGrassDistance: '0',
      FFlagHandleAltEnterFullscreenManually: 'False',
    },
    quality: {
      FIntDebugForceMSAASamples: '4',
      DFFlagTextureQualityOverrideEnabled: 'True',
      DFIntTextureQualityOverride: '3',
    },
  };
  const flagsEl = $('[data-flags]');
  const presetBtns = $$('[data-presets] button');
  function showPreset(name) {
    presetBtns.forEach((b) => b.setAttribute('aria-selected', String(b.dataset.preset === name)));
    const entries = Object.entries(PRESETS[name]);
    const body = entries.map(([k, v], i) => `<span class="line" style="animation-delay:${i * 35}ms">  <span class="k">"${k}"</span><span class="p">: </span><span class="v">"${v}"</span>${i < entries.length - 1 ? '<span class="p">,</span>' : ''}</span>`).join('');
    flagsEl.innerHTML = `<span class="line"><span class="p">{</span></span>${body}<span class="line" style="animation-delay:${entries.length * 35}ms"><span class="p">}</span></span>`;
  }
  presetBtns.forEach((b) => b.addEventListener('click', () => showPreset(b.dataset.preset)));
  showPreset('maxFps');

  /* ---------------- FAQ: smooth open and close ---------------- */

  for (const d of $$('.faq details')) {
    const summary = $('summary', d);
    const body = $('p', d);
    summary.addEventListener('click', (e) => {
      if (reduced) return;
      e.preventDefault();
      if (d.open) {
        const a = body.animate([{ height: `${body.offsetHeight}px`, opacity: 1 }, { height: '0px', opacity: 0 }], { duration: 320, easing: 'cubic-bezier(.22,1,.36,1)' });
        a.onfinish = () => (d.open = false);
      } else {
        d.open = true;
        const h = body.offsetHeight;
        body.animate([{ height: '0px', opacity: 0 }, { height: `${h}px`, opacity: 1 }], { duration: 420, easing: 'cubic-bezier(.22,1,.36,1)' });
      }
    });
    body.style.overflow = 'hidden';
  }

  /* ---------------- Animated background ---------------- */

  const canvas = $('#bg');
  const ctx = canvas.getContext('2d');
  const glow = document.createElement('canvas');
  const gctx = glow.getContext('2d');
  // Soft colour fields; as the page scrolls they drift up and the lower ones come in.
  const BLOBS = [
    { c: [124, 58, 237], r: 0.55, sx: 0.00011, sy: 0.00015, px: 0.2, py: 0.12, ph: 0 },
    { c: [8, 145, 178], r: 0.46, sx: 0.00014, sy: 0.0001, px: 0.88, py: 0.3, ph: 2 },
    { c: [91, 92, 240], r: 0.5, sx: 0.00009, sy: 0.00013, px: 0.5, py: 0.8, ph: 4 },
    { c: [192, 38, 211], r: 0.32, sx: 0.00016, sy: 0.00012, px: 0.08, py: 0.7, ph: 1 },
    { c: [8, 145, 178], r: 0.5, sx: 0.00012, sy: 0.00011, px: 0.15, py: 1.25, ph: 3 },
    { c: [124, 58, 237], r: 0.5, sx: 0.0001, sy: 0.00014, px: 0.85, py: 1.45, ph: 5 },
  ];
  let W = 0;
  let H = 0;
  let dpr = 1;
  let particles = [];

  function resize() {
    dpr = Math.min(devicePixelRatio || 1, 1.5);
    W = innerWidth;
    H = innerHeight;
    canvas.width = Math.round(W * dpr);
    canvas.height = Math.round(H * dpr);
    glow.width = Math.max(32, Math.round(W / 10));
    glow.height = Math.max(32, Math.round(H / 10));
    const count = Math.round(clamp((W * H) / 16000, 28, 90));
    particles = Array.from({ length: count }, (_, i) => particles[i] || {
      x: Math.random() * W,
      y: Math.random() * H,
      vx: (Math.random() - 0.5) * 0.18,
      vy: (Math.random() - 0.5) * 0.18,
      r: Math.random() * 1.4 + 0.4,
      tw: Math.random() * Math.PI * 2,
    });
  }

  function drawGlow(t) {
    const gw = glow.width;
    const gh = glow.height;
    gctx.globalCompositeOperation = 'source-over';
    gctx.fillStyle = '#07060d';
    gctx.fillRect(0, 0, gw, gh);
    gctx.globalCompositeOperation = 'lighter';
    const max = document.documentElement.scrollHeight - innerHeight;
    const scrollShift = max > 0 ? (scrollY / max) * 0.95 : 0;
    for (const b of BLOBS) {
      const x = (b.px + Math.sin(t * b.sx + b.ph) * 0.18) * gw;
      const y = (b.py + Math.cos(t * b.sy + b.ph) * 0.16 - scrollShift) * gh;
      const r = b.r * Math.max(gw, gh);
      const g = gctx.createRadialGradient(x, y, 0, x, y, r);
      g.addColorStop(0, `rgba(${b.c.join(',')},0.34)`);
      g.addColorStop(1, `rgba(${b.c.join(',')},0)`);
      gctx.fillStyle = g;
      gctx.fillRect(0, 0, gw, gh);
    }
  }

  let lastScroll = scrollY;
  function frame(t) {
    drawGlow(t);
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.imageSmoothingEnabled = true;
    ctx.drawImage(glow, 0, 0, canvas.width, canvas.height);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

    const dy = (scrollY - lastScroll) * 0.12;
    lastScroll = scrollY;
    const px = pointer.x;
    const py = pointer.y;
    for (const p of particles) {
      p.x += p.vx;
      p.y += p.vy - dy;
      if (pointer.active) {
        const ddx = px - p.x;
        const ddy = py - p.y;
        const d2 = ddx * ddx + ddy * ddy;
        if (d2 < 32000 && d2 > 1) {
          p.x -= (ddx / Math.sqrt(d2)) * 0.35;
          p.y -= (ddy / Math.sqrt(d2)) * 0.35;
        }
      }
      if (p.x < -20) p.x = W + 20;
      if (p.x > W + 20) p.x = -20;
      if (p.y < -20) p.y = H + 20;
      if (p.y > H + 20) p.y = -20;
    }
    const LINK = 130;
    for (let i = 0; i < particles.length; i++) {
      const a = particles[i];
      for (let j = i + 1; j < particles.length; j++) {
        const b = particles[j];
        const ddx = a.x - b.x;
        const ddy = a.y - b.y;
        const d2 = ddx * ddx + ddy * ddy;
        if (d2 < LINK * LINK) {
          ctx.strokeStyle = `rgba(167,139,250,${(1 - Math.sqrt(d2) / LINK) * 0.16})`;
          ctx.lineWidth = 1;
          ctx.beginPath();
          ctx.moveTo(a.x, a.y);
          ctx.lineTo(b.x, b.y);
          ctx.stroke();
        }
      }
      if (pointer.active) {
        const ddx = a.x - px;
        const ddy = a.y - py;
        const d = Math.sqrt(ddx * ddx + ddy * ddy);
        if (d < 200) {
          ctx.strokeStyle = `rgba(103,232,249,${(1 - d / 200) * 0.28})`;
          ctx.beginPath();
          ctx.moveTo(a.x, a.y);
          ctx.lineTo(px, py);
          ctx.stroke();
        }
      }
      const tw = 0.55 + Math.sin(t * 0.002 + a.tw) * 0.35;
      ctx.fillStyle = `rgba(226,232,255,${tw})`;
      ctx.beginPath();
      ctx.arc(a.x, a.y, a.r, 0, Math.PI * 2);
      ctx.fill();
    }
  }

  let running = false;
  function loop(t) {
    if (!running) return;
    frame(t);
    requestAnimationFrame(loop);
  }
  function start() {
    if (running || reduced) return;
    running = true;
    requestAnimationFrame(loop);
  }
  resize();
  addEventListener('resize', () => {
    resize();
    if (reduced) frame(0);
  });
  if (reduced) {
    frame(0);
  } else {
    start();
    document.addEventListener('visibilitychange', () => {
      if (document.hidden) running = false;
      else start();
    });
  }
})();
