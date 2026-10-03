// HTTP/WebSocket client for the OmniHub companion server
// (crates/omnihub-core/src/remote/api.rs). The phone app is served by that
// server, so every URL is same-origin.

import type { Device, Note, NoteKind, PendingPower, PowerAction, InboxItem, EntrySummary, MonitorInfo, Preset } from '@shared/types';

const TOKEN_KEY = 'omnihub.token';
const NAME_KEY = 'omnihub.deviceName';

export class ApiError extends Error {
  constructor(public status: number, message: string) {
    super(message);
  }
}

export function getToken(): string | null {
  try {
    return localStorage.getItem(TOKEN_KEY);
  } catch {
    return null;
  }
}

export function setToken(token: string | null) {
  try {
    if (token) localStorage.setItem(TOKEN_KEY, token);
    else localStorage.removeItem(TOKEN_KEY);
  } catch {
    /* private mode */
  }
}

export function savedDeviceName(): string {
  try {
    return localStorage.getItem(NAME_KEY) ?? guessDeviceName();
  } catch {
    return guessDeviceName();
  }
}

export function guessDeviceName(): string {
  const ua = navigator.userAgent;
  const android = /Android [\d.]+; ([^;)]+)/.exec(ua);
  if (android) return android[1].replace(/ Build.*/, '').trim();
  if (/iPhone/.test(ua)) return 'iPhone';
  if (/iPad/.test(ua)) return 'iPad';
  return 'Phone';
}

export async function request<T>(method: string, path: string, body?: unknown, headers: Record<string, string> = {}): Promise<T> {
  const token = getToken();
  const h: Record<string, string> = { ...headers };
  if (token) h.authorization = `Bearer ${token}`;
  let payload: BodyInit | undefined;
  if (body instanceof Blob || body instanceof ArrayBuffer || body instanceof Uint8Array) {
    payload = body as BodyInit;
  } else if (body !== undefined) {
    h['content-type'] = 'application/json';
    payload = JSON.stringify(body);
  }
  const res = await fetch(path, { method, headers: h, body: payload });
  if (res.status === 204) return undefined as T;
  const text = await res.text();
  let data: any;
  try {
    data = text ? JSON.parse(text) : undefined;
  } catch {
    data = res.ok ? text : { error: text.slice(0, 200) || res.statusText };
  }
  if (!res.ok) {
    const message: string = data?.error ?? res.statusText;
    if (res.status === 401 && isDeviceAuthFailure(path, message)) {
      window.dispatchEvent(new CustomEvent('omnihub:unauthorized'));
    }
    throw new ApiError(res.status, message);
  }
  return data as T;
}

/**
 * The server answers 401 both for an unknown/revoked device token and for an
 * expired *vault session* ("unlock the vault first" / "the vault locked").
 * Only the former means this phone is no longer paired.
 */
export function isDeviceAuthFailure(path: string, message: string): boolean {
  if (path === '/api/pair') return false;
  if (path.startsWith('/api/vault/')) return /pair this phone|no longer paired/i.test(message);
  return true;
}

export interface ServerInfo {
  name: string;
  app: string;
  version: string;
  platform: string;
  tls: boolean;
  paired: boolean;
  pairingOpen: boolean;
  features: { uploads: boolean; power: boolean; screen: boolean; control: boolean; apps: boolean; notes: boolean; vault: boolean };
}

export interface FsEntry {
  name: string;
  path: string;
  isDir: boolean;
  size: number | null;
  modified: number;
  kind: 'folder' | 'image' | 'video' | 'audio' | 'document' | 'archive' | 'app' | 'file';
  hidden: boolean;
}

export interface Status {
  hostname: string;
  os: string;
  uptime: number;
  cpu: number;
  memory: { used: number; total: number };
  disks: { root: string; label: string; total: number; free: number }[];
  pendingPower: PendingPower | null;
  vaultUnlocked: boolean;
}

export const client = {
  info: () => request<ServerInfo>('GET', '/api/info'),
  pair: async (opts: { secret?: string; pin?: string; deviceName: string }) => {
    const r = await request<{ token: string; deviceId: string; serverName: string }>('POST', '/api/pair', opts);
    setToken(r.token);
    try {
      localStorage.setItem(NAME_KEY, opts.deviceName);
    } catch {
      /* ignore */
    }
    return r;
  },
  unpair: async () => {
    try {
      await request<void>('POST', '/api/unpair');
    } finally {
      setToken(null);
    }
  },
  status: () => request<Status>('GET', '/api/status'),
  /** The paired device record of this phone. */
  me: () => request<Device>('GET', '/api/me'),

  roots: () => request<{ roots: { name: string; path: string }[] }>('GET', '/api/fs/roots'),
  list: (path: string) => request<{ path: string; parent: string | null; entries: FsEntry[] }>('GET', `/api/fs/list?path=${encodeURIComponent(path)}`),
  /** Object URL of an image thumbnail (revoke when done). */
  thumbnail: async (path: string, size = 256): Promise<string> => {
    const res = await fetch(`/api/fs/thumb?size=${size}&path=${encodeURIComponent(path)}`, { headers: { authorization: `Bearer ${getToken()}` } });
    if (!res.ok) throw new ApiError(res.status, 'no preview');
    return URL.createObjectURL(await res.blob());
  },
  /** Start a download of a PC file through the browser's own download manager. */
  download: async (path: string, inline = false) => {
    const t = await request<{ url: string; name: string; size: number }>('POST', '/api/files/ticket', { path });
    triggerDownload(t.url + (inline ? '?inline=1' : ''), t.name, inline);
    return t;
  },
  inbox: () => request<{ items: InboxItem[] }>('GET', '/api/inbox'),
  receive: async (item: InboxItem) => {
    const t = await request<{ url: string; name: string }>('POST', `/api/inbox/${item.id}/ticket`);
    triggerDownload(t.url, t.name, false);
  },
  dismiss: (id: string) => request<void>('DELETE', `/api/inbox/${id}`),

  power: () => request<{ enabled: boolean; actions: { action: PowerAction; label: string; destructive: boolean }[]; pending: PendingPower | null; delaySeconds: number }>('GET', '/api/power'),
  requestPower: (action: PowerAction) => request<{ pending: PendingPower }>('POST', '/api/power', { action, confirm: true }),
  cancelPower: () => request<{ cancelled: boolean }>('POST', '/api/power/cancel'),

  apps: () => request<{ apps: { id: string; name: string; publisher: string; source: string }[] }>('GET', '/api/apps'),
  appIcon: (id: string) => request<{ icon: string | null }>('GET', `/api/apps/${id}/icon`),
  launch: (id: string) => request<{ launched: string }>('POST', `/api/apps/${id}/launch`),

  notes: (kind?: NoteKind, q = '') => request<{ notes: Note[]; claudeFolder: boolean }>('GET', `/api/notes?${kind ? `kind=${kind}&` : ''}q=${encodeURIComponent(q)}`),
  note: (id: string) => request<Note>('GET', `/api/notes/${id}`),
  createNote: (n: { title: string; body: string; kind: NoteKind; tags?: string[]; sendToClaude?: boolean }) => request<Note>('POST', '/api/notes', n),

  screenInfo: () => request<{ monitors: MonitorInfo[]; presets: Preset[]; allowControl: boolean; defaultPreset: string }>('GET', '/api/screen/info'),

  vaultUnlock: (password: string) => request<{ session: string; idleSeconds: number }>('POST', '/api/vault/session', { password }),
  vaultEntries: (session: string) => request<{ entries: EntrySummary[] }>('GET', '/api/vault/entries', undefined, { 'x-vault-session': session }),
  vaultReveal: (session: string, id: string, field: 'password' | 'username' | 'email' | 'notes') =>
    request<{ value: string }>('POST', '/api/vault/reveal', { id, field }, { 'x-vault-session': session }),
  vaultLock: (session: string) => request<void>('DELETE', '/api/vault/session', undefined, { 'x-vault-session': session }),

  ticket: (purpose: 'socket' | 'screen') => request<{ ticket: string }>('POST', '/api/ticket', { purpose }),
};

function triggerDownload(url: string, name: string, inline: boolean) {
  const a = document.createElement('a');
  a.href = url;
  if (!inline) a.download = name;
  else a.target = '_blank';
  a.rel = 'noopener';
  document.body.appendChild(a);
  a.click();
  a.remove();
}

function wsUrl(path: string) {
  const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
  return `${proto}//${location.host}${path}`;
}

export type SocketStatus = 'connecting' | 'open' | 'closed';

/**
 * Live events (transfer:*, inbox:*, power:*, notes:*, screen:*). Reconnects
 * automatically (exponential backoff, immediately when the network comes
 * back or the page becomes visible again). `onStatus` reports the state.
 */
export function connectEvents(onEvent: (topic: string, payload: any) => void, onStatus?: (s: SocketStatus) => void): () => void {
  let ws: WebSocket | null = null;
  let closed = false;
  let retry = 1000;
  let timer: ReturnType<typeof setTimeout> | null = null;
  let opening = false;
  const status = (s: SocketStatus) => {
    if (!closed) onStatus?.(s);
  };
  const schedule = (delay: number) => {
    if (closed) return;
    if (timer) clearTimeout(timer);
    timer = setTimeout(open, delay);
  };
  const open = async () => {
    timer = null;
    if (closed || opening || (ws && ws.readyState <= WebSocket.OPEN)) return;
    opening = true;
    status('connecting');
    try {
      const { ticket } = await client.ticket('socket');
      if (closed) return;
      const sock = new WebSocket(wsUrl(`/api/ws?ticket=${encodeURIComponent(ticket)}`));
      ws = sock;
      sock.onopen = () => {
        retry = 1000;
        status('open');
      };
      sock.onmessage = (m) => {
        try {
          const e = JSON.parse(m.data);
          onEvent(e.topic, e.payload);
        } catch {
          /* ignore */
        }
      };
      sock.onclose = () => {
        if (ws !== sock) return;
        ws = null;
        status('closed');
        schedule((retry = Math.min(retry * 2, 15000)));
      };
    } catch {
      status('closed');
      schedule((retry = Math.min(retry * 2, 15000)));
    } finally {
      opening = false;
    }
  };
  const kick = () => {
    if (closed || document.visibilityState === 'hidden') return;
    if (!ws || ws.readyState > WebSocket.OPEN) {
      retry = 1000;
      schedule(0);
    }
  };
  window.addEventListener('online', kick);
  document.addEventListener('visibilitychange', kick);
  open();
  return () => {
    closed = true;
    if (timer) clearTimeout(timer);
    window.removeEventListener('online', kick);
    document.removeEventListener('visibilitychange', kick);
    const s = ws;
    ws = null;
    s?.close();
  };
}

// ---------- uploads (resumable, chunked, CRC-32 per chunk) ----------

const CRC_TABLE = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();

export function crc32(data: Uint8Array): number {
  let c = 0xffffffff;
  for (let i = 0; i < data.length; i++) c = CRC_TABLE[(c ^ data[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

export const CHUNK_SIZE = 4 * 1024 * 1024;

export interface UploadHandle {
  id: string;
  name: string;
  size: number;
  /** Bytes confirmed by the PC. */
  offset: number;
  /** Bytes on the wire so far, including the chunk in flight (for smooth progress). */
  sent?: number;
  state: 'uploading' | 'paused' | 'done' | 'error' | 'cancelled';
  error?: string;
  sha256?: string;
  path?: string;
}

function isAbort(e: unknown): boolean {
  return e instanceof DOMException && e.name === 'AbortError';
}

/**
 * PUT one chunk with XMLHttpRequest: unlike fetch it reports upload
 * progress, and aborting stops the transfer immediately (pause).
 */
function putChunk(url: string, chunk: Uint8Array, crc: number, signal: AbortSignal | undefined, onSent: (bytes: number) => void): Promise<{ offset: number }> {
  return new Promise((resolve, reject) => {
    if (signal?.aborted) return reject(new DOMException('paused', 'AbortError'));
    const xhr = new XMLHttpRequest();
    xhr.open('PUT', url);
    const token = getToken();
    if (token) xhr.setRequestHeader('authorization', `Bearer ${token}`);
    xhr.setRequestHeader('content-type', 'application/octet-stream');
    xhr.setRequestHeader('x-chunk-crc32', crc.toString(16).padStart(8, '0'));
    const onAbort = () => xhr.abort();
    signal?.addEventListener('abort', onAbort, { once: true });
    const done = () => signal?.removeEventListener('abort', onAbort);
    xhr.upload.onprogress = (e) => onSent(e.loaded);
    xhr.onload = () => {
      done();
      let data: any;
      try {
        data = xhr.responseText ? JSON.parse(xhr.responseText) : undefined;
      } catch {
        data = undefined;
      }
      if (xhr.status >= 200 && xhr.status < 300) return resolve(data);
      const message: string = data?.error ?? (xhr.statusText || `HTTP ${xhr.status}`);
      if (xhr.status === 401 && isDeviceAuthFailure(url, message)) window.dispatchEvent(new CustomEvent('omnihub:unauthorized'));
      reject(new ApiError(xhr.status, message));
    };
    xhr.onerror = () => {
      done();
      reject(new TypeError('network error'));
    };
    xhr.onabort = () => {
      done();
      reject(new DOMException('paused', 'AbortError'));
    };
    xhr.send(chunk as Uint8Array<ArrayBuffer>);
  });
}

/**
 * Upload a file. Survives network drops: on failure it asks the server for
 * the confirmed offset and continues from there (up to `retries` times per
 * chunk). Call `controller.abort()` to pause (the chunk in flight stops at
 * once); call again with `resumeId` to continue a paused upload.
 */
export async function uploadFile(
  file: File,
  onProgress: (u: UploadHandle) => void,
  opts: { dir?: string; signal?: AbortSignal; resumeId?: string; retries?: number } = {},
): Promise<UploadHandle> {
  const retries = opts.retries ?? 5;
  let id = opts.resumeId;
  let offset = 0;
  if (id) {
    offset = (await request<{ offset: number }>('GET', `/api/upload/${id}`)).offset;
  } else {
    const st = await request<{ id: string; offset: number }>('POST', '/api/upload', { name: file.name, size: file.size, dir: opts.dir ?? null });
    id = st.id;
  }
  const h: UploadHandle = { id, name: file.name, size: file.size, offset, sent: offset, state: 'uploading' };
  onProgress({ ...h });
  const paused = () => {
    h.state = 'paused';
    h.sent = h.offset;
    onProgress({ ...h });
    return h;
  };
  let failures = 0;
  while (h.offset < file.size) {
    if (opts.signal?.aborted) return paused();
    const end = Math.min(h.offset + CHUNK_SIZE, file.size);
    const chunk = new Uint8Array(await file.slice(h.offset, end).arrayBuffer());
    const base = h.offset;
    try {
      let last = 0;
      const r = await putChunk(`/api/upload/${id}/chunk?offset=${base}`, chunk, crc32(chunk), opts.signal, (bytes) => {
        const now = performance.now();
        if (now - last < 100) return;
        last = now;
        h.sent = base + Math.min(bytes, chunk.length);
        onProgress({ ...h });
      });
      h.offset = r.offset;
      h.sent = r.offset;
      failures = 0;
      onProgress({ ...h });
    } catch (e) {
      if (isAbort(e) || opts.signal?.aborted) return paused();
      failures++;
      if (e instanceof ApiError && (e.status === 401 || e.status === 403 || e.status === 404 || e.status === 413 || e.status === 507)) {
        h.state = 'error';
        h.error = e.message;
        onProgress({ ...h });
        return h;
      }
      if (failures > retries) {
        h.state = 'error';
        h.error = e instanceof Error ? e.message : String(e);
        onProgress({ ...h });
        return h;
      }
      h.sent = h.offset;
      onProgress({ ...h });
      await new Promise((r) => setTimeout(r, 500 * 2 ** failures));
      if (opts.signal?.aborted) return paused();
      try {
        h.offset = (await request<{ offset: number }>('GET', `/api/upload/${id}`)).offset;
      } catch {
        /* keep trying */
      }
    }
  }
  const done = await request<{ path: string; sha256: string }>('POST', `/api/upload/${id}/finish`);
  h.state = 'done';
  h.path = done.path;
  h.sha256 = done.sha256;
  h.sent = file.size;
  onProgress({ ...h });
  return h;
}

export function cancelUpload(id: string) {
  return request<void>('DELETE', `/api/upload/${id}`);
}

// ---------- screen sharing protocol ----------

/** Header of a binary frame message (see capture/stream.rs `frame_message`). */
export interface FrameHeader {
  seq: number;
  width: number;
  height: number;
  cursor: { x: number; y: number } | null;
  encodeMs: number;
  ts: number;
}

export const FRAME_HEADER = 20;

export function parseFrame(buf: ArrayBuffer): { header: FrameHeader; jpeg: Uint8Array } | null {
  const v = new DataView(buf);
  if (buf.byteLength < FRAME_HEADER || v.getUint8(0) !== 1) return null;
  const flags = v.getUint8(1);
  const cx = v.getInt16(10, true);
  const cy = v.getInt16(12, true);
  return {
    header: {
      seq: v.getUint32(2, true),
      width: v.getUint16(6, true),
      height: v.getUint16(8, true),
      cursor: flags & 0b10 && cx >= 0 && cy >= 0 ? { x: cx, y: cy } : null,
      encodeMs: v.getUint16(14, true),
      ts: v.getUint32(16, true),
    },
    jpeg: new Uint8Array(buf, FRAME_HEADER),
  };
}

export interface ScreenStats {
  fps: number;
  kbps: number;
  rtt: number;
  encodeMs: number;
  quality: number;
  maxWidth: number;
  control: boolean;
  monitor: MonitorInfo | null;
}

/** Messages the viewer sends (see capture/stream.rs `ViewerMessage`). */
export type ViewerMessage =
  | { t: 'ack'; seq: number; ts: number }
  | { t: 'preset'; id: string }
  | { t: 'pointer'; x: number; y: number; kind: 'move' | 'down' | 'up' | 'click'; button?: number }
  | { t: 'wheel'; dy: number; dx?: number }
  | { t: 'key'; key: string }
  | { t: 'text'; text: string };

export async function openScreen(monitor: number, preset: string): Promise<WebSocket> {
  const { ticket } = await client.ticket('screen');
  const ws = new WebSocket(wsUrl(`/api/screen/ws?ticket=${encodeURIComponent(ticket)}&monitor=${monitor}&preset=${encodeURIComponent(preset)}`));
  ws.binaryType = 'arraybuffer';
  return ws;
}

/** Pairing secret from a QR link (https://pc:47800/#pair=SECRET). */
export function pairSecretFromUrl(): string | null {
  const m = /[#&]pair=([A-Za-z0-9_-]+)/.exec(location.hash);
  return m ? m[1] : null;
}
