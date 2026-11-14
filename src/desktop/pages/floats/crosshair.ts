// The crosshair's look, kept in this PC's browser storage (shared by the
// settings card and the crosshair window, which follows changes live).

export interface CrosshairLook {
  style: 'cross' | 'dot' | 'circle' | 'tcross' | 'chevron';
  color: string;
  size: number;
  thickness: number;
  gap: number;
  outline: boolean;
  dot: boolean;
  opacity: number;
}

export const KEY = 'omnihub.crosshair';
export const DEFAULT: CrosshairLook = { style: 'cross', color: '#00ff88', size: 10, thickness: 2, gap: 4, outline: true, dot: false, opacity: 1 };

export function readLook(): CrosshairLook {
  try {
    return { ...DEFAULT, ...(JSON.parse(localStorage.getItem(KEY) ?? '{}') as Partial<CrosshairLook>) };
  } catch {
    return DEFAULT;
  }
}

export function saveLook(l: CrosshairLook) {
  try {
    localStorage.setItem(KEY, JSON.stringify(l));
  } catch {
    /* not kept */
  }
}
