// Chats without Windows: OmniHub's own chats with Claude and ChatGPT's
// models, pretend Claude Code and Codex conversations in project folders,
// streamed answers, ideas and a month of usage.

import type { ChatFolder, ChatMessage, ChatSource, ChatSummary, ChatTools, ChatView, NewChatKind, Note, OpenAiSettings, UsagePeriod, UsageReport, UsageTokens } from '@shared/types';
import { emit } from './bus';
import { noteSave, notesList } from './notes';

const NOW = Math.floor(Date.now() / 1000);
const MIN = 60;
const HOUR = 3600;
const DAY = 86_400;
const code = 'C:\\Users\\Kerio\\code';

interface Chat {
  summary: ChatSummary;
  messages: ChatMessage[];
}

const msg = (role: ChatMessage['role'], text: string, ago: number, tools?: string[]): ChatMessage => ({ role, text, at: NOW - ago, ...(tools ? { tools } : {}) });

function chat(s: Partial<ChatSummary> & Pick<ChatSummary, 'source' | 'id' | 'title'>, messages: ChatMessage[]): Chat {
  const last = [...messages].reverse().find((m) => m.role === 'user' || m.role === 'assistant');
  return {
    summary: { preview: last ? last.text.slice(0, 140) : '', updated: messages.length ? messages[messages.length - 1].at : NOW, folder: null, model: null, provider: null, branch: null, running: false, ...s },
    messages,
  };
}

const chats: Chat[] = [
  chat({ source: 'claudeCode', id: '5d5e2c1e-8f39-4c39-9a71-0b6d2a6e9f10', title: 'Speed up the Balatro joker mod loader', folder: `${code}\\balatro-jokers`, model: 'claude-opus-5-5', branch: 'main' }, [
    msg('user', 'The jokers take ages to load when Balatro starts. Can you find out why and fix it?', 42 * MIN),
    msg('assistant', 'Looking at how the mod loads its sprites and localization first.', 41 * MIN),
    msg('tools', '', 41 * MIN, ['Read main.lua', 'Searched for love.graphics.newImage', 'Read jokers/registry.lua']),
    msg('assistant', 'Found it: every joker loads its own copy of the 2× atlas (`assets/2x/jokers.png`, 18 MB) instead of sharing one. With 40 jokers that is 40 decodes at start.\n\nI changed the registry to load the atlas once and give each joker a quad into it.', 38 * MIN),
    msg('tools', '', 37 * MIN, ['Edited registry.lua', 'Edited joker_base.lua', 'Ran lua tests/run.lua']),
    msg('assistant', 'Tests pass (23/23). Start-up time in the test harness went from **4.1 s to 0.3 s**. Want me to also cache the localization strings?', 36 * MIN),
  ]),
  chat({ source: 'omni', id: 'c1', title: 'Weekend in Tokyo on a budget', provider: 'claude', model: 'claude-opus-5-5' }, [
    msg('user', 'Plan a 3-day weekend in Tokyo for under $600, I love games and ramen', 3 * HOUR),
    msg('assistant', 'Here is a plan that keeps you around **$540** without flights:\n\n**Day 1 – Akihabara**\n- Super Potato and Retro Game Camp for old consoles\n- Ramen at Kyushu Jangara (≈ ¥1,100)\n\n**Day 2 – Nakano & Shimokitazawa**\n- Nakano Broadway: four floors of figures and games\n- Fuunji tsukemen near Shinjuku (go before 11:00 to skip the queue)\n\n**Day 3 – Ikebukuro**\n- Pokémon Center Mega Tokyo, Sunshine City\n- Mutekiya ramen\n\nStay in a capsule hotel near Shinjuku (¥4,000/night) and get a 72-hour Tokyo Subway Ticket (¥1,500).', 3 * HOUR - 40),
  ]),
  chat({ source: 'codex', id: '0199a213-81c0-7800-8aa1-bbab2a035a53', title: 'Dark mode for the shop site', folder: `${code}\\kerio-shop`, model: 'gpt-6-codex', branch: 'dev' }, [
    msg('user', 'Add a dark mode toggle that remembers the choice', 5 * HOUR),
    msg('tools', '', 5 * HOUR - 60, ['Ran npm run build', 'Edited theme.ts, Header.tsx, globals.css']),
    msg('assistant', 'Dark mode is in. The toggle is in the header, follows the system setting until you pick one, and saves the choice in localStorage. `npm run build` passes.', 5 * HOUR - 120),
  ]),
  chat({ source: 'omni', id: 'c2', title: 'What changed in GPT-6?', provider: 'openai', model: 'gpt-6-astra' }, [
    msg('user', 'Explain like I am 15 what is different about GPT-6 Astra vs Sol', DAY + 2 * HOUR),
    msg('assistant', 'Think of them as the same family with different budgets:\n\n- **Astra** is the big one: best at hard problems, slower and pricier.\n- **Sol** is close to Astra on most everyday tasks but much cheaper and quicker.\n\nFor homework help or coding questions Sol is usually plenty; Astra is for the really tricky stuff.', DAY + 2 * HOUR - 30),
  ]),
  chat({ source: 'claudeCode', id: '8a7b6c5d-4e3f-4a1b-9c8d-7e6f5a4b3c2d', title: 'OmniHub phone pairing bug', folder: `${code}\\omnihub`, model: 'claude-sonnet-5-5', branch: 'fix/pairing' }, [
    msg('user', 'Pairing fails when the PC has two network adapters', 2 * DAY),
    msg('tools', '', 2 * DAY - 120, ['Read net.rs', 'Ran cargo test -p omnihub-core remote']),
    msg('assistant', 'The QR code picked the first adapter (a Hyper-V switch). I made it prefer the adapter with the default route and added a test with two adapters.', 2 * DAY - 300),
  ]),
  chat({ source: 'codex', id: '0199b000-0000-7000-8000-00000000abcd', title: 'Fix the flaky checkout test', folder: `${code}\\kerio-shop`, model: 'gpt-6-codex', branch: 'main' }, [
    msg('user', 'checkout.spec.ts fails one time out of five on CI', 4 * DAY),
    msg('assistant', 'It waited for a fixed 500 ms before reading the total. I replaced that with waiting for the total to show, and ran it 50 times locally without a failure.', 4 * DAY - 400),
  ]),
];

const live = new Map<string, { prompt: string; text: string; tools: string[]; note: string; started: number; cancel: boolean }>();
const k = (s: ChatSource, id: string) => `${s}:${id}`;

function find(source: ChatSource, id: string): Chat {
  const c = chats.find((x) => x.summary.source === source && x.summary.id === id);
  if (!c) throw 'That conversation isn’t on this PC any more.';
  return c;
}

function copy(c: Chat): ChatSummary {
  return { ...c.summary, running: live.has(k(c.summary.source, c.summary.id)) };
}

const REPLIES: Record<ChatSource, (text: string) => { tools: string[]; text: string }> = {
  omni: (t) => ({ tools: [], text: `Good question! Here's a short answer about “${t.slice(0, 60)}”:\n\n1. Start with the simplest version that works.\n2. Check it on your own PC first.\n3. Save the steps as an idea so you can come back to them.` }),
  claudeCode: () => ({ tools: ['Read main.lua', 'Edited localization/en-us.lua', 'Ran lua tests/run.lua'], text: 'Done. The localization strings are now read once at start-up and cached; tests still pass (23/23).' }),
  codex: () => ({ tools: ['Ran npm test', 'Edited checkout.ts'], text: 'Updated the checkout total to round per line, like the receipts. All 48 tests pass.' }),
};

function stream(c: Chat, text: string) {
  const { source, id } = c.summary;
  const key = k(source, id);
  const reply = REPLIES[source](text);
  const run = { prompt: text, text: '', tools: [] as string[], note: '', started: Math.floor(Date.now() / 1000), cancel: false };
  live.set(key, run);
  emit('chats:changed', { source, id });
  let t = 250;
  if (source !== 'omni') {
    setTimeout(() => {
      run.note = 'Planning the change';
      emit('chats:delta', { source, id, kind: 'note', text: run.note });
    }, t);
    for (const tool of reply.tools) {
      t += 550;
      setTimeout(() => {
        if (run.cancel) return;
        run.tools.push(tool);
        emit('chats:delta', { source, id, kind: 'tool', text: tool });
      }, t);
    }
  }
  const words = reply.text.split(/(?<= )/);
  words.forEach((w, i) =>
    setTimeout(() => {
      if (run.cancel) return;
      run.text += w;
      emit('chats:delta', { source, id, kind: 'text', text: w });
    }, t + 300 + i * 40),
  );
  setTimeout(() => {
    live.delete(key);
    if (run.tools.length) c.messages.push(msg('tools', '', 0, run.tools));
    if (run.text.trim()) c.messages.push(msg('assistant', run.text.trim(), 0));
    if (run.cancel) c.messages.push({ role: 'notice', text: 'Stopped.', at: Math.floor(Date.now() / 1000), tone: 'info' });
    c.summary.updated = Math.floor(Date.now() / 1000);
    c.summary.preview = run.text.slice(0, 140);
    emit('chats:delta', { source, id, kind: 'done', text: '' });
    emit('chats:changed', { source, id });
  }, t + 400 + words.length * 40);
}

function tokens(input: number, output: number, cacheRead: number, cost: number): UsageTokens {
  return { input, output, cacheRead, cacheWrite: Math.round(cacheRead / 20), cost };
}

function period(scale: number, priced: boolean): UsagePeriod {
  const days = Array.from({ length: 30 }, (_, i) => {
    const d = new Date((NOW - (29 - i) * DAY) * 1000);
    const wave = 0.4 + Math.abs(Math.sin(i * 1.7)) * (i % 6 === 0 ? 0.2 : 1);
    const t = tokens(Math.round(9000 * scale * wave), Math.round(42_000 * scale * wave), Math.round(1_900_000 * scale * wave), priced ? +(3.1 * scale * wave).toFixed(2) : 0);
    return { date: d.toISOString().slice(0, 10), ...t };
  }).filter((_, i) => i % 7 !== 3);
  const sum = (list: typeof days): UsageTokens => list.reduce((a, d) => tokens(a.input + d.input, a.output + d.output, a.cacheRead + d.cacheRead, +(a.cost + d.cost).toFixed(2)), tokens(0, 0, 0, 0));
  return { today: sum(days.slice(-1)), week: sum(days.slice(-6)), month: sum(days), days };
}

export const chatsMock = {
  list: (): ChatSummary[] => chats.map(copy).sort((a, b) => b.updated - a.updated),
  view(source: ChatSource, id: string): ChatView {
    const c = find(source, id);
    const l = live.get(k(source, id));
    const elsewhere = source === 'claudeCode' && id.startsWith('8a7b');
    return { summary: copy(c), messages: c.messages.map((m) => ({ ...m })), earlier: false, live: l ? { prompt: l.prompt, text: l.text, tools: [...l.tools], note: l.note, started: l.started } : null, busyElsewhere: elsewhere };
  },
  send(source: ChatSource, id: string, text: string) {
    const c = find(source, id);
    if (!text.trim()) throw 'Type a message first.';
    if (live.has(k(source, id))) throw 'It’s still answering. Wait for it, or press Stop.';
    c.messages.push(msg('user', text.trim(), 0));
    stream(c, text.trim());
  },
  create(kind: NewChatKind, text: string, folder: string | null): ChatSummary {
    if (!text.trim()) throw 'Type a message first.';
    const source: ChatSource = kind === 'claudeCode' ? 'claudeCode' : kind === 'codex' ? 'codex' : 'omni';
    if (source !== 'omni' && !folder) throw 'Pick the project folder it should work in.';
    const id = source === 'omni' ? `c${chats.length + 1}` : crypto.randomUUID();
    const c = chat({ source, id, title: text.trim().split('\n')[0].slice(0, 60), folder, provider: kind === 'claude' ? 'claude' : kind === 'openai' ? 'openai' : null, model: kind === 'openai' ? 'gpt-6-astra' : kind === 'codex' ? 'gpt-6-codex' : 'claude-opus-5-5' }, [msg('user', text.trim(), 0)]);
    chats.push(c);
    stream(c, text.trim());
    return copy(c);
  },
  stop(source: ChatSource, id: string) {
    const l = live.get(k(source, id));
    if (l) l.cancel = true;
  },
  rename(id: string, title: string) {
    find('omni', id).summary.title = title.trim().slice(0, 120);
    emit('chats:changed', { source: 'omni', id });
  },
  remove(id: string) {
    const i = chats.findIndex((c) => c.summary.source === 'omni' && c.summary.id === id);
    if (i >= 0) chats.splice(i, 1);
    emit('chats:changed', { source: 'omni', id });
  },
  folders(): ChatFolder[] {
    const m = new Map<string, ChatFolder>();
    for (const c of chats) {
      const f = c.summary.folder;
      if (!f) continue;
      const cur = m.get(f) ?? { path: f, name: f.split('\\').pop() ?? f, last: 0, claudeCode: false, codex: false };
      cur.last = Math.max(cur.last, c.summary.updated);
      if (c.summary.source === 'claudeCode') cur.claudeCode = true;
      if (c.summary.source === 'codex') cur.codex = true;
      m.set(f, cur);
    }
    return [...m.values()].sort((a, b) => b.last - a.last);
  },
  tools: (): ChatTools => ({
    claudeCode: { installed: true, path: 'C:\\Users\\Kerio\\.local\\bin\\claude.exe', version: '2.1.302 (Claude Code)', hasChats: true },
    codex: { installed: true, path: 'C:\\Users\\Kerio\\AppData\\Roaming\\npm\\codex.cmd', version: 'codex-cli 0.131.0', hasChats: true },
    claudeKey: true,
    openaiKey: openai.hasKey,
  }),
  usage: (): UsageReport => ({
    omni: { claude: period(0.05, true), openai: period(0.03, false), maker: tokens(84_000, 61_000, 2_400_000, 3.42) },
    claudeCode: { found: true, period: period(1, true), window: { start: NOW - 2 * HOUR - 840, end: NOW + 3 * HOUR - 840, tokens: tokens(31_000, 148_000, 6_100_000, 4.87) }, limits: [], limitsAt: null, plan: null },
    codex: {
      found: true,
      period: period(0.4, false),
      window: null,
      limits: [
        { name: '5-hour limit', usedPercent: 38, reached: false, resetsAt: NOW + 2 * HOUR + 600 },
        { name: 'Weekly limit', usedPercent: 61, reached: false, resetsAt: NOW + 3 * DAY },
      ],
      limitsAt: NOW - 25 * MIN,
      plan: 'plus',
    },
  }),
  ideas: (): Note[] => notesList({ kind: 'idea', query: '', tag: null, limit: 300 }),
  saveIdea(text: string, from: string | null): Note {
    if (!text.trim()) throw 'The idea is empty.';
    const title = (text.split('\n').find((l) => l.trim()) ?? 'Idea').replace(/^[#*\-\s]+/, '').slice(0, 80);
    return noteSave({ kind: 'idea', title, body: from ? `${text.trim()}\n\n— from ${from}` : text.trim(), tags: ['chat'], pinned: false, color: null });
  },
};

const openai: OpenAiSettings = { model: 'gpt-6-astra', effort: 'default', web: false, hasKey: true };

export const openaiMock = {
  settings: (): OpenAiSettings => ({ ...openai }),
  update(patch: Partial<OpenAiSettings>): OpenAiSettings {
    Object.assign(openai, { ...patch, hasKey: openai.hasKey });
    return { ...openai };
  },
  setKey(key: string) {
    if (key && !key.startsWith('sk-')) throw 'That doesn’t look like an OpenAI API key (they start with sk-).';
    openai.hasKey = !!key;
  },
  models: (): string[] => (openai.hasKey ? ['gpt-6-astra', 'gpt-6-sol', 'gpt-6-luna', 'gpt-6-codex', 'gpt-5.5', 'o5-mini'] : []),
};
