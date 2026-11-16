// OmniHub Passwords — runs in every page and frame.
//
// Finds sign-in fields (username, email, password, 2FA code), shows a small
// OmniHub menu under the focused one, and fills what the user picks. The
// menu lives in a closed shadow root, and it never shows a password: the app
// hands one over only for a login saved for this frame's own site.

(() => {
  if (window.__omnihubPasswords) return;
  window.__omnihubPasswords = true;

  // Inline, so pages need no access to the extension's files (and cannot
  // probe for it through web-accessible resources).
  const ICON =
    'data:image/svg+xml,' +
    encodeURIComponent(
      '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#8b6cff"/><stop offset="1" stop-color="#4f8cff"/></linearGradient></defs><rect width="32" height="32" rx="8" fill="url(#g)"/><circle cx="16" cy="13" r="5" fill="none" stroke="#fff" stroke-width="2.6"/><path d="M16 18v8M16 22h4" stroke="#fff" stroke-width="2.6" stroke-linecap="round"/></svg>',
    );
  const USERNAME_RE = /user(name)?|login|e-?mail|identifier|account|benutzer|usuario|utilisateur|courriel|phone|mobile/i;
  const OTP_RE = /one.?time|otp|totp|2fa|mfa|two.?factor|verification.?code|verify.?code|auth.?code|security.?code|passcode|token|totppin|idvpin/i;
  const SKIP_RE = /search|query|captcha|coupon|promo|zip|postal|card.?number|cvc|cvv|first.?name|last.?name|address|city/i;

  let menuHost = null;
  let shadow = null;
  let menu = null;
  let badge = null;
  let current = null; // the field the menu belongs to
  let cache = null; // { at, data } — last match answer for this frame
  let dismissed = false; // the user closed the menu: don't reopen it by itself
  let lastCaptured = '';
  let active = -1; // keyboard-highlighted menu item
  let filling = false; // focus moves while filling: don't reopen the menu

  // ---------------------------------------------------------------- messaging

  function ask(kind, extra = {}) {
    return new Promise((resolve) => {
      try {
        chrome.runtime.sendMessage({ kind, ...extra }, (r) => {
          if (chrome.runtime.lastError) resolve({ ok: false, code: 'extension', error: chrome.runtime.lastError.message });
          else resolve(r ?? { ok: false, code: 'extension', error: 'No answer' });
        });
      } catch (e) {
        // The extension was reloaded or removed: this script is orphaned.
        resolve({ ok: false, code: 'extension', error: String(e?.message ?? e) });
      }
    });
  }

  async function matches(force = false) {
    if (!force && cache && Date.now() - cache.at < 20000) return cache.data;
    const r = await ask('match');
    const data = r.ok ? { ok: true, ...r.data } : { ok: false, code: r.code, error: r.error };
    if (r.ok || r.code === 'locked') cache = { at: Date.now(), data };
    return data;
  }

  // ------------------------------------------------------------------ fields

  function visible(el) {
    if (!el.isConnected || el.disabled || el.readOnly) return false;
    const r = el.getBoundingClientRect();
    if (r.width < 20 || r.height < 10) return false;
    const st = getComputedStyle(el);
    return st.visibility !== 'hidden' && st.display !== 'none' && Number(st.opacity) > 0.05;
  }

  function attrs(el) {
    return [el.name, el.id, el.getAttribute('autocomplete'), el.getAttribute('aria-label'), el.placeholder, el.getAttribute('data-testid')].filter(Boolean).join(' ');
  }

  function isPassword(el) {
    return el instanceof HTMLInputElement && el.type === 'password';
  }

  function isOtp(el) {
    if (!(el instanceof HTMLInputElement) || !['text', 'tel', 'number', 'password', ''].includes(el.type)) return false;
    const ac = (el.getAttribute('autocomplete') ?? '').toLowerCase();
    if (ac.includes('one-time-code')) return true;
    if (el.type === 'password') return false;
    return OTP_RE.test(attrs(el)) && !USERNAME_RE.test(el.getAttribute('autocomplete') ?? '');
  }

  function isUsername(el) {
    if (!(el instanceof HTMLInputElement) || !['text', 'email', 'tel', ''].includes(el.type)) return false;
    if (isOtp(el)) return false;
    const ac = (el.getAttribute('autocomplete') ?? '').toLowerCase();
    if (/\b(username|email|webauthn)\b/.test(ac)) return true;
    if (el.type === 'email') return true;
    const a = attrs(el);
    if (SKIP_RE.test(a)) return false;
    if (USERNAME_RE.test(a)) return true;
    // A plain text box right before a password box is the login.
    const pw = passwordFields(scopeOf(el))[0];
    return !!pw && prevInput(pw) === el;
  }

  function scopeOf(el) {
    return el.form ?? el.closest('form, [role=dialog], main, section, body') ?? document.body;
  }

  function inputsIn(scope) {
    return [...scope.querySelectorAll('input')].filter(visible);
  }

  function passwordFields(scope) {
    return inputsIn(scope).filter(isPassword);
  }

  function prevInput(el) {
    const all = inputsIn(scopeOf(el)).filter((i) => !['hidden', 'checkbox', 'radio', 'submit', 'button'].includes(i.type));
    const at = all.indexOf(el);
    return at > 0 ? all[at - 1] : null;
  }

  function isNewPassword(el) {
    const ac = (el.getAttribute('autocomplete') ?? '').toLowerCase();
    if (ac.includes('new-password')) return true;
    if (ac.includes('current-password')) return false;
    return /new|confirm|repeat|register|signup|sign-up|create/i.test(attrs(el)) || passwordFields(scopeOf(el)).length >= 2;
  }

  function fieldKind(el) {
    if (!(el instanceof HTMLInputElement) || !visible(el)) return null;
    if (isPassword(el)) return isOtp(el) ? 'otp' : 'password';
    if (isOtp(el)) return 'otp';
    if (isUsername(el)) return 'username';
    return null;
  }

  /** The login fields that belong with `el` (or with the whole page). */
  function loginFields(el) {
    const scope = el ? scopeOf(el) : document.body;
    const inputs = inputsIn(scope);
    const pw = inputs.filter(isPassword).filter((p) => !isOtp(p));
    const user = inputs.filter((i) => !isPassword(i) && isUsername(i));
    const otp = inputs.filter(isOtp);
    return { user, pw, otp };
  }

  function pageHasLoginFields() {
    const f = loginFields(null);
    return f.pw.length > 0 || f.user.length > 0 || f.otp.length > 0;
  }

  // ----------------------------------------------------------------- filling

  const valueSetter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;

  function setValue(el, value) {
    if (value == null || value === '') return false;
    el.focus({ preventScroll: true });
    valueSetter.call(el, value);
    el.dispatchEvent(new InputEvent('input', { bubbles: true, composed: true, inputType: 'insertReplacementText', data: value }));
    el.dispatchEvent(new Event('change', { bubbles: true }));
    return true;
  }

  function loginFor(el, data) {
    const wantsEmail = el.type === 'email' || /e-?mail|identifier/i.test(attrs(el));
    return wantsEmail ? data.email || data.username : data.username || data.email;
  }

  async function fillEntry(id, target, quiet = false) {
    filling = true;
    try {
      return await fillEntryInner(id, target, quiet);
    } finally {
      // Done here: the menu stays closed until the user asks again.
      dismissed = true;
      setTimeout(() => (filling = false), 0);
    }
  }

  async function fillEntryInner(id, target, quiet) {
    const r = await ask('fill', { id });
    if (!r.ok) {
      // A popup fill reaches every frame; frames of other sites just decline.
      if (!(quiet && r.code === 'wrong-site')) toast(r.error || 'OmniHub could not fill this login.');
      return false;
    }
    const data = r.data;
    const { user, pw, otp } = loginFields(target);
    let filled = false;
    if (target && fieldKind(target) === 'otp') {
      filled = setValue(target, data.totp);
      if (!data.totp) toast('This login has no 2FA code saved in OmniHub.');
    } else {
      for (const u of user.slice(0, 1)) filled = setValue(u, loginFor(u, data)) || filled;
      // Fill the sign-in password, not "new password" boxes on a change form.
      const login = pw.filter((p) => !isNewPassword(p));
      for (const p of (login.length ? login : pw).slice(0, 1)) filled = setValue(p, data.password) || filled;
      if (otp.length && data.totp) filled = setValue(otp[0], data.totp) || filled;
    }
    // Leave the focus where the user is likely to continue.
    if (target?.isConnected) target.focus({ preventScroll: true });
    if (filled && user.length && !pw.length) ask('remember-username', { username: loginFor(user[0], data) });
    return filled;
  }

  async function fillGenerated(target) {
    const r = await ask('generate', { length: 20 });
    if (!r.ok) return toast(r.error || 'Could not generate a password.');
    for (const p of passwordFields(scopeOf(target)).filter(isNewPassword)) setValue(p, r.data.password);
    if (!isNewPassword(target)) setValue(target, r.data.password);
    toast('Strong password filled. OmniHub will offer to save it when you submit.');
  }

  // -------------------------------------------------------------------- UI

  const CSS = `
    :host { all: initial; }
    * { box-sizing: border-box; font-family: system-ui, -apple-system, "Segoe UI", sans-serif; }
    .badge { position: fixed; width: 22px; height: 22px; border-radius: 6px; cursor: pointer; display: grid; place-items: center;
      background: #1c1b22; box-shadow: 0 1px 4px rgba(0,0,0,.35); pointer-events: auto; }
    .badge img { width: 16px; height: 16px; }
    .menu { position: fixed; min-width: 260px; max-width: 360px; background: #17161c; color: #ecebf3; border: 1px solid #2f2d3a;
      border-radius: 12px; box-shadow: 0 12px 32px rgba(0,0,0,.45); padding: 6px; pointer-events: auto; font-size: 13px; }
    .head { display: flex; align-items: center; gap: 8px; padding: 6px 8px 8px; color: #a9a6b8; font-size: 11px; text-transform: uppercase; letter-spacing: .04em; }
    .head img { width: 14px; height: 14px; }
    .item { display: flex; align-items: center; gap: 10px; width: 100%; padding: 8px; border: 0; border-radius: 8px; background: none; color: inherit; text-align: left; cursor: pointer; font-size: 13px; }
    .item:hover, .item:focus-visible, .item.active { background: #262431; outline: none; }
    .avatar { flex: none; width: 28px; height: 28px; border-radius: 8px; background: #7c5cff; color: #fff; display: grid; place-items: center; font-weight: 600; }
    .t { min-width: 0; flex: 1; }
    .t b { display: block; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    .t span { display: block; color: #a9a6b8; font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    .tag { font-size: 10px; color: #c9b8ff; border: 1px solid #4b3f8f; border-radius: 999px; padding: 1px 6px; }
    .msg { padding: 8px 10px 10px; color: #cfcde0; line-height: 1.4; }
    .row { display: flex; gap: 6px; padding: 4px 6px 6px; flex-wrap: wrap; }
    .btn { border: 0; border-radius: 8px; padding: 7px 10px; background: #7c5cff; color: #fff; font-weight: 600; font-size: 12px; cursor: pointer; }
    .btn.ghost { background: #262431; color: #ecebf3; }
    .sep { height: 1px; background: #2f2d3a; margin: 4px 2px; }
    .banner { position: fixed; top: 16px; right: 16px; width: 340px; background: #17161c; color: #ecebf3; border: 1px solid #2f2d3a;
      border-radius: 14px; box-shadow: 0 16px 40px rgba(0,0,0,.5); padding: 14px; pointer-events: auto; font-size: 13px; }
    .banner h3 { margin: 0 0 4px; font-size: 14px; display: flex; gap: 8px; align-items: center; }
    .banner h3 img { width: 18px; height: 18px; }
    .banner p { margin: 0 0 10px; color: #a9a6b8; word-break: break-word; }
    .toast { position: fixed; bottom: 18px; left: 50%; transform: translateX(-50%); background: #17161c; color: #ecebf3; border: 1px solid #2f2d3a;
      border-radius: 10px; padding: 9px 14px; font-size: 13px; box-shadow: 0 8px 24px rgba(0,0,0,.4); pointer-events: none; max-width: 420px; }
  `;

  function ensureHost() {
    if (menuHost?.isConnected) return;
    menuHost = document.createElement('omnihub-passwords');
    menuHost.style.cssText = 'all:initial; position:fixed; top:0; left:0; width:0; height:0; z-index:2147483647; pointer-events:none;';
    shadow = menuHost.attachShadow({ mode: 'closed' });
    const style = document.createElement('style');
    style.textContent = CSS;
    shadow.append(style);
    document.documentElement.append(menuHost);
  }

  function h(tag, props = {}, ...children) {
    const el = document.createElement(tag);
    for (const [k, v] of Object.entries(props)) {
      if (k === 'class') el.className = v;
      else if (k.startsWith('on')) el.addEventListener(k.slice(2), v);
      else el.setAttribute(k, v);
    }
    for (const c of children.flat()) if (c != null) el.append(c);
    return el;
  }

  function place() {
    if (!current || !current.isConnected) return hideAll();
    const r = current.getBoundingClientRect();
    if (badge) {
      badge.style.left = `${Math.max(0, r.right - 28)}px`;
      badge.style.top = `${r.top + (r.height - 22) / 2}px`;
    }
    if (menu) {
      const below = r.bottom + 6;
      const height = menu.offsetHeight || 200;
      menu.style.left = `${Math.max(8, Math.min(r.left, innerWidth - (menu.offsetWidth || 280) - 8))}px`;
      menu.style.top = `${below + height > innerHeight && r.top > height + 12 ? r.top - height - 6 : below}px`;
    }
  }

  function showBadge(field) {
    ensureHost();
    current = field;
    if (!badge) {
      badge = h('div', { class: 'badge', title: 'OmniHub Passwords', onmousedown: (e) => e.preventDefault(), onclick: () => (menu ? closeMenu(true) : openMenu(true)) }, h('img', { src: ICON, alt: '' }));
      shadow.append(badge);
    }
    place();
  }

  function closeMenu(byUser = false) {
    menu?.remove();
    menu = null;
    active = -1;
    if (byUser) dismissed = true;
  }

  function hideAll() {
    closeMenu();
    badge?.remove();
    badge = null;
    current = null;
  }

  function initials(s) {
    return (s || '?').trim().slice(0, 1).toUpperCase();
  }

  function entryButton(e, label) {
    const sub = e.username || e.email || (e.hasPassword ? '••••••••' : '');
    return h(
      'button',
      { class: 'item', type: 'button', onmousedown: (ev) => ev.preventDefault(), onclick: async () => {
        const target = current;
        closeMenu();
        await fillEntry(e.id, target);
      } },
      h('div', { class: 'avatar' }, initials(e.title)),
      h('div', { class: 't' }, h('b', {}, e.title || e.url || 'Login'), h('span', {}, sub)),
      label ? h('span', { class: 'tag' }, label) : null,
    );
  }

  function header(text) {
    return h('div', { class: 'head' }, h('img', { src: ICON, alt: '' }), text);
  }

  async function openMenu(byUser = false) {
    if (!current) return;
    const field = current;
    const kind = fieldKind(field);
    if (!kind) return;
    const data = await matches(byUser);
    if (current !== field) return;
    // Opened by focusing a field: stay quiet unless there is something to offer.
    if (!byUser) {
      if (dismissed) return;
      if (!data.ok && data.code !== 'locked' && data.code !== 'not-paired') return;
      if (data.ok && !data.entries.length && !(kind === 'password' && isNewPassword(field))) return;
    }
    closeMenu();
    ensureHost();
    menu = h('div', { class: 'menu', role: 'listbox', onmousedown: (e) => e.preventDefault() });
    if (data.ok) {
      let list = data.entries;
      if (kind === 'otp') list = list.filter((e) => e.hasTotp);
      const prefer = data.prefer && list.find((e) => e.id === data.prefer);
      if (prefer) list = [prefer, ...list.filter((e) => e !== prefer)];
      menu.append(header(list.length ? (kind === 'otp' ? '2FA code from OmniHub' : `Logins for ${data.site}`) : 'OmniHub Passwords'));
      for (const e of list.slice(0, 8)) menu.append(entryButton(e, e === prefer ? 'Continue' : kind === 'otp' ? 'Code' : null));
      if (!list.length) menu.append(h('div', { class: 'msg' }, kind === 'otp' ? 'No login with a 2FA code for this site.' : `No saved login for ${data.site || 'this site'}.`));
      if (kind === 'password' && isNewPassword(field)) {
        menu.append(h('div', { class: 'sep' }));
        menu.append(h('button', { class: 'item', type: 'button', onclick: () => { const t = current; closeMenu(); fillGenerated(t); } },
          h('div', { class: 'avatar' }, '✱'), h('div', { class: 't' }, h('b', {}, 'Use a strong password'), h('span', {}, 'Generated by OmniHub'))));
      }
    } else if (data.code === 'locked') {
      menu.append(header('OmniHub is locked'));
      menu.append(h('div', { class: 'msg' }, 'Unlock your vault to fill this login.'));
      const row = h('div', { class: 'row' });
      const st = await ask('state');
      if (st.ok && st.data.vault?.helloEnabled) row.append(h('button', { class: 'btn', type: 'button', onclick: () => unlock('hello') }, 'Windows Hello'));
      row.append(h('button', { class: st.ok && st.data.vault?.helloEnabled ? 'btn ghost' : 'btn', type: 'button', onclick: () => unlock('window') }, 'Unlock in OmniHub'));
      menu.append(row);
    } else if (data.code === 'not-paired') {
      menu.append(header('OmniHub Passwords'));
      menu.append(h('div', { class: 'msg' }, 'Connect this browser to OmniHub once to fill logins from your vault.'));
      menu.append(h('div', { class: 'row' }, h('button', { class: 'btn', type: 'button', onclick: () => { closeMenu(); ask('open-popup'); } }, 'Connect')));
    } else if (data.code === 'app-not-running') {
      menu.append(header('OmniHub is not running'));
      menu.append(h('div', { class: 'row' }, h('button', { class: 'btn', type: 'button', onclick: async () => {
        closeMenu();
        toast('Starting OmniHub…');
        await ask('launch-app');
        cache = null;
        if (current) openMenu(true);
      } }, 'Start OmniHub')));
    } else {
      menu.append(header('OmniHub Passwords'));
      menu.append(h('div', { class: 'msg' }, data.error || 'OmniHub is not available.'));
    }
    shadow.append(menu);
    place();
  }

  async function unlock(method) {
    closeMenu();
    const r = await ask('unlock', { method });
    if (!r.ok) return toast(r.error || 'Could not unlock.');
    if (r.data?.pending) return toast('Unlock the vault in the OmniHub window, then come back.');
    cache = null;
    if (current) openMenu(true);
  }

  let toastTimer = null;
  function toast(text) {
    ensureHost();
    shadow.querySelector('.toast')?.remove();
    const t = h('div', { class: 'toast' }, text);
    shadow.append(t);
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => t.remove(), 3500);
  }

  // ------------------------------------------------------------- save offer

  async function showOffer() {
    if (window !== top) return;
    const r = await ask('pending-offer');
    const o = r.ok ? r.data : null;
    if (!o) return;
    ensureHost();
    shadow.querySelector('.banner')?.remove();
    const answer = (a) => async () => {
      banner.remove();
      const res = await ask('offer-answer', { answer: a });
      if (a === 'save') toast(res.ok ? (res.data?.updated ? 'Password updated in OmniHub.' : 'Saved to OmniHub.') : res.error || 'Could not save.');
    };
    const changed = o.status === 'changed';
    const banner = h(
      'div',
      { class: 'banner', role: 'dialog' },
      h('h3', {}, h('img', { src: ICON, alt: '' }), changed ? 'Update password in OmniHub?' : 'Save login to OmniHub?'),
      h('p', {}, `${o.username || 'No username'} · ${o.site}${o.locked ? ' — OmniHub will ask you to unlock first.' : ''}`),
      h('div', { class: 'row' },
        h('button', { class: 'btn', type: 'button', onclick: answer('save') }, changed ? 'Update' : 'Save'),
        h('button', { class: 'btn ghost', type: 'button', onclick: answer('later') }, 'Not now'),
        h('button', { class: 'btn ghost', type: 'button', onclick: answer('never') }, 'Never for this site')),
    );
    shadow.append(banner);
  }

  // ------------------------------------------------------------- capturing

  function capture(from) {
    const scope = from ? scopeOf(from) : document.body;
    const pw = passwordFields(scope).filter((p) => p.value && !isOtp(p));
    const user = inputsIn(scope).filter((i) => !isPassword(i) && isUsername(i) && i.value);
    if (!pw.length) {
      // Identifier-first sign-in (Gmail, Microsoft…): remember who is signing in.
      if (user.length) ask('remember-username', { username: user[0].value.trim() });
      return;
    }
    // On a change-password form the new password is the one to keep.
    const fresh = pw.filter(isNewPassword);
    const password = (fresh.length ? fresh[fresh.length - 1] : pw[0]).value;
    const username = user[0]?.value.trim() ?? '';
    const key = `${username}\u0000${password}`;
    if (key === lastCaptured) return;
    lastCaptured = key;
    ask('captured', { username, password });
  }

  document.addEventListener('submit', (e) => capture(e.target instanceof HTMLFormElement ? e.target.querySelector('input') ?? e.target : null), true);
  document.addEventListener(
    'click',
    (e) => {
      const b = e.target instanceof Element ? e.target.closest('button, input[type=submit], input[type=button], [role=button]') : null;
      if (!b) return;
      const label = `${b.textContent ?? ''} ${b.value ?? ''} ${b.getAttribute('aria-label') ?? ''} ${b.id}`.toLowerCase();
      if (b.type === 'submit' || /sign.?in|log.?in|next|continue|submit|weiter|suivant|siguiente|create|register|sign.?up|save/.test(label)) {
        const f = b.closest('form') ?? document.activeElement;
        capture(f instanceof HTMLInputElement ? f : f?.querySelector?.('input') ?? null);
      }
    },
    true,
  );
  document.addEventListener(
    'keydown',
    (e) => {
      if (e.key === 'Escape' && menu) {
        closeMenu(true);
        return;
      }
      // Arrow keys move through the menu, Enter picks (the field keeps focus).
      if (menu && e.target === current && (e.key === 'ArrowDown' || e.key === 'ArrowUp' || (e.key === 'Enter' && active >= 0))) {
        const items = [...menu.querySelectorAll('.item')];
        if (!items.length) return;
        e.preventDefault();
        e.stopImmediatePropagation();
        if (e.key === 'Enter') {
          items[active]?.click();
          return;
        }
        active = (active + (e.key === 'ArrowDown' ? 1 : -1) + items.length) % items.length;
        items.forEach((it, i) => it.classList.toggle('active', i === active));
        return;
      }
      if (e.key === 'ArrowDown' && !menu && e.target === current) {
        openMenu(true);
        return;
      }
      if (e.key === 'Enter' && e.target instanceof HTMLInputElement && fieldKind(e.target)) capture(e.target);
    },
    true,
  );

  // --------------------------------------------------------------- focus

  function onField(el, clicked) {
    if (!(el instanceof HTMLInputElement) || !fieldKind(el)) return;
    if (filling) {
      showBadge(el);
      return;
    }
    if (current !== el || clicked) dismissed = false;
    showBadge(el);
    if (!menu) openMenu(false);
  }
  document.addEventListener('focusin', (e) => onField(e.composedPath()[0], false), true);
  // Clicking a field that already has the focus (autofocus) opens the menu too.
  document.addEventListener('click', (e) => onField(e.composedPath()[0], true), true);
  document.addEventListener(
    'focusout',
    () => {
      setTimeout(() => {
        const a = document.activeElement;
        if (a !== current && a !== menuHost) hideAll();
      }, 150);
    },
    true,
  );
  addEventListener('scroll', () => requestAnimationFrame(place), true);
  addEventListener('resize', () => requestAnimationFrame(place));

  // ------------------------------------------------------- from the worker

  chrome.runtime.onMessage.addListener((msg, _sender, reply) => {
    if (msg?.kind === 'state-changed') {
      cache = null;
      if (current && document.activeElement === current) openMenu(false);
    } else if (msg?.kind === 'offer-ready') {
      setTimeout(showOffer, 600);
    } else if (msg?.kind === 'fill-best' || msg?.kind === 'fill-entry') {
      if (!pageHasLoginFields()) return reply({ filled: false });
      // The shortcut acts in the frame being typed in (or the page itself).
      if (msg.kind === 'fill-best' && window !== top && !document.hasFocus()) return reply({ filled: false });
      const active = document.activeElement instanceof HTMLInputElement && fieldKind(document.activeElement) ? document.activeElement : null;
      (async () => {
        let id = msg.id;
        if (!id) {
          const m = await matches(true);
          if (!m.ok) {
            toast(m.error || 'OmniHub is not available.');
            return reply({ filled: false });
          }
          id = m.prefer ?? m.entries[0]?.id;
          if (!id) {
            toast('No saved login for this site.');
            return reply({ filled: false });
          }
        }
        const f = loginFields(null);
        const target = active ?? f.user[0] ?? f.pw[0] ?? f.otp[0] ?? null;
        reply({ filled: await fillEntry(id, target, msg.kind === 'fill-entry' && window !== top) });
      })();
      return true;
    }
    return false;
  });

  if (window === top) {
    // A login submitted on the previous page waits for an answer.
    if (document.readyState === 'complete') showOffer();
    else addEventListener('load', showOffer, { once: true });
  }
  // The page may have focused a field before this script ran (autofocus).
  if (document.hasFocus()) onField(document.activeElement, false);
})();
