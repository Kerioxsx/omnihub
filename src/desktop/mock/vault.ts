// Password vault for the mock backend. No real crypto — the point is the
// state machine (none → unlocked ⇄ locked), throttling and auto-lock.

import type { Entry, EntryInput, EntrySummary, GeneratorOptions, Strength, VaultStatus } from '@shared/types';
import { emit } from './bus';
import { audit, onSettingsChange, settings } from './core';
import { DAY, NOW } from './rng';

const DEMO_PASSWORD = 'correct horse battery';
const PRIMARY = ['google.com', 'gmail.com', 'googlemail.com', 'accounts.google', 'microsoft.com', 'live.com', 'outlook.com', 'hotmail.com', 'msn.com', 'apple.com', 'icloud.com', 'me.com', 'appleid', 'yahoo.com', 'proton.me', 'protonmail.com'];

let exists = false;
let unlocked = false;
let master = '';
let lastTouch = Date.now();
let failures = 0;
let throttleUntil = 0;
let entries: Entry[] = [];

function seedEntries(): Entry[] {
  const e = (id: string, kind: Entry['kind'], title: string, username: string, email: string, password: string, url: string, notes: string, tags: string[], favorite: boolean, ageDays: number): Entry => ({
    id,
    kind,
    title,
    username,
    email,
    password,
    url,
    notes,
    tags,
    favorite,
    created: Math.floor(NOW - (ageDays + 30) * DAY),
    updated: Math.floor(NOW - ageDays * DAY),
    passwordChanged: Math.floor(NOW - ageDays * DAY),
  });
  return [
    e('v-gmail', 'email', 'Gmail (personal)', 'alex.morgan', 'alex.morgan@gmail.com', 'Tr0ub4dor&3-sunrise', 'https://accounts.google.com', 'Recovery phone ends in 42. 2-Step Verification is on.', ['personal'], true, 40),
    e('v-github', 'login', 'GitHub', 'alexm-dev', 'alex.morgan@gmail.com', 'vX7#qL2!pN9@wR4$zK', 'https://github.com/login', 'Recovery codes are in the safe.', ['dev'], true, 12),
    e('v-steam', 'login', 'Steam', 'alexplays', 'alex.morgan@gmail.com', 'steamPass2021!', 'https://store.steampowered.com/login', 'Steam Guard on phone.', ['games'], false, 220),
    e('v-discord', 'login', 'Discord', 'alex#0420', 'alex.morgan@gmail.com', 'q8Kc-2vRm-Lz7X-pT4w', 'https://discord.com/login', '', ['social'], false, 60),
    e('v-netflix', 'login', 'Netflix', '', 'alex.morgan@gmail.com', 'netflix123', 'https://www.netflix.com/login', 'Shared with the family profile.', ['streaming'], false, 410),
    e('v-msa', 'email', 'Microsoft account', '', 'alex.morgan@outlook.com', 'Blue-Kettle-Orbit-73', 'https://login.live.com', 'Used for Windows sign-in and Xbox.', ['personal'], false, 95),
    e('v-wifi', 'wifi', 'Home Wi-Fi (Morgan-5G)', 'Morgan-5G', '', 'lantern-cobalt-meadow-91', '', 'WPA3-Personal. Guest network: Morgan-Guest / see below.', ['home'], false, 300),
    e('v-visa', 'card', 'Visa ending 4242', 'ALEX MORGAN', '', '7391', '', 'Number: 4242 4242 4242 4242\nExpires: 09/29\nCVC: 314', ['finance'], false, 150),
    e('v-router', 'other', 'Router admin', 'admin', '', 'R0uter!Adm1n#2024', 'http://192.168.1.1', 'ASUS RT-AX86U, firmware 3.0.0.6', ['home'], false, 400),
    e('v-ssh', 'note', 'Homelab SSH', 'alex', '', '', '', 'Host nas.local\n  User alex\n  Port 2222\n  IdentityFile ~/.ssh/id_ed25519', ['homelab'], false, 25),
  ];
}

function isPrimary(e: Pick<Entry, 'url' | 'username' | 'email'>): boolean {
  const hay = `${e.url} ${e.username} ${e.email}`.toLowerCase();
  return PRIMARY.some((d) => hay.includes(d));
}

function summary(e: Entry): EntrySummary {
  return {
    id: e.id,
    kind: e.kind,
    title: e.title,
    username: e.username,
    email: e.email,
    url: e.url,
    tags: [...e.tags],
    favorite: e.favorite,
    updated: e.updated,
    hasPassword: e.password.length > 0,
    hasNotes: e.notes.length > 0,
    primaryAccount: isPrimary(e),
    passwordScore: e.password ? strength(e.password).score : 0,
  };
}

function requireOpen() {
  if (!exists) throw new Error('No vault yet. Create one first.');
  if (!unlocked) throw new Error('The vault is locked.');
  lastTouch = Date.now();
}

export function lock(reason: string) {
  if (!unlocked) return;
  unlocked = false;
  emit('vault:locked', { reason });
}

// Auto-lock timer.
setInterval(() => {
  if (unlocked && Date.now() - lastTouch > settings.vault.autoLockMinutes * 60_000) lock('idle');
}, 1000);
onSettingsChange((before, after) => {
  if (before.vault.autoLockMinutes !== after.vault.autoLockMinutes) lastTouch = Date.now();
});

export function status(): VaultStatus {
  const idle = (Date.now() - lastTouch) / 1000;
  return {
    exists,
    unlocked,
    entries: exists ? entries.length : 0,
    dpapi: true,
    helloAvailable: true,
    helloEnabled: settings.vault.helloEnabled,
    autoLockMinutes: settings.vault.autoLockMinutes,
    locksIn: unlocked ? Math.max(0, Math.round(settings.vault.autoLockMinutes * 60 - idle)) : null,
    retryAfter: Math.max(0, Math.ceil((throttleUntil - Date.now()) / 1000)),
  };
}

export function create(password: string): void {
  if (exists) throw new Error('A vault already exists.');
  const s = strength(password);
  if (password.length < 8 || s.score < 2) throw new Error('Choose a stronger master password (at least “Fair”).');
  exists = true;
  unlocked = true;
  master = password;
  lastTouch = Date.now();
  entries = seedEntries();
  audit('desktop', 'vault.create', 'Vault created (demo entries imported)');
  emit('vault:unlocked', {});
}

export function unlock(password: string): void {
  if (!exists) throw new Error('No vault yet. Create one first.');
  if (Date.now() < throttleUntil) throw new Error(`Too many attempts. Try again in ${Math.ceil((throttleUntil - Date.now()) / 1000)} s.`);
  if (password !== master && password !== DEMO_PASSWORD) {
    failures++;
    if (failures >= 3) throttleUntil = Date.now() + Math.min(60, 5 * 2 ** (failures - 3)) * 1000;
    audit('desktop', 'vault.unlock', 'Wrong master password', false);
    throw new Error('Wrong master password.');
  }
  failures = 0;
  throttleUntil = 0;
  unlocked = true;
  lastTouch = Date.now();
  audit('desktop', 'vault.unlock', 'Unlocked with the master password');
  emit('vault:unlocked', {});
}

export function helloUnlock(): void {
  if (!settings.vault.helloEnabled) throw new Error('Windows Hello is not enabled for this vault.');
  unlocked = true;
  lastTouch = Date.now();
  audit('desktop', 'vault.unlock', 'Unlocked with Windows Hello');
  emit('vault:unlocked', { via: 'hello' });
}

export function touch(): void {
  if (unlocked) lastTouch = Date.now();
}

export function list(): EntrySummary[] {
  requireOpen();
  return entries.map(summary).sort((a, b) => Number(b.favorite) - Number(a.favorite) || a.title.localeCompare(b.title));
}

export function get(id: string): Entry {
  requireOpen();
  const e = entries.find((x) => x.id === id);
  if (!e) throw new Error('entry not found');
  return { ...e, tags: [...e.tags] };
}

export function save(input: EntryInput): EntrySummary {
  requireOpen();
  const now = Math.floor(Date.now() / 1000);
  if (!input.title.trim()) throw new Error('Give the entry a title.');
  if (input.id) {
    const e = entries.find((x) => x.id === input.id);
    if (!e) throw new Error('entry not found');
    const pwChanged = input.password != null && input.password !== e.password;
    Object.assign(e, { kind: input.kind, title: input.title, username: input.username, email: input.email, url: input.url, tags: [...input.tags], favorite: input.favorite, updated: now });
    if (input.notes != null) e.notes = input.notes;
    if (pwChanged) {
      e.password = input.password ?? '';
      e.passwordChanged = now;
    }
    return summary(e);
  }
  const e: Entry = { id: `v-${Date.now().toString(36)}`, kind: input.kind, title: input.title, username: input.username, email: input.email, password: input.password ?? '', url: input.url, notes: input.notes ?? '', tags: [...input.tags], favorite: input.favorite, created: now, updated: now, passwordChanged: now };
  entries.push(e);
  return summary(e);
}

export function remove(id: string): void {
  requireOpen();
  entries = entries.filter((e) => e.id !== id);
}

export function copy(id: string, field: string): void {
  const e = get(id);
  const value = field === 'password' ? e.password : field === 'username' ? e.username : field === 'email' ? e.email : field === 'url' ? e.url : e.notes;
  if (!value) throw new Error(`This entry has no ${field}.`);
  if (field === 'password') audit('desktop', 'vault.copy', `Copied the password of “${e.title}”`);
}

export function changePassword(oldPassword: string, newPassword: string): void {
  requireOpen();
  if (oldPassword !== master && oldPassword !== DEMO_PASSWORD) throw new Error('The current master password is wrong.');
  if (strength(newPassword).score < 2) throw new Error('Choose a stronger new master password.');
  master = newPassword;
  audit('desktop', 'vault.change-password', 'Master password changed');
}

export function helloEnable(): void {
  requireOpen();
  settings.vault.helloEnabled = true;
  emit('settings:changed', structuredClone(settings));
}

export function helloDisable(): void {
  settings.vault.helloEnabled = false;
  emit('settings:changed', structuredClone(settings));
}

export function importBackup(password: string): number {
  requireOpen();
  if (password.length < 4) throw new Error('Wrong backup password.');
  return 3;
}

// ---------- generator & strength ----------

export function generate(o: GeneratorOptions): string {
  const amb = /[Il1O0o]/g;
  const sets = [o.lowercase && 'abcdefghijklmnopqrstuvwxyz', o.uppercase && 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', o.digits && '0123456789', o.symbols && '!@#$%^&*()-_=+[]{};:,.?/'].filter((s): s is string => !!s).map((s) => (o.avoidAmbiguous ? s.replace(amb, '') : s));
  if (!sets.length) throw new Error('Pick at least one character set.');
  const len = Math.max(4, Math.min(128, Math.round(o.length)));
  const rand = (n: number) => {
    const buf = new Uint32Array(1);
    crypto.getRandomValues(buf);
    return buf[0] % n;
  };
  const all = sets.join('');
  const chars = sets.map((s) => s[rand(s.length)]);
  while (chars.length < len) chars.push(all[rand(all.length)]);
  for (let i = chars.length - 1; i > 0; i--) {
    const j = rand(i + 1);
    [chars[i], chars[j]] = [chars[j], chars[i]];
  }
  return chars.join('');
}

const COMMON = ['password', '123456', 'qwerty', 'letmein', 'welcome', 'admin', 'iloveyou', 'monkey', 'dragon', 'netflix', 'steam', 'abc123'];

export function strength(pw: string): Strength {
  const feedback: string[] = [];
  if (!pw) return { score: 0, bits: 0, label: 'Empty', feedback: ['Type a password.'] };
  const words = pw.trim().split(/[\s\-_.]+/).filter((w) => w.length >= 3);
  let bits: number;
  if (words.length >= 3 && /^[a-z\s\-_.]+$/i.test(pw)) {
    bits = words.length * 12.9;
    if (words.length < 5) feedback.push('Add another word or two for a stronger passphrase.');
  } else {
    let pool = 0;
    if (/[a-z]/.test(pw)) pool += 26;
    if (/[A-Z]/.test(pw)) pool += 26;
    if (/\d/.test(pw)) pool += 10;
    if (/[^a-zA-Z0-9]/.test(pw)) pool += 33;
    bits = pw.length * Math.log2(Math.max(pool, 2));
    if (/(.)\1{2,}/.test(pw)) {
      bits *= 0.7;
      feedback.push('Avoid repeated characters.');
    }
    if (/(?:0123|1234|2345|3456|abcd|qwer|asdf)/i.test(pw)) {
      bits *= 0.6;
      feedback.push('Avoid sequences like 1234 or qwer.');
    }
    if (pw.length < 12) feedback.push('Use at least 12 characters.');
    if (pool <= 36) feedback.push('Mix in upper case, digits or symbols.');
  }
  if (COMMON.some((c) => pw.toLowerCase().includes(c))) {
    bits = Math.min(bits, 18);
    feedback.unshift('Contains a very common password.');
  }
  if (/\b(19|20)\d{2}\b/.test(pw)) feedback.push('Years are easy to guess.');
  const score = (bits < 28 ? 0 : bits < 36 ? 1 : bits < 60 ? 2 : bits < 80 ? 3 : 4) as Strength['score'];
  return { score, bits: Math.round(bits), label: ['Very weak', 'Weak', 'Fair', 'Strong', 'Very strong'][score], feedback };
}
