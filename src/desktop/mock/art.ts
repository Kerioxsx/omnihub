// Procedural SVG art for the mock: app icons, screenshot scenes, the frozen
// desktop shown by the region overlay, and a QR-looking pairing code.

import { Rng, hashString } from './rng';

export function svgUrl(svg: string): string {
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
}

const GRADIENTS: [string, string][] = [
  ['#7c3aed', '#22d3ee'],
  ['#f43f5e', '#fb923c'],
  ['#10b981', '#3b82f6'],
  ['#f59e0b', '#ef4444'],
  ['#6366f1', '#ec4899'],
  ['#0ea5e9', '#14b8a6'],
  ['#8b5cf6', '#f472b6'],
  ['#22c55e', '#eab308'],
  ['#334155', '#64748b'],
  ['#e11d48', '#7c3aed'],
];

export function monogram(name: string): string {
  const words = name.replace(/[^A-Za-z0-9 ]/g, ' ').split(/\s+/).filter(Boolean);
  if (!words.length) return '?';
  if (words.length === 1) return words[0].slice(0, 2).replace(/^./, (c) => c.toUpperCase());
  return (words[0][0] + words[1][0]).toUpperCase();
}

export function appIcon(name: string, colors?: [string, string]): string {
  const [a, b] = colors ?? GRADIENTS[hashString(name) % GRADIENTS.length];
  const m = monogram(name);
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" width="64" height="64">
<defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="${a}"/><stop offset="1" stop-color="${b}"/></linearGradient>
<radialGradient id="s" cx=".3" cy=".2" r=".9"><stop offset="0" stop-color="#fff" stop-opacity=".35"/><stop offset=".6" stop-color="#fff" stop-opacity="0"/></radialGradient></defs>
<rect x="4" y="4" width="56" height="56" rx="15" fill="url(#g)"/><rect x="4" y="4" width="56" height="56" rx="15" fill="url(#s)"/>
<text x="32" y="41" text-anchor="middle" font-family="Segoe UI, Inter, Arial, sans-serif" font-weight="700" font-size="${m.length > 1 ? 22 : 26}" fill="#fff">${m}</text></svg>`;
  return svgUrl(svg);
}

// ---------- screenshot scenes ----------

export type SceneKind = 'game' | 'code' | 'browser' | 'chat' | 'blender' | 'desktop';

const SKIES: [string, string, string][] = [
  ['#1e1b4b', '#7c3aed', '#fb923c'],
  ['#082f49', '#0e7490', '#67e8f9'],
  ['#3b0764', '#be185d', '#fda4af'],
  ['#052e16', '#15803d', '#bef264'],
  ['#1c1917', '#9a3412', '#fcd34d'],
  ['#0f172a', '#1d4ed8', '#a5b4fc'],
];

function mountains(r: Rng, w: number, h: number, base: number, color: string, opacity: number, rough: number): string {
  const pts: string[] = [`0,${h}`];
  const steps = 14;
  for (let i = 0; i <= steps; i++) {
    const x = (i / steps) * w;
    const y = base - r.range(0, rough) * h;
    pts.push(`${x.toFixed(0)},${y.toFixed(0)}`);
  }
  pts.push(`${w},${h}`);
  return `<polygon points="${pts.join(' ')}" fill="${color}" opacity="${opacity}"/>`;
}

function gameScene(r: Rng, w: number, h: number, detail: boolean): string {
  const [s0, s1, s2] = r.pick(SKIES);
  const sunX = r.range(0.2, 0.8) * w;
  const sunY = r.range(0.25, 0.45) * h;
  let out = `<defs><linearGradient id="sky" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${s0}"/><stop offset=".6" stop-color="${s1}"/><stop offset="1" stop-color="${s2}"/></linearGradient>
<radialGradient id="sun"><stop offset="0" stop-color="#fff7ed"/><stop offset=".4" stop-color="${s2}" stop-opacity=".9"/><stop offset="1" stop-color="${s2}" stop-opacity="0"/></radialGradient></defs>
<rect width="${w}" height="${h}" fill="url(#sky)"/><circle cx="${sunX}" cy="${sunY}" r="${h * 0.32}" fill="url(#sun)"/>`;
  if (detail) for (let i = 0; i < 40; i++) out += `<circle cx="${r.range(0, w)}" cy="${r.range(0, h * 0.4)}" r="${r.range(0.5, 1.6) * (w / 800)}" fill="#fff" opacity="${r.range(0.2, 0.7)}"/>`;
  out += mountains(r, w, h, h * 0.62, s0, 0.45, 0.22);
  out += mountains(r, w, h, h * 0.74, s0, 0.7, 0.16);
  out += mountains(r, w, h, h * 0.86, '#020617', 0.85, 0.1);
  // HUD
  const u = w / 320;
  out += `<g opacity=".92"><rect x="${10 * u}" y="${h - 18 * u}" width="${70 * u}" height="${5 * u}" rx="${2 * u}" fill="#0008"/><rect x="${10 * u}" y="${h - 18 * u}" width="${r.range(30, 68) * u}" height="${5 * u}" rx="${2 * u}" fill="#f43f5e"/>
<rect x="${10 * u}" y="${h - 11 * u}" width="${50 * u}" height="${3 * u}" rx="${1.5 * u}" fill="#0008"/><rect x="${10 * u}" y="${h - 11 * u}" width="${r.range(15, 48) * u}" height="${3 * u}" rx="${1.5 * u}" fill="#38bdf8"/>
<circle cx="${w - 24 * u}" cy="${24 * u}" r="${16 * u}" fill="#0007" stroke="#fff6" stroke-width="${u}"/><circle cx="${w - 24 * u}" cy="${24 * u}" r="${2 * u}" fill="#facc15"/>
<path d="M${w / 2 - 6 * u} ${h / 2}h${4 * u}M${w / 2 + 2 * u} ${h / 2}h${4 * u}M${w / 2} ${h / 2 - 6 * u}v${4 * u}M${w / 2} ${h / 2 + 2 * u}v${4 * u}" stroke="#fff" stroke-width="${0.8 * u}"/></g>`;
  return out;
}

function codeScene(r: Rng, w: number, h: number): string {
  const u = w / 320;
  const colors = ['#c084fc', '#7dd3fc', '#fca5a5', '#86efac', '#fde68a', '#e2e8f0', '#94a3b8'];
  let out = `<rect width="${w}" height="${h}" fill="#11111b"/><rect width="${w}" height="${10 * u}" fill="#181825"/><rect y="${10 * u}" width="${46 * u}" height="${h}" fill="#181825"/>`;
  for (let i = 0; i < 14; i++) out += `<rect x="${6 * u}" y="${(16 + i * 8) * u}" width="${r.range(14, 34) * u}" height="${3 * u}" rx="${u}" fill="#45475a"/>`;
  out += `<rect x="${48 * u}" y="${2 * u}" width="${40 * u}" height="${8 * u}" rx="${1.5 * u}" fill="#1e1e2e"/><rect x="${52 * u}" y="${5 * u}" width="${26 * u}" height="${2 * u}" fill="#cdd6f4"/>`;
  let y = 16 * u;
  let indent = 0;
  while (y < h - 6 * u) {
    out += `<rect x="${50 * u}" y="${y}" width="${6 * u}" height="${2.4 * u}" fill="#45475a"/>`;
    let x = (60 + indent * 6) * u;
    const parts = r.int(1, 5);
    for (let p = 0; p < parts; p++) {
      const len = r.range(6, 30) * u;
      if (x + len > w - 60 * u) break;
      out += `<rect x="${x}" y="${y}" width="${len}" height="${2.4 * u}" rx="${u}" fill="${r.pick(colors)}"/>`;
      x += len + 3 * u;
    }
    indent = Math.max(0, Math.min(4, indent + r.pick([-1, 0, 0, 1])));
    y += 5.2 * u;
  }
  out += `<rect x="${w - 56 * u}" y="${10 * u}" width="${56 * u}" height="${h}" fill="#181825"/>`;
  for (let i = 0; i < 6; i++) out += `<rect x="${w - 50 * u}" y="${(18 + i * 14) * u}" width="${44 * u}" height="${10 * u}" rx="${2 * u}" fill="${i === 1 ? '#7c3aed55' : '#313244'}"/>`;
  return out;
}

function browserScene(r: Rng, w: number, h: number): string {
  const u = w / 320;
  const [a, b] = r.pick(GRADIENTS);
  let out = `<defs><linearGradient id="hero" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="${a}"/><stop offset="1" stop-color="${b}"/></linearGradient></defs>
<rect width="${w}" height="${h}" fill="#f8fafc"/><rect width="${w}" height="${18 * u}" fill="#e2e8f0"/><rect x="${40 * u}" y="${4 * u}" width="${w - 80 * u}" height="${10 * u}" rx="${5 * u}" fill="#fff"/>
<rect x="${20 * u}" y="${28 * u}" width="${w - 40 * u}" height="${60 * u}" rx="${6 * u}" fill="url(#hero)"/><rect x="${32 * u}" y="${44 * u}" width="${110 * u}" height="${9 * u}" rx="${2 * u}" fill="#fff"/><rect x="${32 * u}" y="${58 * u}" width="${80 * u}" height="${4 * u}" rx="${2 * u}" fill="#ffffffaa"/>`;
  for (let i = 0; i < 3; i++) {
    const x = (20 + i * 95) * u;
    out += `<rect x="${x}" y="${96 * u}" width="${88 * u}" height="${h - 104 * u}" rx="${5 * u}" fill="#fff" stroke="#e2e8f0"/><rect x="${x + 8 * u}" y="${104 * u}" width="${40 * u}" height="${24 * u}" rx="${3 * u}" fill="${r.pick(GRADIENTS)[0]}33"/>`;
    for (let k = 0; k < 3; k++) out += `<rect x="${x + 8 * u}" y="${(134 + k * 7) * u}" width="${r.range(40, 70) * u}" height="${3 * u}" rx="${1.5 * u}" fill="#cbd5e1"/>`;
  }
  return out;
}

function chatScene(r: Rng, w: number, h: number): string {
  const u = w / 320;
  let out = `<rect width="${w}" height="${h}" fill="#313338"/><rect width="${22 * u}" height="${h}" fill="#1e1f22"/><rect x="${22 * u}" width="${62 * u}" height="${h}" fill="#2b2d31"/>`;
  for (let i = 0; i < 7; i++) out += `<circle cx="${11 * u}" cy="${(14 + i * 20) * u}" r="${7 * u}" fill="${r.pick(GRADIENTS)[0]}"/>`;
  for (let i = 0; i < 12; i++) out += `<rect x="${28 * u}" y="${(14 + i * 11) * u}" width="${r.range(26, 50) * u}" height="${3.5 * u}" rx="${1.5 * u}" fill="${i === 3 ? '#dbdee1' : '#80848e'}"/>`;
  let y = 16 * u;
  while (y < h - 30 * u) {
    out += `<circle cx="${98 * u}" cy="${y + 5 * u}" r="${6 * u}" fill="${r.pick(GRADIENTS)[1]}"/><rect x="${108 * u}" y="${y}" width="${r.range(24, 40) * u}" height="${3.5 * u}" rx="${1.5 * u}" fill="#f2f3f5"/>`;
    const lines = r.int(1, 3);
    for (let k = 0; k < lines; k++) out += `<rect x="${108 * u}" y="${y + (7 + k * 6) * u}" width="${r.range(60, 190) * u}" height="${3 * u}" rx="${1.5 * u}" fill="#b5bac1"/>`;
    if (r.chance(0.25)) {
      out += `<rect x="${108 * u}" y="${y + (8 + lines * 6) * u}" width="${80 * u}" height="${40 * u}" rx="${4 * u}" fill="${r.pick(GRADIENTS)[0]}88"/>`;
      y += 44 * u;
    }
    y += (14 + lines * 6) * u;
  }
  out += `<rect x="${92 * u}" y="${h - 18 * u}" width="${w - 100 * u}" height="${12 * u}" rx="${4 * u}" fill="#383a40"/>`;
  return out;
}

function blenderScene(r: Rng, w: number, h: number): string {
  const u = w / 320;
  let out = `<defs><radialGradient id="vp" cx=".5" cy=".45" r=".7"><stop offset="0" stop-color="#4b4b52"/><stop offset="1" stop-color="#2a2a2e"/></radialGradient>
<linearGradient id="obj" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#fbbf24"/><stop offset="1" stop-color="#ea580c"/></linearGradient></defs>
<rect width="${w}" height="${h}" fill="url(#vp)"/>`;
  const hy = h * 0.55;
  for (let i = -12; i <= 12; i++) out += `<line x1="${w / 2 + i * 14 * u}" y1="${hy}" x2="${w / 2 + i * 60 * u}" y2="${h}" stroke="#ffffff14"/>`;
  for (let i = 0; i < 8; i++) {
    const y = hy + (h - hy) * Math.pow(i / 8, 1.6);
    out += `<line x1="0" y1="${y}" x2="${w}" y2="${y}" stroke="#ffffff14"/>`;
  }
  const cx = w * r.range(0.4, 0.6);
  const cy = h * 0.5;
  const s = 36 * u;
  out += `<polygon points="${cx},${cy - s} ${cx + s},${cy - s / 2} ${cx},${cy} ${cx - s},${cy - s / 2}" fill="#fde68a"/><polygon points="${cx - s},${cy - s / 2} ${cx},${cy} ${cx},${cy + s} ${cx - s},${cy + s / 2}" fill="url(#obj)"/><polygon points="${cx + s},${cy - s / 2} ${cx},${cy} ${cx},${cy + s} ${cx + s},${cy + s / 2}" fill="#c2410c"/>
<polygon points="${cx},${cy - s} ${cx + s},${cy - s / 2} ${cx + s},${cy + s / 2} ${cx},${cy + s} ${cx - s},${cy + s / 2} ${cx - s},${cy - s / 2}" fill="none" stroke="#f97316" stroke-width="${u}"/>
<rect width="${w}" height="${9 * u}" fill="#1d1d1f"/><rect x="${w - 54 * u}" y="${9 * u}" width="${54 * u}" height="${h}" fill="#28282bdd"/>`;
  for (let i = 0; i < 10; i++) out += `<rect x="${w - 50 * u}" y="${(14 + i * 9) * u}" width="${r.range(20, 44) * u}" height="${4 * u}" rx="${u}" fill="#505055"/>`;
  return out;
}

function desktopScene(r: Rng, w: number, h: number): string {
  const u = w / 320;
  let out = `<defs><linearGradient id="wall" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#0b1026"/><stop offset=".5" stop-color="#312e81"/><stop offset="1" stop-color="#0e7490"/></linearGradient>
<radialGradient id="aur" cx=".7" cy=".2" r=".6"><stop offset="0" stop-color="#22d3ee" stop-opacity=".45"/><stop offset="1" stop-color="#22d3ee" stop-opacity="0"/></radialGradient>
<radialGradient id="aur2" cx=".2" cy=".8" r=".6"><stop offset="0" stop-color="#a855f7" stop-opacity=".4"/><stop offset="1" stop-color="#a855f7" stop-opacity="0"/></radialGradient></defs>
<rect width="${w}" height="${h}" fill="url(#wall)"/><rect width="${w}" height="${h}" fill="url(#aur)"/><rect width="${w}" height="${h}" fill="url(#aur2)"/>`;
  const win = (x: number, y: number, ww: number, hh: number, inner: string) =>
    `<g transform="translate(${x * u} ${y * u})"><rect width="${ww * u}" height="${hh * u}" rx="${3 * u}" fill="#000" opacity=".35" transform="translate(${1.5 * u} ${2.5 * u})"/><svg width="${ww * u}" height="${hh * u}" viewBox="0 0 ${ww * u} ${hh * u}"><rect width="100%" height="100%" rx="${3 * u}" fill="#1e1e2e"/>${inner}</svg></g>`;
  out += win(18, 16, 170, 110, codeScene(new Rng(r.int(1, 1e9)), 170 * u, 110 * u));
  out += win(132, 38, 168, 112, browserScene(new Rng(r.int(1, 1e9)), 168 * u, 112 * u));
  out += `<rect y="${h - 12 * u}" width="${w}" height="${12 * u}" fill="#0f172acc"/>`;
  for (let i = 0; i < 8; i++) out += `<rect x="${w / 2 + (i - 4) * 11 * u}" y="${h - 10 * u}" width="${8 * u}" height="${8 * u}" rx="${2 * u}" fill="${GRADIENTS[i][0]}"/>`;
  out += `<rect x="${w - 34 * u}" y="${h - 8.5 * u}" width="${26 * u}" height="${2.4 * u}" rx="${u}" fill="#e2e8f0"/><rect x="${w - 30 * u}" y="${h - 5 * u}" width="${18 * u}" height="${2 * u}" rx="${u}" fill="#94a3b8"/>`;
  return out;
}

export function scene(seed: string, kind: SceneKind, w: number, h: number, detail = false): string {
  const r = new Rng(hashString(seed));
  const body =
    kind === 'game' ? gameScene(r, w, h, detail) : kind === 'code' ? codeScene(r, w, h) : kind === 'browser' ? browserScene(r, w, h) : kind === 'chat' ? chatScene(r, w, h) : kind === 'blender' ? blenderScene(r, w, h) : desktopScene(r, w, h);
  return svgUrl(`<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}">${body}</svg>`);
}

// ---------- QR-looking pairing code ----------

export function fakeQr(text: string): string {
  const n = 33;
  const r = new Rng(hashString(text));
  const grid: boolean[][] = Array.from({ length: n }, () => Array.from({ length: n }, () => r.chance(0.48)));
  const finder = (ox: number, oy: number) => {
    for (let y = -1; y <= 7; y++)
      for (let x = -1; x <= 7; x++) {
        const gx = ox + x;
        const gy = oy + y;
        if (gx < 0 || gy < 0 || gx >= n || gy >= n) continue;
        const edge = x === 0 || x === 6 || y === 0 || y === 6;
        const core = x >= 2 && x <= 4 && y >= 2 && y <= 4;
        grid[gy][gx] = x >= 0 && x <= 6 && y >= 0 && y <= 6 && (edge || core);
      }
  };
  finder(0, 0);
  finder(n - 7, 0);
  finder(0, n - 7);
  for (let i = 8; i < n - 8; i++) {
    grid[6][i] = i % 2 === 0;
    grid[i][6] = i % 2 === 0;
  }
  // alignment pattern
  for (let y = -2; y <= 2; y++) for (let x = -2; x <= 2; x++) grid[n - 9 + y][n - 9 + x] = Math.max(Math.abs(x), Math.abs(y)) !== 1;
  let rects = '';
  for (let y = 0; y < n; y++) {
    let x = 0;
    while (x < n) {
      if (!grid[y][x]) {
        x++;
        continue;
      }
      let len = 1;
      while (x + len < n && grid[y][x + len]) len++;
      rects += `<rect x="${x + 4}" y="${y + 4}" width="${len}" height="1"/>`;
      x += len;
    }
  }
  const size = n + 8;
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${size} ${size}" shape-rendering="crispEdges"><rect width="${size}" height="${size}" fill="#fff"/><g fill="#000">${rects}</g></svg>`;
}
