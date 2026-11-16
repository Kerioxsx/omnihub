// The scanner's picture work, all on the phone: find the page's corners,
// straighten it (perspective), and clean it up for reading.

export type Pt = [number, number];
/** Top-left, top-right, bottom-right, bottom-left; 0–1 of the picture. */
export type Quad = [Pt, Pt, Pt, Pt];

export const FULL: Quad = [
  [0.04, 0.04],
  [0.96, 0.04],
  [0.96, 0.96],
  [0.04, 0.96],
];

/** A photo as a canvas, at most `max` pixels on its longest side. */
export async function loadPhoto(file: Blob, max = 3000): Promise<HTMLCanvasElement> {
  const bmp = await createImageBitmap(file, { imageOrientation: 'from-image' } as ImageBitmapOptions);
  const k = Math.min(1, max / Math.max(bmp.width, bmp.height));
  const c = document.createElement('canvas');
  c.width = Math.round(bmp.width * k);
  c.height = Math.round(bmp.height * k);
  c.getContext('2d')!.drawImage(bmp, 0, 0, c.width, c.height);
  bmp.close();
  return c;
}

/**
 * The page: the biggest bright region (a page is lighter than the table),
 * and its four extreme corners. Falls back to nearly the whole picture.
 */
export function findPage(src: HTMLCanvasElement): Quad {
  const W = 320;
  const k = W / Math.max(src.width, src.height);
  const w = Math.max(8, Math.round(src.width * k));
  const h = Math.max(8, Math.round(src.height * k));
  const c = document.createElement('canvas');
  c.width = w;
  c.height = h;
  const ctx = c.getContext('2d', { willReadFrequently: true })!;
  ctx.filter = 'blur(2px)';
  ctx.drawImage(src, 0, 0, w, h);
  const px = ctx.getImageData(0, 0, w, h).data;
  const gray = new Uint8Array(w * h);
  const hist = new Array(256).fill(0);
  for (let i = 0; i < w * h; i++) {
    const g = (px[i * 4] * 77 + px[i * 4 + 1] * 150 + px[i * 4 + 2] * 29) >> 8;
    gray[i] = g;
    hist[g]++;
  }
  // Otsu's threshold.
  let sum = 0;
  for (let i = 0; i < 256; i++) sum += i * hist[i];
  let sumB = 0;
  let wB = 0;
  let best = 0;
  let t = 128;
  for (let i = 0; i < 256; i++) {
    wB += hist[i];
    if (!wB) continue;
    const wF = w * h - wB;
    if (!wF) break;
    sumB += i * hist[i];
    const mB = sumB / wB;
    const mF = (sum - sumB) / wF;
    const between = wB * wF * (mB - mF) ** 2;
    if (between > best) {
      best = between;
      t = i;
    }
  }
  // The largest bright region (4-connected).
  const label = new Int32Array(w * h).fill(-1);
  let bestSize = 0;
  let bestId = -1;
  const stack: number[] = [];
  let id = 0;
  for (let s = 0; s < w * h; s++) {
    if (gray[s] <= t || label[s] !== -1) continue;
    let size = 0;
    stack.push(s);
    label[s] = id;
    while (stack.length) {
      const p = stack.pop()!;
      size++;
      const x = p % w;
      const y = (p / w) | 0;
      for (const q of [x > 0 ? p - 1 : -1, x < w - 1 ? p + 1 : -1, y > 0 ? p - w : -1, y < h - 1 ? p + w : -1]) {
        if (q >= 0 && label[q] === -1 && gray[q] > t) {
          label[q] = id;
          stack.push(q);
        }
      }
    }
    if (size > bestSize) {
      bestSize = size;
      bestId = id;
    }
    id++;
  }
  const area = bestSize / (w * h);
  if (bestId < 0 || area < 0.12 || area > 0.97) return FULL;
  let tl: Pt = [0, 0], tr: Pt = [0, 0], br: Pt = [0, 0], bl: Pt = [0, 0];
  let mTl = Infinity, mTr = -Infinity, mBr = -Infinity, mBl = -Infinity;
  for (let p = 0; p < w * h; p++) {
    if (label[p] !== bestId) continue;
    const x = p % w;
    const y = (p / w) | 0;
    if (x + y < mTl) (mTl = x + y), (tl = [x, y]);
    if (x - y > mTr) (mTr = x - y), (tr = [x, y]);
    if (x + y > mBr) (mBr = x + y), (br = [x, y]);
    if (y - x > mBl) (mBl = y - x), (bl = [x, y]);
  }
  const n = (p: Pt): Pt => [Math.min(1, Math.max(0, p[0] / (w - 1))), Math.min(1, Math.max(0, p[1] / (h - 1)))];
  return [n(tl), n(tr), n(br), n(bl)];
}

const dist = (a: Pt, b: Pt) => Math.hypot(a[0] - b[0], a[1] - b[1]);

export type Look = 'color' | 'gray' | 'bw';

/** The page inside `q`, straightened, at most `max` pixels on its longest side. */
export function straighten(src: HTMLCanvasElement, q: Quad, look: Look, max = 2200): HTMLCanvasElement {
  const P = q.map(([x, y]) => [x * src.width, y * src.height]) as Quad;
  let ow = Math.max(dist(P[0], P[1]), dist(P[3], P[2]));
  let oh = Math.max(dist(P[0], P[3]), dist(P[1], P[2]));
  const k = Math.min(1, max / Math.max(ow, oh));
  ow = Math.max(16, Math.round(ow * k));
  oh = Math.max(16, Math.round(oh * k));
  // Unit square → quad (Heckbert).
  const [[x0, y0], [x1, y1], [x2, y2], [x3, y3]] = P;
  const dx1 = x1 - x2, dx2 = x3 - x2, dx3 = x0 - x1 + x2 - x3;
  const dy1 = y1 - y2, dy2 = y3 - y2, dy3 = y0 - y1 + y2 - y3;
  let g = 0, hh = 0;
  if (Math.abs(dx3) > 1e-9 || Math.abs(dy3) > 1e-9) {
    const det = dx1 * dy2 - dx2 * dy1 || 1e-9;
    g = (dx3 * dy2 - dx2 * dy3) / det;
    hh = (dx1 * dy3 - dx3 * dy1) / det;
  }
  const a = x1 - x0 + g * x1, b = x3 - x0 + hh * x3, c = x0;
  const d = y1 - y0 + g * y1, e = y3 - y0 + hh * y3, f = y0;
  const sw = src.width, sh = src.height;
  const sp = src.getContext('2d', { willReadFrequently: true })!.getImageData(0, 0, sw, sh).data;
  const out = document.createElement('canvas');
  out.width = ow;
  out.height = oh;
  const octx = out.getContext('2d')!;
  const img = octx.createImageData(ow, oh);
  const op = img.data;
  for (let j = 0; j < oh; j++) {
    const v = (j + 0.5) / oh;
    for (let i = 0; i < ow; i++) {
      const u = (i + 0.5) / ow;
      const z = g * u + hh * v + 1;
      const sx = Math.min(sw - 1, Math.max(0, (a * u + b * v + c) / z));
      const sy = Math.min(sh - 1, Math.max(0, (d * u + e * v + f) / z));
      // Bilinear.
      const xi = sx | 0, yi = sy | 0;
      const fx = sx - xi, fy = sy - yi;
      const xn = Math.min(sw - 1, xi + 1), yn = Math.min(sh - 1, yi + 1);
      const i00 = (yi * sw + xi) * 4, i10 = (yi * sw + xn) * 4, i01 = (yn * sw + xi) * 4, i11 = (yn * sw + xn) * 4;
      const o = (j * ow + i) * 4;
      for (let ch = 0; ch < 3; ch++) {
        const top = sp[i00 + ch] * (1 - fx) + sp[i10 + ch] * fx;
        const bot = sp[i01 + ch] * (1 - fx) + sp[i11 + ch] * fx;
        op[o + ch] = top * (1 - fy) + bot * fy;
      }
      op[o + 3] = 255;
    }
  }
  tidy(op, look);
  octx.putImageData(img, 0, 0);
  return out;
}

/** Stretch the levels (paper white, ink dark); gray or black-and-white when asked. */
function tidy(p: Uint8ClampedArray, look: Look) {
  const hist = new Array(256).fill(0);
  const n = p.length / 4;
  for (let i = 0; i < p.length; i += 4) hist[(p[i] * 77 + p[i + 1] * 150 + p[i + 2] * 29) >> 8]++;
  const at = (frac: number) => {
    let acc = 0;
    for (let v = 0; v < 256; v++) if ((acc += hist[v]) >= n * frac) return v;
    return 255;
  };
  const lo = at(0.02);
  const hi = Math.max(lo + 30, at(0.9));
  const scale = 255 / (hi - lo);
  for (let i = 0; i < p.length; i += 4) {
    if (look === 'color') {
      for (let ch = 0; ch < 3; ch++) p[i + ch] = (p[i + ch] - lo) * scale;
    } else {
      let y = (((p[i] * 77 + p[i + 1] * 150 + p[i + 2] * 29) >> 8) - lo) * scale;
      if (look === 'bw') y = y > 200 ? 255 : y < 110 ? 0 : (y - 110) * 2.8;
      p[i] = p[i + 1] = p[i + 2] = y;
    }
  }
}

/** JPEG as base64 (no data: prefix). */
export function jpegBase64(c: HTMLCanvasElement, quality = 0.85): string {
  return c.toDataURL('image/jpeg', quality).split(',')[1] ?? '';
}
