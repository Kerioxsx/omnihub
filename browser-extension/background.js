// OmniHub Passwords — background service worker.
//
// Talks to the OmniHub app through the native-messaging host
// (app.omnihub.vault). The app decides what may be filled where: this worker
// only passes along the URL of the frame that asks, which the browser — not
// the page — tells us (sender.url).

const HOST = 'app.omnihub.vault';
const TIMEOUT_MS = 15000;
const PAIR_TIMEOUT_MS = 125000;

let port = null;
let seq = 0;
const waiting = new Map();
const state = {
  connected: false,
  helloDone: false,
  paired: false,
  vault: { exists: false, unlocked: false, helloEnabled: false },
  offerSave: true,
  error: null, // code: host-missing | app-not-running | disabled | …
  errorText: null,
  pairing: null, // { requestId, code }
};
// Per tab: the entry picked on a username-only page (Gmail step 1) and the
// username typed there, so the password step can follow on.
const tabMemory = new Map();
// Per tab: a login typed and submitted, waiting for "Save to OmniHub?".
const saveOffers = new Map();

function browserName() {
  const brands = navigator.userAgentData?.brands?.map((b) => b.brand) ?? [];
  const name = navigator.brave ? 'Brave' : brands.includes('Microsoft Edge') ? 'Edge' : brands.includes('Google Chrome') ? 'Chrome' : 'Chromium';
  const os = navigator.userAgentData?.platform || 'this PC';
  return `${name} on ${os}`;
}

function setBadge() {
  const text = !state.connected || state.error ? '!' : !state.paired ? '?' : state.vault.unlocked ? '' : '🔒';
  chrome.action.setBadgeText({ text: text === '🔒' ? '' : text }).catch(() => {});
  chrome.action.setBadgeBackgroundColor({ color: '#7c5cff' }).catch(() => {});
  chrome.action.setTitle({ title: state.vault.unlocked ? 'OmniHub Passwords — unlocked' : 'OmniHub Passwords' }).catch(() => {});
}

function failAll(code, text) {
  for (const { reject, timer } of waiting.values()) {
    clearTimeout(timer);
    reject(Object.assign(new Error(text), { code }));
  }
  waiting.clear();
}

function connect() {
  if (port) return;
  try {
    port = chrome.runtime.connectNative(HOST);
  } catch (e) {
    state.error = 'host-missing';
    state.errorText = String(e?.message ?? e);
    return;
  }
  state.connected = true;
  state.error = null;
  port.onMessage.addListener(onMessage);
  port.onDisconnect.addListener(() => {
    const msg = chrome.runtime.lastError?.message ?? 'disconnected';
    port = null;
    state.connected = false;
    state.helloDone = false;
    state.error = /not found|forbidden|not allowed/i.test(msg) ? 'host-missing' : 'disconnected';
    state.errorText = msg;
    failAll(state.error, msg);
    setBadge();
  });
}

function onMessage(msg) {
  if (msg?.type === 'event') {
    if (msg.topic === 'vault:locked') state.vault.unlocked = false;
    if (msg.topic === 'vault:unlocked') state.vault.unlocked = true;
    setBadge();
    broadcast({ kind: 'state-changed' });
    notifyTabs();
    return;
  }
  const w = waiting.get(msg?.id);
  if (!w) return;
  waiting.delete(msg.id);
  clearTimeout(w.timer);
  if (msg.ok) w.resolve(msg);
  else {
    if (['app-not-running', 'disabled', 'not-paired', 'unknown-extension'].includes(msg.code)) {
      state.error = msg.code === 'not-paired' ? null : msg.code;
      state.errorText = msg.error;
      if (msg.code === 'not-paired') state.paired = false;
    }
    if (msg.code === 'locked') state.vault.unlocked = false;
    w.reject(Object.assign(new Error(msg.error || 'OmniHub refused'), { code: msg.code }));
  }
}

function rawSend(payload, timeout = TIMEOUT_MS) {
  connect();
  if (!port) return Promise.reject(Object.assign(new Error(state.errorText || 'OmniHub host not installed'), { code: state.error || 'host-missing' }));
  const id = ++seq;
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      waiting.delete(id);
      reject(Object.assign(new Error('OmniHub did not answer'), { code: 'timeout' }));
    }, timeout);
    waiting.set(id, { resolve, reject, timer });
    port.postMessage({ ...payload, id });
  });
}

async function hello() {
  const { token } = await chrome.storage.local.get('token');
  const r = await rawSend({ type: 'hello', token: token ?? null, browser: browserName() });
  state.helloDone = true;
  state.paired = !!r.paired;
  state.vault = r.vault ?? state.vault;
  state.offerSave = r.offerSave !== false;
  state.error = null;
  setBadge();
  return r;
}

/** Send a request after making sure the session said hello. */
async function send(payload, timeout) {
  if (!state.helloDone) await hello();
  return rawSend(payload, timeout);
}

async function refresh() {
  try {
    state.helloDone = false;
    await hello();
  } catch (e) {
    if (!state.error) {
      state.error = e.code ?? 'error';
      state.errorText = e.message;
    }
    setBadge();
  }
  return snapshot();
}

function snapshot() {
  return { connected: state.connected, paired: state.paired, vault: state.vault, error: state.error, errorText: state.errorText, pairing: state.pairing, offerSave: state.offerSave };
}

async function pair() {
  const start = await send({ type: 'pair', browser: browserName() });
  state.pairing = { requestId: start.requestId, code: start.code };
  broadcast({ kind: 'state-changed' });
  try {
    const done = await rawSend({ type: 'pair-wait', requestId: start.requestId }, PAIR_TIMEOUT_MS);
    await chrome.storage.local.set({ token: done.token });
    state.paired = true;
    state.vault = done.vault ?? state.vault;
    notifyTabs();
    return snapshot();
  } finally {
    state.pairing = null;
    setBadge();
    broadcast({ kind: 'state-changed' });
  }
}

function broadcast(msg) {
  chrome.runtime.sendMessage(msg).catch(() => {});
}

/** Let the pages in view refresh their fill menus (after unlocking, pairing…). */
function notifyTabs() {
  chrome.tabs.query({ active: true }).then(
    (tabs) => tabs.forEach((t) => chrome.tabs.sendMessage(t.id, { kind: 'state-changed' }).catch(() => {})),
    () => {},
  );
}

function siteKey(url) {
  try {
    const h = new URL(url).hostname;
    return h.split('.').slice(-2).join('.');
  } catch {
    return '';
  }
}

async function handle(msg, sender) {
  // Only the extension's own pages (the popup, also when opened in a tab)
  // may name a URL and tab; a content script always acts for the frame it
  // runs in, as the browser reports it. Web pages cannot load our pages (no
  // web-accessible resources), so the origin identifies them.
  const fromExtension = sender.id === chrome.runtime.id && (sender.url ?? '').startsWith(chrome.runtime.getURL(''));
  const frameUrl = sender.url ?? '';
  const tabId = fromExtension ? msg.tabId : sender.tab?.id;
  const pageUrl = fromExtension && msg.url ? msg.url : frameUrl;
  switch (msg.kind) {
    case 'state':
      return msg.refresh ? refresh() : snapshot();
    case 'pair':
      return pair();
    case 'launch-app': {
      const r = await rawSend({ type: 'launch-app' }, 20000).catch((e) => ({ ok: false, error: e.message }));
      // The host reconnects to the app; start a fresh session.
      try {
        port?.disconnect();
      } catch {}
      port = null;
      state.helloDone = false;
      await new Promise((res) => setTimeout(res, 300));
      return { launched: !!r.ok, state: await refresh() };
    }
    case 'unlock':
      return send({ type: 'unlock', method: msg.method }, msg.method === 'hello' ? 70000 : TIMEOUT_MS).then((r) => {
        state.vault = r.vault ?? state.vault;
        setBadge();
        return r;
      });
    case 'lock':
      return send({ type: 'lock' }).then((r) => {
        state.vault.unlocked = false;
        setBadge();
        return r;
      });
    case 'match': {
      const url = pageUrl;
      const r = await send({ type: 'match', url });
      const mem = tabId != null ? tabMemory.get(tabId) : null;
      const prefer = mem && mem.site === siteKey(url) && Date.now() - mem.at < 3 * 60 * 1000 ? mem.id : null;
      return { entries: r.entries, site: r.site, prefer, vault: state.vault };
    }
    case 'fill': {
      const url = pageUrl;
      const r = await send({ type: 'fill', entry: msg.id, url });
      if (tabId != null) tabMemory.set(tabId, { id: msg.id, site: siteKey(url), at: Date.now(), username: tabMemory.get(tabId)?.username });
      return { username: r.username, email: r.email, password: r.password, totp: r.totp };
    }
    case 'search':
      return send({ type: 'search', q: msg.q ?? '' });
    case 'copy':
      return send({ type: 'copy', entry: msg.id, field: msg.field });
    case 'generate':
      return send({ type: 'generate', length: msg.length ?? 20 });
    case 'remember-username':
      if (tabId != null) tabMemory.set(tabId, { ...(tabMemory.get(tabId) ?? {}), site: siteKey(frameUrl), username: msg.username, at: Date.now() });
      return {};
    case 'captured': {
      // A form was submitted with a password: ask the app whether it is new.
      if (!state.offerSave || !msg.password) return {};
      const neverSave = (await chrome.storage.local.get('neverSave')).neverSave ?? [];
      if (neverSave.includes(siteKey(frameUrl))) return {};
      const username = msg.username || (tabId != null ? tabMemory.get(tabId)?.username : '') || '';
      let status = 'new';
      try {
        status = (await send({ type: 'check', url: frameUrl, username, password: msg.password })).status;
      } catch (e) {
        if (e.code === 'locked') status = 'locked';
        else return {};
      }
      if (status === 'same' || status === 'ignore') return {};
      if (tabId != null) {
        saveOffers.set(tabId, { url: frameUrl, username, password: msg.password, status, at: Date.now() });
        // The top frame shows the question (the form may sit in an iframe).
        chrome.tabs.sendMessage(tabId, { kind: 'offer-ready' }, { frameId: 0 }).catch(() => {});
      }
      return { offered: true };
    }
    case 'pending-offer': {
      const o = tabId != null ? saveOffers.get(tabId) : null;
      if (!o || Date.now() - o.at > 2 * 60 * 1000) return null;
      return { site: new URL(o.url).hostname, username: o.username, status: o.status, locked: o.status === 'locked' };
    }
    case 'offer-answer': {
      const o = tabId != null ? saveOffers.get(tabId) : null;
      if (tabId != null) saveOffers.delete(tabId);
      if (!o) return {};
      if (msg.answer === 'never') {
        const neverSave = (await chrome.storage.local.get('neverSave')).neverSave ?? [];
        await chrome.storage.local.set({ neverSave: [...new Set([...neverSave, siteKey(o.url)])] });
        return {};
      }
      if (msg.answer !== 'save') return {};
      return send({ type: 'save', url: o.url, username: o.username, password: o.password });
    }
    case 'open-popup':
      // The toolbar popup when the browser allows it, otherwise the same page in a tab.
      return chrome.action.openPopup().then(
        () => ({}),
        () => chrome.tabs.create({ url: chrome.runtime.getURL(`popup.html${tabId != null ? `?tab=${tabId}` : ''}`) }).then(() => ({})),
      );
    default:
      throw new Error('unknown request');
  }
}

chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {
  handle(msg, sender).then(
    (r) => sendResponse({ ok: true, data: r }),
    (e) => sendResponse({ ok: false, code: e.code ?? 'error', error: e.message }),
  );
  return true;
});

chrome.tabs.onRemoved.addListener((tabId) => {
  tabMemory.delete(tabId);
  saveOffers.delete(tabId);
});

chrome.commands.onCommand.addListener(async (command) => {
  if (command !== 'fill-login') return;
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  if (tab?.id != null) chrome.tabs.sendMessage(tab.id, { kind: 'fill-best' }).catch(() => {});
});
