// Colours read from a photo wallpaper (see wallpaper.rs), turned into the
// CSS variables both apps use, so the whole theme matches the photo.

export interface Palette {
  accent: string;
  accent2: string;
  light: boolean;
  tint: string;
}

export interface WallInfo {
  slot: 'pc' | 'phone';
  version: number;
  width: number;
  height: number;
  sourceWidth: number;
  sourceHeight: number;
  upscaled: boolean;
  palette: Palette;
  /** The gallery photo it came from ("" when set directly). */
  source?: string;
}

function rgba(hex: string, a: number): string {
  const n = parseInt(hex.slice(1), 16);
  return `rgba(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}, ${a})`;
}

const VARS = ['--accent', '--accent-2', '--accent-soft', '--accent-glow', '--photo-tint'];

/** Use the photo's colours (or go back to the theme's with null). */
export function applyPalette(p: Palette | null, root: HTMLElement = document.documentElement) {
  if (!p) {
    VARS.forEach((v) => root.style.removeProperty(v));
    return;
  }
  root.style.setProperty('--accent', p.accent);
  root.style.setProperty('--accent-2', p.accent2);
  root.style.setProperty('--accent-soft', rgba(p.accent, p.light ? 0.14 : 0.18));
  root.style.setProperty('--accent-glow', rgba(p.accent, p.light ? 0.32 : 0.45));
  root.style.setProperty('--photo-tint', p.tint);
}

/** "3840 × 2160, enlarged 2.4× and sharpened". */
export function wallSummary(w: WallInfo): string {
  const k = w.width / w.sourceWidth;
  return `${w.width} × ${w.height}${w.upscaled ? `, enlarged ${k.toFixed(1)}× from ${w.sourceWidth} × ${w.sourceHeight} and sharpened` : w.sourceWidth > w.width ? `, brought down from ${w.sourceWidth} × ${w.sourceHeight}` : ''}`;
}
