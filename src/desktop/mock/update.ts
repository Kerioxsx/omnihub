// Updates for the mock backend: a pretend 0.2.4 that downloads and
// "installs" (the page just reloads at the end).

import type { Release, UpdateInfo, UpdateState } from '@shared/types';
import { emit } from './bus';

const CURRENT = '0.2.3';
const RELEASE: Release = {
  version: '0.2.4',
  notes: '- Phones connect on every Windows PC (the companion now listens on IPv4 and IPv6)\n- Updates install from inside the app\n- Fast scans can stop asking for administrator approval',
  pageUrl: 'https://github.com/Kerioxsx/omnihub/releases/tag/v0.2.4',
  publishedAt: new Date().toISOString(),
  setup: { name: 'OmniHub_0.2.4_x64-setup.exe', url: 'https://github.com/Kerioxsx/omnihub/releases/download/v0.2.4/OmniHub_0.2.4_x64-setup.exe', size: 9_300_000 },
  msi: null,
  sumsUrl: 'https://github.com/Kerioxsx/omnihub/releases/download/v0.2.4/SHA256SUMS.txt',
};

const available = new URLSearchParams(window.location.search).get('update') !== 'none';
let state: UpdateState = available ? { state: 'available', release: RELEASE } : { state: 'upToDate', checkedAt: Math.floor(Date.now() / 1000) };

function set(s: UpdateState) {
  state = s;
  emit('update:state', s);
}

export function info(): UpdateInfo {
  return { current: CURRENT, state };
}

export async function check(): Promise<UpdateInfo> {
  set({ state: 'checking' });
  await new Promise((r) => setTimeout(r, 900));
  set(available ? { state: 'available', release: RELEASE } : { state: 'upToDate', checkedAt: Math.floor(Date.now() / 1000) });
  return info();
}

export async function install(): Promise<void> {
  const total = RELEASE.setup?.size ?? 1;
  for (let done = 0; done < total; done += total / 12) {
    set({ state: 'downloading', version: RELEASE.version, done: Math.round(done), total });
    await new Promise((r) => setTimeout(r, 120));
  }
  set({ state: 'installing', version: RELEASE.version });
}
