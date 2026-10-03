// Phone companion server, power actions and screen-sharing bridges.

import type { AdbDevice, Device, InboxItem, MonitorInfo, PairingInfo, PendingPower, PowerAction, Preset, ScrcpyOptions, ScrcpyStatus, ServerStatus, SunshineStatus, ViewerInfo } from '@shared/types';
import { emit } from './bus';
import { audit, flags, onSettingsChange, settings } from './core';
import { fakeQr } from './art';
import { DAY, MB, NOW } from './rng';

const FINGERPRINT = 'A3:9F:1C:7E:42:D8:0B:65:F1:2A:9C:44:E7:13:B8:5D:60:2F:CA:91:7B:E4:38:0D:56:AF:C2:19:8E:73:D4:0A';

let running = false;
let pairingOpen = false;
let pairTimer: ReturnType<typeof setTimeout> | null = null;
let viewers: ViewerInfo[] = [];
const demoTimers: ReturnType<typeof setTimeout>[] = [];

const devices: Device[] = [
  { id: 'dev-pixel', name: 'Pixel 8 Pro', userAgent: 'Mozilla/5.0 (Linux; Android 15; Pixel 8 Pro) AppleWebKit/537.36 Chrome/129.0 Mobile Safari/537.36', created: NOW - Math.round(6.1 * DAY), lastSeen: NOW - 3 * 3600, lastIp: '192.168.1.57', revoked: false },
  { id: 'dev-iphone', name: 'iPhone 15', userAgent: 'Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 Version/18.0 Mobile/15E148 Safari/604.1', created: NOW - Math.round(4.9 * DAY), lastSeen: NOW - Math.round(1.8 * DAY), lastIp: '192.168.1.63', revoked: false },
];
const inbox: InboxItem[] = [];

export const monitors: MonitorInfo[] = [
  { index: 0, name: 'DELL U2723QE', x: 0, y: 0, width: 3840, height: 2160, primary: true },
  { index: 1, name: 'LG 27GL850', x: 3840, y: 0, width: 2560, height: 1440, primary: false },
];

export const presets: Preset[] = [
  { id: 'saver', label: 'Data saver · 720p', maxWidth: 1280, quality: 55, fps: 30 },
  { id: 'balanced', label: 'Balanced · 1080p', maxWidth: 1920, quality: 70, fps: 60 },
  { id: 'sharp', label: 'Sharp · 1440p', maxWidth: 2560, quality: 78, fps: 60 },
  { id: 'max', label: 'Maximum · up to 4K', maxWidth: 3840, quality: 85, fps: 60 },
];

function urls(): string[] {
  const r = settings.remote;
  const scheme = r.tls ? 'https' : 'http';
  if (r.bind === 'localhost') return [`${scheme}://localhost:${r.port}`];
  const out = [`${scheme}://192.168.1.42:${r.port}`];
  if (r.allowTailscale) out.push(`${scheme}://100.101.7.12:${r.port}`);
  return out;
}

export function status(): ServerStatus {
  if (!running) return { running: false, port: 0, tls: false, bind: 'lan', urls: [], fingerprint: null, error: null, viewers: [], pairingOpen: false };
  return { running: true, port: settings.remote.port, tls: settings.remote.tls, bind: settings.remote.bind, urls: urls(), fingerprint: settings.remote.tls ? FINGERPRINT : null, error: null, viewers: viewers.map((v) => ({ ...v })), pairingOpen };
}

const pushStatus = () => emit('remote:status', status());
const pushDevices = () => emit('remote:devices', devices.map((d) => ({ ...d })));
const pushViewers = () => emit('screen:viewers', viewers.map((v) => ({ ...v })));

function simulateUpload(name: string, size: number, device: string, delay: number) {
  const id = `up-${Date.now().toString(36)}-${Math.round(Math.random() * 1e4)}`;
  demoTimers.push(
    setTimeout(() => {
      if (!running) return;
      emit('transfer:started', { id, direction: 'upload', name, size, device });
      let done = 0;
      const step = size / 18;
      const t = setInterval(() => {
        if (!running) {
          clearInterval(t);
          emit('transfer:cancelled', { id, direction: 'upload' });
          return;
        }
        done = Math.min(size, done + step * (0.7 + Math.random() * 0.6));
        if (done >= size) {
          clearInterval(t);
          const path = `${settings.remote.incomingDir ?? 'C:\\Users\\Alex\\Downloads\\OmniHub'}\\${name}`;
          emit('transfer:done', { id, direction: 'upload', name, path, size, sha256: 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855', device });
          audit(device, 'files.upload', `${name} → ${path}`);
        } else emit('transfer:progress', { id, direction: 'upload', name, done: Math.round(done), size, device });
      }, 300);
    }, delay),
  );
}

function simulateDownload(item: InboxItem, device: string, delay: number) {
  const id = `down-${item.id}`;
  demoTimers.push(
    setTimeout(() => {
      if (!running) return;
      emit('transfer:started', { id, direction: 'download', name: item.name, size: item.size, device });
      let done = 0;
      const t = setInterval(() => {
        done = Math.min(item.size, done + item.size / 14);
        if (done >= item.size) {
          clearInterval(t);
          emit('transfer:done', { id, direction: 'download', name: item.name, size: item.size, device });
          audit(device, 'files.download', item.name);
        } else emit('transfer:progress', { id, direction: 'download', name: item.name, done: Math.round(done), size: item.size, device });
      }, 280);
    }, delay),
  );
}

export function start(): ServerStatus {
  if (running) return status();
  running = true;
  pushStatus();
  audit('desktop', 'remote.start', `Listening on port ${settings.remote.port} (${settings.remote.tls ? 'HTTPS' : 'HTTP'})`);
  // A paired phone shows up, starts watching the screen and sends a photo.
  demoTimers.push(
    setTimeout(() => {
      if (!running) return;
      const pixel = devices.find((d) => d.id === 'dev-pixel');
      if (pixel) pixel.lastSeen = Math.floor(Date.now() / 1000);
      pushDevices();
    }, 1800),
  );
  demoTimers.push(
    setTimeout(() => {
      if (!running || !settings.remote.allowScreen) return;
      viewers = [{ id: 'view-1', device: 'Pixel 8 Pro', monitor: monitors[0], since: Math.floor(Date.now() / 1000), controlling: false }];
      pushViewers();
      pushStatus();
      audit('Pixel 8 Pro', 'screen.view', 'Started watching DELL U2723QE');
    }, 3500),
  );
  simulateUpload('PXL_20261003_184522.jpg', 4.6 * MB, 'Pixel 8 Pro', 5200);
  simulateUpload('Screen_Recording_20261003.mp4', 182 * MB, 'Pixel 8 Pro', 6000);
  return status();
}

export function stop(): void {
  if (!running) return;
  running = false;
  pairingOpen = false;
  viewers = [];
  demoTimers.splice(0).forEach(clearTimeout);
  if (pairTimer) clearTimeout(pairTimer);
  pushViewers();
  pushStatus();
  audit('desktop', 'remote.stop', 'Phone companion stopped');
}

onSettingsChange((before, after) => {
  const r0 = before.remote;
  const r1 = after.remote;
  if (r0.enabled !== r1.enabled) {
    if (r1.enabled) start();
    else stop();
  } else if (running && (r0.port !== r1.port || r0.tls !== r1.tls || r0.bind !== r1.bind || r0.allowTailscale !== r1.allowTailscale)) {
    pushStatus();
  }
  if (r0.allowScreen && !r1.allowScreen && viewers.length) {
    viewers = [];
    pushViewers();
    pushStatus();
  }
});

export function pairBegin(): PairingInfo {
  if (!running) throw new Error('Turn on the phone companion first.');
  const pin = String(Math.floor(100000 + Math.random() * 900000));
  const secret = Array.from(crypto.getRandomValues(new Uint8Array(12)), (b) => b.toString(16).padStart(2, '0')).join('');
  const list = urls().map((u) => `${u}/#pair=${secret}`);
  pairingOpen = true;
  pushStatus();
  if (pairTimer) clearTimeout(pairTimer);
  pairTimer = setTimeout(() => {
    if (!pairingOpen) return;
    pairingOpen = false;
    const now = Math.floor(Date.now() / 1000);
    const dev: Device = { id: `dev-${now.toString(36)}`, name: 'Galaxy S24', userAgent: 'Mozilla/5.0 (Linux; Android 14; SM-S921B) AppleWebKit/537.36 Chrome/129.0 Mobile Safari/537.36', created: now, lastSeen: now, lastIp: '192.168.1.71', revoked: false };
    devices.unshift(dev);
    emit('remote:paired', { ...dev });
    pushDevices();
    pushStatus();
    audit(dev.name, 'pair', `Paired from ${dev.lastIp}`);
  }, 6500);
  return { pin, secret, expiresAt: Math.floor(Date.now() / 1000) + 300, urls: list, qrSvg: fakeQr(list[0]), fingerprint: settings.remote.tls ? FINGERPRINT : null };
}

export function pairCancel(): void {
  pairingOpen = false;
  if (pairTimer) clearTimeout(pairTimer);
  pushStatus();
}

export function deviceList(): Device[] {
  return devices.map((d) => ({ ...d }));
}

function findDevice(id: string): Device {
  const d = devices.find((x) => x.id === id);
  if (!d) throw new Error('device not found');
  return d;
}

export function renameDevice(id: string, name: string): void {
  const d = findDevice(id);
  if (!name.trim()) throw new Error('Name cannot be empty.');
  d.name = name.trim().slice(0, 60);
  pushDevices();
}

export function revokeDevice(id: string): void {
  const d = findDevice(id);
  d.revoked = true;
  viewers = viewers.filter((v) => v.device !== d.name);
  pushDevices();
  pushViewers();
  audit('desktop', 'device.revoke', `Revoked ${d.name}`);
}

export function send(paths: string[], deviceId: string | null): InboxItem[] {
  if (!paths.length) return [];
  const target = deviceId ? findDevice(deviceId) : null;
  const items = paths.map((p, i) => {
    const item: InboxItem = { id: `inbox-${Date.now().toString(36)}-${i}`, name: p.split('\\').pop() ?? p, size: Math.round((p.endsWith('.mp4') ? 640 : 2.4) * MB), created: Math.floor(Date.now() / 1000), deviceId };
    inbox.unshift(item);
    emit('inbox:new', { item, deviceId });
    if (running) simulateDownload(item, target?.name ?? 'Pixel 8 Pro', 900 + i * 400);
    return item;
  });
  audit('desktop', 'files.send', `${items.length} file(s) offered to ${target?.name ?? 'all phones'}`);
  return items;
}

export function inboxList(): InboxItem[] {
  return [...inbox];
}

export function stopViewer(id: string): void {
  viewers = viewers.filter((v) => v.id !== id);
  pushViewers();
  pushStatus();
  audit('desktop', 'screen.stop', 'Stopped a screen viewer');
}

export function stopAllViewers(): void {
  viewers = [];
  pushViewers();
  pushStatus();
}

// ---------- power ----------

let pendingPower: PendingPower | null = null;
let powerTimer: ReturnType<typeof setTimeout> | null = null;

const LABELS: Record<PowerAction, string> = { shutdown: 'Shut down', restart: 'Restart', sleep: 'Sleep', hibernate: 'Hibernate', lock: 'Lock', signOut: 'Sign out', displayOff: 'Turn off display' };

export function powerSchedule(action: PowerAction, delaySeconds: number, requestedBy = 'desktop'): PendingPower {
  if (powerTimer) clearTimeout(powerTimer);
  const p: PendingPower = { id: `pw-${Date.now().toString(36)}`, action, label: LABELS[action], at: Math.floor(Date.now() / 1000) + delaySeconds, requestedBy };
  pendingPower = p;
  emit('power:pending', p);
  powerTimer = setTimeout(() => {
    pendingPower = null;
    emit('power:executed', { action, ok: true, error: null, dryRun: true });
  }, delaySeconds * 1000);
  audit(requestedBy, 'power.schedule', `${p.label} in ${delaySeconds} s`);
  return p;
}

export function powerCancel(): boolean {
  if (!pendingPower) return false;
  if (powerTimer) clearTimeout(powerTimer);
  emit('power:cancelled', { id: pendingPower.id, reason: 'Cancelled on the PC' });
  audit('desktop', 'power.cancel', `${pendingPower.label} cancelled`);
  pendingPower = null;
  return true;
}

export function powerPending(): PendingPower | null {
  return pendingPower;
}

if (flags.power) setTimeout(() => powerSchedule('shutdown', 95, 'Pixel 8 Pro'), 1200);

// ---------- scrcpy & Sunshine ----------

let scrcpyRunning = false;
const adbDevices: AdbDevice[] = [
  { serial: '38151FDJH000T5', state: 'device', model: 'Pixel 8 Pro', wireless: false },
  { serial: '192.168.1.66:5555', state: 'device', model: 'Galaxy Tab S9', wireless: true },
];

export function scrcpyStatus(): ScrcpyStatus {
  const dir = 'C:\\Users\\Alex\\AppData\\Local\\Microsoft\\WinGet\\Packages\\Genymobile.scrcpy_Microsoft.Winget.Source_8wekyb3d8bbwe\\scrcpy-win64-v3.1';
  return { found: true, path: `${dir}\\scrcpy.exe`, version: '3.1', adb: `${dir}\\adb.exe`, devices: adbDevices.map((d) => ({ ...d })), running: scrcpyRunning, installHint: 'winget install --id Genymobile.scrcpy' };
}

export function scrcpyLaunch(o: ScrcpyOptions): void {
  if (!adbDevices.length) throw new Error('No Android device connected. Enable USB debugging and plug the phone in.');
  if (o.serial && !adbDevices.some((d) => d.serial === o.serial)) throw new Error(`Device ${o.serial} is not connected.`);
  scrcpyRunning = true;
  audit('desktop', 'scrcpy.launch', `Mirroring ${adbDevices.find((d) => d.serial === o.serial)?.model ?? adbDevices[0].model} (${o.preset})`);
}

export function scrcpyStop(): void {
  scrcpyRunning = false;
}

export function scrcpyWireless(serial: string): string {
  const d = adbDevices.find((x) => x.serial === serial);
  if (!d) throw new Error(`Device ${serial} is not connected.`);
  if (d.wireless) throw new Error('That device is already connected over Wi-Fi.');
  const addr = '192.168.1.60:5555';
  if (!adbDevices.some((x) => x.serial === addr)) adbDevices.push({ serial: addr, state: 'device', model: d.model, wireless: true });
  return addr;
}

export function scrcpyConnect(addr: string): string {
  if (!/^[\w.-]+:\d{2,5}$/.test(addr.trim())) throw new Error('Use the form 192.168.1.60:5555');
  if (!adbDevices.some((x) => x.serial === addr)) adbDevices.push({ serial: addr.trim(), state: 'device', model: 'Android device', wireless: true });
  return `connected to ${addr.trim()}`;
}

export function scrcpyPair(addr: string, code: string): string {
  if (!/^[\w.-]+:\d{2,5}$/.test(addr.trim())) throw new Error('Use the address shown under “Pair device with pairing code”, e.g. 192.168.1.60:37199');
  if (!/^\d{6}$/.test(code.trim())) throw new Error('Failed: wrong pairing code (it has 6 digits).');
  return `Successfully paired to ${addr.trim()} [guid=adb-38151FDJH000T5-${Math.random().toString(36).slice(2, 8)}]`;
}

export function sunshineStatus(): SunshineStatus {
  return { installed: true, path: 'C:\\Program Files\\Sunshine\\sunshine.exe', running: false, webUi: 'https://localhost:47990' };
}
