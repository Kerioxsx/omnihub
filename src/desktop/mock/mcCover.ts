// Mix covers without the PC: drawn pack icons, the same blend the app makes
// (each pack's icon whole on a wash of its colours, soft seams), a pretend
// Pollinations painting, and a pretend sign-in.

import type { PollSignIn, PollStatus } from '@shared/types';

const svg = (body: string) => `data:image/svg+xml;charset=utf-8,${encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128">${body}</svg>`)}`;

/** A pack's demo icon, drawn. */
export function packIcon(name: string): string {
  const n = name.toLowerCase();
  if (n.includes('fabulously')) return svg('<rect width="128" height="128" rx="22" fill="#2a2622"/><circle cx="64" cy="64" r="52" fill="#e9dfc5"/><g fill="#2a2622"><rect x="30" y="40" width="8" height="48"/><rect x="30" y="40" width="26" height="8"/><rect x="30" y="60" width="20" height="8"/><rect x="66" y="40" width="32" height="8"/><rect x="66" y="80" width="32" height="8"/><rect x="66" y="40" width="8" height="48"/><rect x="90" y="40" width="8" height="48"/></g>');
  if (n.includes('cobblemon')) return svg('<rect width="128" height="128" rx="22" fill="#1f1b2e"/><path d="M64 14 108 39v50L64 114 20 89V39z" fill="#ece9f5"/><path d="M64 14 108 39 64 64 20 39z" fill="#e8384f"/><path d="M64 64v50L20 89V39z" fill="#c9c4dc"/><circle cx="64" cy="64" r="15" fill="#1f1b2e"/><circle cx="64" cy="64" r="8" fill="#ece9f5"/>');
  if (n.includes('create')) return svg('<rect width="128" height="128" rx="22" fill="#3b2c1f"/><circle cx="64" cy="64" r="40" fill="#c8964c"/><g fill="#8a5a2b">' + Array.from({ length: 8 }, (_, i) => `<rect x="58" y="12" width="12" height="22" transform="rotate(${i * 45} 64 64)"/>`).join('') + '</g><circle cx="64" cy="64" r="14" fill="#3b2c1f"/>');
  let h = 0;
  for (const c of name) h = (h * 31 + c.charCodeAt(0)) % 360;
  const cells = Array.from({ length: 64 }, (_, i) => `<rect x="${(i % 8) * 16}" y="${Math.floor(i / 8) * 16}" width="16" height="16" fill="hsl(${h} 55% ${30 + ((i * 7919) % 17)}%)"/>`).join('');
  return svg(cells);
}

function load(src: string): Promise<HTMLImageElement> {
  return new Promise((ok, bad) => {
    const i = new Image();
    i.onload = () => ok(i);
    i.onerror = () => bad(new Error('picture'));
    i.src = src;
  });
}

function roundRect(g: CanvasRenderingContext2D, x: number, y: number, s: number, r: number) {
  g.beginPath();
  g.roundRect(x, y, s, s, r);
}

const S = 512;

/** Where each pack goes: its part (a clip path) and its icon (centre, side). */
function layout(n: number) {
  const c = S / 2;
  if (n === 1) return [{ clip: (g: CanvasRenderingContext2D) => g.rect(0, 0, S, S), cx: c, cy: c, side: S * 0.84 }];
  if (n === 2)
    return [
      { clip: (g: CanvasRenderingContext2D) => (g.moveTo(0, 0), g.lineTo(S, 0), g.lineTo(0, S), g.closePath()), cx: S / 3, cy: S / 3, side: S * 0.6 },
      { clip: (g: CanvasRenderingContext2D) => (g.moveTo(S, 0), g.lineTo(S, S), g.lineTo(0, S), g.closePath()), cx: (2 * S) / 3, cy: (2 * S) / 3, side: S * 0.6 },
    ];
  const [side, dist] = n === 3 ? [0.46, 0.27] : n === 4 ? [0.38, 0.3] : [0.32, 0.31];
  const w = (2 * Math.PI) / n;
  return Array.from({ length: n }, (_, k) => {
    const a0 = k * w - Math.PI;
    const mid = a0 + w / 2;
    return {
      clip: (g: CanvasRenderingContext2D) => (g.moveTo(c, c), g.arc(c, c, S, a0, a0 + w), g.closePath()),
      cx: c + S * dist * Math.cos(mid),
      cy: c + S * dist * Math.sin(mid),
      side: S * side,
    };
  });
}

/** The blend the app makes, drawn in the browser. */
export async function blendCover(icons: string[]): Promise<string> {
  const pics = await Promise.all(icons.slice(0, 6).map(load));
  const cv = document.createElement('canvas');
  cv.width = cv.height = S;
  const g = cv.getContext('2d')!;
  g.fillStyle = '#24242c';
  g.fillRect(0, 0, S, S);
  layout(pics.length).forEach((p, k) => {
    g.save();
    g.beginPath();
    p.clip(g);
    g.clip();
    g.filter = 'blur(28px) brightness(0.5)';
    g.drawImage(pics[k], -40, -40, S + 80, S + 80);
    g.filter = 'none';
    g.shadowColor = 'rgba(0,0,0,.55)';
    g.shadowBlur = p.side * 0.09;
    g.shadowOffsetY = p.side * 0.035;
    roundRect(g, p.cx - p.side / 2, p.cy - p.side / 2, p.side, p.side * 0.14);
    g.fillStyle = '#000';
    g.fill();
    g.shadowColor = 'transparent';
    g.clip();
    g.imageSmoothingEnabled = false;
    g.drawImage(pics[k], p.cx - p.side / 2, p.cy - p.side / 2, p.side, p.side);
    g.restore();
  });
  const v = g.createRadialGradient(S / 2, S / 2, S * 0.44, S / 2, S / 2, S * 0.72);
  v.addColorStop(0, 'rgba(0,0,0,0)');
  v.addColorStop(1, 'rgba(0,0,0,.22)');
  g.fillStyle = v;
  g.fillRect(0, 0, S, S);
  return cv.toDataURL('image/png');
}

/** A pretend painting standing in for Pollinations' answer. */
export async function paintedCover(icons: string[]): Promise<string> {
  const pics = await Promise.all(icons.slice(0, 6).map(load));
  const cv = document.createElement('canvas');
  cv.width = cv.height = S;
  const g = cv.getContext('2d')!;
  const sky = g.createLinearGradient(0, 0, 0, S);
  sky.addColorStop(0, '#f6d9a8');
  sky.addColorStop(0.55, '#e8687a');
  sky.addColorStop(1, '#3a2a55');
  g.fillStyle = sky;
  g.fillRect(0, 0, S, S);
  // Blocky hills.
  const px = 16;
  for (let x = 0; x < S; x += px) {
    const h = 300 + Math.round((Math.sin(x / 60) * 40 + Math.sin(x / 23) * 14) / px) * px;
    for (let y = h; y < S; y += px) {
      const shade = y === h ? '#6fbf4a' : (x / px + y / px) % 3 === 0 ? '#6b4a2f' : '#7d5737';
      g.fillStyle = shade;
      g.fillRect(x, y, px, px);
    }
  }
  g.globalAlpha = 0.95;
  g.imageSmoothingEnabled = false;
  pics.forEach((p, k) => {
    const side = pics.length > 1 ? 210 : 300;
    const x = pics.length > 1 ? 70 + k * (S - 140 - side) / (pics.length - 1) : (S - side) / 2;
    g.drawImage(p, x, 150 - (k % 2) * 30, side, side);
  });
  g.globalAlpha = 1;
  return cv.toDataURL('image/png');
}

let poll: PollStatus = { connected: false, user: null, via: null, connectedAt: null, waiting: false };
let checks = 0;

export const pollMock = {
  status: (): PollStatus => ({ ...poll }),
  signIn(): PollSignIn {
    poll = { ...poll, waiting: true };
    checks = 0;
    return { userCode: 'WXQT-7K2M', url: 'https://enter.pollinations.ai/device?user_code=WXQT-7K2M', expiresIn: 1800, interval: 2 };
  },
  check(): PollStatus {
    if (!poll.waiting) throw 'No sign-in is waiting.';
    checks += 1;
    if (checks >= 2) poll = { connected: true, user: 'kerio', via: 'signIn', connectedAt: Math.floor(Date.now() / 1000), waiting: false };
    return { ...poll };
  },
  setKey(key: string): PollStatus {
    if (!key.trim()) return pollMock.forget();
    if (!/^sk_[A-Za-z0-9_.-]{17,}$/.test(key.trim())) throw 'That doesn’t look like a Pollinations secret key (they start with sk_). Get one at enter.pollinations.ai, or sign in instead.';
    poll = { connected: true, user: null, via: 'key', connectedAt: Math.floor(Date.now() / 1000), waiting: false };
    return { ...poll };
  },
  forget(): PollStatus {
    poll = { connected: false, user: null, via: null, connectedAt: null, waiting: false };
    return { ...poll };
  },
};
