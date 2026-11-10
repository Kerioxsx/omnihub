// Settings → Look: the phone's theme (matching the PC or an elegant
// palette), a photo wallpaper and glass.

import { wallSummary } from '@shared/palette';
import { Check, ImagePlus, Moon, Sun, SunMoon, Trash2 } from 'lucide-react';
import { useRef, useState } from 'react';
import { PRESETS, type Preset, removePhoneWallpaper, resolve, setLook, uploadPhoneWallpaper, useLook } from '../lib/look';
import { getThemePref, setThemePref, type ThemePref } from '../lib/theme';
import { toast } from '../state';
import { Button, Segmented, Switch } from '../ui/common';
import { cx, errorMessage } from '../lib/util';

const PC_NAMES: Record<string, string> = { classic: 'Classic', photo: 'Your photo', aurora: 'Aurora', synthwave: 'Synthwave', ocean: 'Ocean', galaxy: 'Galaxy', sakura: 'Sakura', ember: 'Ember', minecraft: 'Minecraft', matrix: 'Matrix', rain: 'Rain', snowfall: 'Snowfall', fireflies: 'Fireflies', lava: 'Lava lamp', clouds: 'Clouds', bankai: 'Bankai', hollow: 'Hollow', mugetsu: 'Mugetsu', hakka: 'Hakka no Togame', soul: 'Soul Society', hueco: 'Hueco Mundo', karakura: 'Karakura night', senbon: 'Senbonzakura' };

export function LookSection() {
  const { prefs, pc, wallUrl, resolved } = useLook();
  const [theme, setTheme] = useState<ThemePref>(getThemePref());
  const [busy, setBusy] = useState(false);
  const file = useRef<HTMLInputElement>(null);
  const themes: ('pc' | Preset)[] = ['pc', ...(Object.keys(PRESETS) as Preset[])];
  const choose = async (f: File | undefined) => {
    if (!f) return;
    setBusy(true);
    try {
      const w = await uploadPhoneWallpaper(f);
      setLook({ wallpaper: 'mine' });
      toast.success('Wallpaper ready', wallSummary(w));
    } catch (e) {
      toast.error('Couldn’t use that photo', errorMessage(e));
    } finally {
      setBusy(false);
      if (file.current) file.current.value = '';
    }
  };
  const wall = prefs.wallpaper === 'mine' ? pc?.wallpapers.phone : prefs.wallpaper === 'pc' ? pc?.wallpapers.pc : null;
  return (
    <section>
      <h2 className="mb-2.5 px-1 text-[13px] font-semibold uppercase tracking-[0.08em] text-faint">Look</h2>
      <div className="card divide-y divide-[var(--border)]">
        <div className="py-3.5">
          <div className="mb-2.5 px-4 text-[15px] font-semibold">Theme</div>
          <div className="hscroll flex gap-2.5 px-4 pb-1">
            {themes.map((t) => {
              const sw = resolve({ ...prefs, theme: t, fromWallpaper: false });
              const on = prefs.theme === t;
              return (
                <button key={t} onClick={() => setLook({ theme: t })} aria-pressed={on} className={cx('press relative h-[92px] w-[84px] shrink-0 overflow-hidden rounded-2xl border text-left', on ? 'border-accent ring-2 ring-accent/40' : 'border-line')}>
                  <span className="absolute inset-0" style={{ background: sw.bg }} />
                  <span className="absolute left-2.5 top-2.5 flex gap-1">
                    <span className="h-3.5 w-3.5 rounded-full" style={{ background: sw.palette.accent }} />
                    <span className="h-3.5 w-3.5 rounded-full" style={{ background: sw.palette.accent2 }} />
                  </span>
                  {on && (
                    <span className="absolute right-2 top-2 grid h-5 w-5 place-items-center rounded-full bg-accent text-white">
                      <Check size={13} strokeWidth={3} />
                    </span>
                  )}
                  <span className={cx('absolute inset-x-2.5 bottom-2 text-[12.5px] font-semibold leading-tight', sw.palette.light ? 'text-[#15151f]' : 'text-white')}>
                    {t === 'pc' ? 'Match PC' : PRESETS[t].name}
                    {t === 'pc' && pc && <span className="block text-[10.5px] font-medium opacity-70">{PC_NAMES[pc.look] ?? pc.look}</span>}
                  </span>
                </button>
              );
            })}
          </div>
          {prefs.theme === 'classic' && (
            <div className="mt-3 px-4">
              <Segmented
                value={theme}
                onChange={(t) => {
                  setTheme(t);
                  setThemePref(t);
                  setLook({});
                }}
                options={[
                  { value: 'system', label: <><SunMoon size={15} /> System</> },
                  { value: 'dark', label: <><Moon size={15} /> Dark</> },
                  { value: 'light', label: <><Sun size={15} /> Light</> },
                ]}
              />
            </div>
          )}
        </div>
        <div className="px-4 py-3.5">
          <div className="mb-2.5 text-[15px] font-semibold">Wallpaper</div>
          <Segmented
            value={prefs.wallpaper}
            onChange={(w) => setLook({ wallpaper: w })}
            options={[
              { value: 'none', label: 'None' },
              { value: 'pc', label: 'PC’s photo' },
              { value: 'mine', label: 'My photo' },
            ]}
          />
          {prefs.wallpaper !== 'none' && (
            <div className="mt-3 flex items-center gap-3">
              <div className="relative h-[86px] w-[52px] shrink-0 overflow-hidden rounded-xl border border-line bg-surface-2">{wallUrl && wall ? <img src={wallUrl} alt="" className="h-full w-full object-cover" /> : null}</div>
              <div className="min-w-0 flex-1 text-[13px] text-dim">
                {wall ? (
                  wallSummary(wall)
                ) : prefs.wallpaper === 'pc' ? (
                  'The PC has no photo yet. Pick one in OmniHub on the PC: Settings → Appearance → Your photo.'
                ) : (
                  'Pick any photo: it fills the screen, sized sharp for this phone.'
                )}
                {prefs.wallpaper === 'mine' && (
                  <div className="mt-2 flex gap-2">
                    <Button size="sm" onClick={() => file.current?.click()} disabled={busy}>
                      <ImagePlus size={15} /> {busy ? 'Preparing…' : wall ? 'Change' : 'Choose a photo'}
                    </Button>
                    {wall && (
                      <Button size="sm" variant="ghost" onClick={() => void removePhoneWallpaper().catch(() => undefined)}>
                        <Trash2 size={15} /> Remove
                      </Button>
                    )}
                  </div>
                )}
              </div>
              <input ref={file} type="file" accept="image/*" className="hidden" onChange={(e) => void choose(e.target.files?.[0])} />
            </div>
          )}
        </div>
        <label className="flex items-center gap-3 px-4 py-3.5">
          <div className="min-w-0 flex-1">
            <div className="text-[15px] font-semibold">Colours from the wallpaper</div>
            <div className="text-[13px] text-dim">The accent and light or dark follow the photo.</div>
          </div>
          <Switch on={prefs.fromWallpaper} onChange={(v) => setLook({ fromWallpaper: v })} label="Colours from the wallpaper" disabled={!resolved.wall} />
        </label>
        <label className="flex items-center gap-3 px-4 py-3.5">
          <div className="min-w-0 flex-1">
            <div className="text-[15px] font-semibold">Glass</div>
            <div className="text-[13px] text-dim">Frosted, see-through cards and bars. Off is a little lighter on older phones.</div>
          </div>
          <Switch on={prefs.glass} onChange={(v) => setLook({ glass: v })} label="Glass" />
        </label>
      </div>
    </section>
  );
}
