// OmniHub's site: the download link for the newest release, the index of the
// app's pages, and a viewer for screenshots. Everything works without it too.

(() => {
  const REPO = 'Kerioxsx/omnihub';
  const $ = (s, el = document) => el.querySelector(s);
  const $$ = (s, el = document) => [...el.querySelectorAll(s)];
  const size = (b) => (b >= 1e6 ? `${Math.round(b / 1e6)} MB` : `${Math.round(b / 1e3)} KB`);

  /* ---------- The newest release ---------- */

  let release = null;

  function applyRelease(r) {
    release = r;
    for (const a of $$('[data-download]')) a.href = r.url;
    for (const m of $$('[data-download-meta]')) m.textContent = `Version ${r.version}${r.size ? `, ${size(r.size)}` : ''}`;
    const name = $('[data-asset-name]');
    if (name) name.textContent = `It's ${r.name}.`;
    const note = $('[data-version-text]');
    if (note) note.textContent = `Version ${r.version} is out. Free for Windows 10 and 11.`;
  }

  // Asked fresh on each visit: nothing is stored in the browser (no cookies, no local storage).
  async function loadRelease() {
    try {
      const res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, { headers: { Accept: 'application/vnd.github+json' } });
      if (!res.ok) return;
      const json = await res.json();
      const assets = json.assets || [];
      const asset = assets.find((a) => /x64-setup\.exe$/i.test(a.name)) || assets.find((a) => /\.exe$/i.test(a.name)) || assets.find((a) => /\.msi$/i.test(a.name));
      if (!asset) return;
      applyRelease({ version: String(json.tag_name || '').replace(/^v/, ''), url: asset.browser_download_url, name: asset.name, size: asset.size });
    } catch {
      /* Offline or rate-limited: the links still open the Releases page. */
    }
  }
  loadRelease();

  const toast = $('.toast');
  let toastTimer = 0;
  for (const a of $$('[data-download]')) {
    a.addEventListener('click', () => {
      if (!release || !toast) return;
      toast.innerHTML = `<b>Downloading OmniHub ${release.version}</b>Open it when it finishes. If Windows says it protected your PC, choose More info, then Run anyway.`;
      toast.classList.add('show');
      clearTimeout(toastTimer);
      toastTimer = setTimeout(() => toast.classList.remove('show'), 9000);
    });
  }
  toast?.addEventListener('click', () => toast.classList.remove('show'));

  /* ---------- Top bar ---------- */

  const top = $('.top');
  const onScroll = () => top.classList.toggle('is-scrolled', scrollY > 8);
  addEventListener('scroll', onScroll, { passive: true });
  onScroll();

  /* ---------- The index ---------- */

  const index = $('[data-index]');
  if (index) {
    const tabs = $$('[role="tab"]', index);
    const body = $('[data-index-body]', index);
    const text = $('[data-index-text]', index);
    const title = $('[data-index-title]', index);
    const images = new Map([[tabs[0].dataset.shot, $('img', body)]]);
    const fine = matchMedia('(hover: hover) and (pointer: fine)');
    let current = tabs[0];

    const imageFor = (tab) => {
      let img = images.get(tab.dataset.shot);
      if (!img) {
        img = new Image(2880, 1800);
        img.src = tab.dataset.shot;
        img.alt = tab.dataset.alt || '';
        img.decoding = 'async';
        body.append(img);
        images.set(tab.dataset.shot, img);
      }
      return img;
    };

    const select = (tab, focus = false) => {
      if (tab === current) return;
      current = tab;
      for (const t of tabs) {
        const on = t === tab;
        t.setAttribute('aria-selected', String(on));
        t.tabIndex = on ? 0 : -1;
      }
      const img = imageFor(tab);
      const show = () => {
        if (current !== tab) return;
        for (const i of images.values()) i.classList.toggle('is-on', i === img);
      };
      img.complete ? show() : img.addEventListener('load', show, { once: true });
      text.textContent = tab.dataset.text;
      title.textContent = `OmniHub · ${tab.textContent.trim()}`;
      if (focus) tab.focus();
    };

    title.textContent = `OmniHub · ${tabs[0].textContent.trim()}`;
    tabs.forEach((t, i) => {
      t.tabIndex = i === 0 ? 0 : -1;
      t.addEventListener('click', () => select(t));
      t.addEventListener('focus', () => select(t));
      t.addEventListener('pointerenter', () => fine.matches && select(t));
      t.addEventListener('keydown', (e) => {
        const step = { ArrowDown: 1, ArrowRight: 1, ArrowUp: -1, ArrowLeft: -1 }[e.key];
        if (step) {
          e.preventDefault();
          select(tabs[(i + step + tabs.length) % tabs.length], true);
        } else if (e.key === 'Home' || e.key === 'End') {
          e.preventDefault();
          select(tabs[e.key === 'Home' ? 0 : tabs.length - 1], true);
        }
      });
    });

    // The first word stretches out once the page has settled; the other pictures load after that.
    const settle = () => {
      index.classList.add('is-ready');
      const later = () => tabs.slice(1).forEach(imageFor);
      'requestIdleCallback' in window ? requestIdleCallback(later, { timeout: 3000 }) : setTimeout(later, 1500);
    };
    document.fonts?.ready.then(() => setTimeout(settle, 250)) ?? setTimeout(settle, 400);
  }

  /* ---------- Screenshot viewer ---------- */

  const viewer = $('.viewer');
  if (viewer && typeof viewer.showModal === 'function') {
    const big = $('img', viewer);
    let opener = null;
    for (const img of $$('.shot img, .phone img, .index-stage')) {
      const target = img.matches('.index-stage') ? $('.win-body', img) : img;
      target.classList.add('zoomable');
      target.addEventListener('click', () => {
        const src = img.matches('.index-stage') ? $('.is-on', img) : img;
        if (!src) return;
        opener = document.activeElement;
        big.src = src.currentSrc || src.src;
        big.alt = src.alt;
        viewer.showModal();
      });
    }
    $('.viewer-close', viewer).addEventListener('click', () => viewer.close());
    viewer.addEventListener('click', (e) => e.target === viewer && viewer.close());
    viewer.addEventListener('close', () => opener?.focus?.());
  }
})();
