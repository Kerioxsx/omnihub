// OmniHub Passwords — toolbar popup (also opens as a tab when the browser
// will not open the popup for us; then ?tab= names the page it is for).

const view = document.getElementById('view');
const siteEl = document.getElementById('site');
const lockBtn = document.getElementById('lock');

const params = new URLSearchParams(location.search);
if (params.has('tab')) document.body.classList.add('tab');

let tab = null;
let pairing = false;

function ask(kind, extra = {}) {
  return new Promise((resolve) => {
    chrome.runtime.sendMessage({ kind, ...extra }, (r) => {
      if (chrome.runtime.lastError) resolve({ ok: false, code: 'extension', error: chrome.runtime.lastError.message });
      else resolve(r ?? { ok: false, code: 'extension', error: 'No answer' });
    });
  });
}

function h(tag, props = {}, ...children) {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(props)) {
    if (k === 'class') el.className = v;
    else if (k.startsWith('on')) el.addEventListener(k.slice(2), v);
    else if (v === true) el.setAttribute(k, '');
    else if (v !== false && v != null) el.setAttribute(k, v);
  }
  for (const c of children.flat()) if (c != null) el.append(c);
  return el;
}

function show(...nodes) {
  view.replaceChildren(...nodes);
}

function status(text, kind = '') {
  let s = view.querySelector('.status');
  if (!s) {
    s = h('div', { class: 'status' });
    view.append(s);
  }
  s.className = `status ${kind}`;
  s.textContent = text;
}

async function currentTab() {
  if (params.has('tab')) return chrome.tabs.get(Number(params.get('tab'))).catch(() => null);
  const [t] = await chrome.tabs.query({ active: true, currentWindow: true });
  return t ?? null;
}

function isWebPage(url) {
  return /^https?:\/\//i.test(url ?? '');
}

// ------------------------------------------------------------------ states

function setupNote() {
  return h(
    'div',
    { class: 'note' },
    'In OmniHub:',
    h('ol', {}, h('li', {}, 'Open ', h('b', {}, 'Vault'), ' and unlock it.'), h('li', {}, 'Open ', h('b', {}, 'Browser autofill'), ' and turn it on.'), h('li', {}, 'Come back here and click ', h('b', {}, 'Retry'), '.')),
  );
}

function retryButton(label = 'Retry') {
  return h('button', { class: 'btn', type: 'button', onclick: () => render(true) }, label);
}

function renderError(st) {
  switch (st.error) {
    case 'host-missing':
      return show(
        h('p', {}, h('b', {}, 'OmniHub is not set up for this browser yet.')),
        h('p', { class: 'muted' }, 'The extension talks to the OmniHub app on this PC. Install OmniHub from its GitHub Releases page, then turn on browser autofill.'),
        setupNote(),
        h('div', { class: 'row', style: 'margin-top:12px' }, retryButton()),
      );
    case 'app-not-running':
      return show(
        h('p', {}, h('b', {}, 'OmniHub is not running.')),
        h('p', { class: 'muted' }, 'Start it to fill logins from your vault.'),
        h('div', { class: 'row' },
          h('button', { class: 'btn', type: 'button', onclick: async (e) => {
            e.target.disabled = true;
            status('Starting OmniHub…');
            const r = await ask('launch-app');
            if (r.ok && r.data.launched) render(true);
            else status('OmniHub did not start. Start it from the Start menu.', 'bad');
            e.target.disabled = false;
          } }, 'Start OmniHub'),
          retryButton()),
      );
    case 'disabled':
      return show(h('p', {}, h('b', {}, 'Browser autofill is off.')), h('p', { class: 'muted' }, 'Turn it on in OmniHub → Vault → Browser autofill.'), h('div', { class: 'row' }, retryButton()));
    case 'unknown-extension':
      return show(h('p', {}, h('b', {}, 'This copy of the extension is not the one OmniHub trusts.')), h('p', { class: 'muted' }, 'Load the extension from the folder OmniHub shows in Vault → Browser autofill.'));
    default:
      return show(h('p', {}, h('b', {}, 'Cannot reach OmniHub.')), h('p', { class: 'muted' }, st.errorText || 'Unknown error.'), h('div', { class: 'row' }, retryButton()));
  }
}

function renderPair(st) {
  if (st.pairing) {
    return show(
      h('h2', {}, 'Connect this browser'),
      h('p', {}, 'OmniHub shows a request with this code. Check it matches, then click ', h('b', {}, 'Allow'), ' in OmniHub.'),
      h('div', { class: 'code' }, st.pairing.code),
      h('p', { class: 'muted' }, h('span', { class: 'spinner' }), 'Waiting for OmniHub…'),
    );
  }
  show(
    h('h2', {}, 'Connect this browser'),
    h('p', {}, 'Allow this browser to fill logins from your OmniHub vault. You approve it once in OmniHub, and can remove it there any time.'),
    h('button', { class: 'btn wide', type: 'button', onclick: async () => {
      pairing = true;
      const r = await ask('pair');
      pairing = false;
      if (r.ok) render(true);
      else {
        await render(false);
        status(r.error || 'Pairing failed.', 'bad');
      }
    } }, 'Connect to OmniHub'),
  );
}

function renderLocked(st) {
  const unlock = (method) => async (e) => {
    e.target.disabled = true;
    status(method === 'hello' ? 'Confirm with Windows Hello…' : 'Opening OmniHub…');
    const r = await ask('unlock', { method });
    e.target.disabled = false;
    if (!r.ok) return status(r.error || 'Could not unlock.', 'bad');
    if (r.data.vault?.unlocked) return render(true);
    status('Unlock the vault in the OmniHub window. This popup updates when it is open.');
  };
  show(
    h('p', {}, h('b', {}, st.vault.exists ? 'Your vault is locked.' : 'No vault yet.')),
    st.vault.exists
      ? h('div', { class: 'row' },
          st.vault.helloEnabled ? h('button', { class: 'btn', type: 'button', onclick: unlock('hello') }, 'Unlock with Windows Hello') : null,
          h('button', { class: st.vault.helloEnabled ? 'btn ghost' : 'btn', type: 'button', onclick: unlock('window') }, 'Unlock in OmniHub'))
      : h('p', { class: 'muted' }, 'Create your vault in OmniHub → Vault first.'),
  );
}

function entryRow(e, { canFill }) {
  const copy = (field, label) => h('button', { type: 'button', title: `Copy ${label}`, onclick: async () => {
    const r = await ask('copy', { id: e.id, field });
    status(r.ok ? `${label[0].toUpperCase()}${label.slice(1)} copied — the clipboard clears itself.` : r.error, r.ok ? 'good' : 'bad');
  } }, label === 'password' ? 'Pass' : label === '2FA code' ? 'Code' : 'User');
  return h(
    'div',
    { class: 'entry' },
    h('div', { class: 'avatar' }, (e.title || '?').slice(0, 1).toUpperCase()),
    h('div', { class: 't' }, h('b', {}, e.title || e.url || 'Login'), h('span', {}, e.username || e.email || e.url || '')),
    h('div', { class: 'acts' },
      canFill ? h('button', { class: 'fill', type: 'button', onclick: () => fill(e.id) }, 'Fill') : null,
      e.username || e.email ? copy(e.username ? 'username' : 'email', 'username') : null,
      e.hasPassword ? copy('password', 'password') : null,
      e.hasTotp ? copy('totp', '2FA code') : null),
  );
}

async function fill(id) {
  const send = () => chrome.tabs.sendMessage(tab.id, { kind: 'fill-entry', id });
  try {
    await send();
  } catch {
    // The page was open before the extension was installed: add the filler now.
    try {
      await chrome.scripting.executeScript({ target: { tabId: tab.id, allFrames: true }, files: ['content.js'] });
      await send();
    } catch {
      return status('This page cannot be filled. Use the copy buttons instead.', 'bad');
    }
  }
  if (!params.has('tab')) window.close();
  else status('Filled.', 'good');
}

async function renderUnlocked() {
  lockBtn.hidden = false;
  const nodes = [];
  if (isWebPage(tab?.url)) {
    const r = await ask('match', { url: tab.url, tabId: tab.id });
    const entries = r.ok ? r.data.entries : [];
    nodes.push(h('h2', {}, `Logins for ${r.ok ? r.data.site : new URL(tab.url).hostname}`));
    if (entries.length) nodes.push(...entries.slice(0, 6).map((e) => entryRow(e, { canFill: true })));
    else nodes.push(h('p', { class: 'muted' }, r.ok ? 'Nothing saved for this site yet. OmniHub offers to save logins you type.' : r.error));
  }
  const results = h('div');
  const search = h('input', { type: 'search', placeholder: 'Search your vault', 'aria-label': 'Search your vault' });
  let timer = null;
  search.addEventListener('input', () => {
    clearTimeout(timer);
    timer = setTimeout(async () => {
      const q = search.value.trim();
      if (!q) return results.replaceChildren();
      const r = await ask('search', { q });
      results.replaceChildren(...(r.ok ? r.data.entries.slice(0, 8).map((e) => entryRow(e, { canFill: false })) : [h('p', { class: 'muted' }, r.error)]));
      if (r.ok && !r.data.entries.length) results.replaceChildren(h('p', { class: 'muted' }, 'No matches.'));
    }, 200);
  });
  nodes.push(h('h2', {}, 'Search'), search, results);

  const pw = h('input', { type: 'text', readonly: true, 'aria-label': 'Generated password' });
  const gen = async () => {
    const r = await ask('generate', { length: 20 });
    if (r.ok) pw.value = r.data.password;
  };
  nodes.push(
    h('h2', {}, 'Password generator'),
    h('div', { class: 'gen' }, pw,
      h('button', { class: 'icon', type: 'button', title: 'New password', onclick: gen }, '↻'),
      h('button', { class: 'icon', type: 'button', title: 'Copy', onclick: async () => {
        await navigator.clipboard.writeText(pw.value);
        status('Password copied.', 'good');
      } }, '⧉')),
  );
  show(...nodes);
  gen();
  if (!isWebPage(tab?.url)) search.focus();
}

async function render(refresh = false) {
  const r = await ask('state', { refresh });
  if (!r.ok) return show(h('p', {}, r.error || 'The extension could not start.'));
  const st = r.data;
  lockBtn.hidden = true;
  if (st.error) return renderError(st);
  if (!st.paired) return renderPair(st);
  if (!st.vault.unlocked) return renderLocked(st);
  return renderUnlocked();
}

lockBtn.addEventListener('click', async () => {
  await ask('lock');
  render(true);
});

chrome.runtime.onMessage.addListener((msg) => {
  if (msg?.kind === 'state-changed' && !pairing) render(false);
  if (msg?.kind === 'state-changed' && pairing) ask('state').then((r) => r.ok && r.data.pairing && renderPair(r.data));
});

(async () => {
  tab = await currentTab();
  siteEl.textContent = isWebPage(tab?.url) ? new URL(tab.url).hostname : 'Not a web page';
  render(true);
})();
