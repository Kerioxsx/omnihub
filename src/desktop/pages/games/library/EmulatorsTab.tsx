// Emulators: the ones found on this PC (or added by hand), your ROM folders
// as a box-art grid, Play in the right emulator, and a Game Boy Advance
// save straight into the Pokémon save editor.

import type { EmuState, Rom } from '@shared/types';
import { Cpu, FolderPlus, Pencil, Play, Plus, RefreshCw, Settings2, Trash2, X } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { api, errorText } from '../../../api';
import { Button, IconButton } from '../../../components/ui/Button';
import { Badge, Card } from '../../../components/ui/Card';
import { Segmented, Select } from '../../../components/ui/Form';
import { EmptyState } from '../../../components/ui/States';
import { navigate } from '../../../lib/router';
import { toast } from '../../../state/toasts';
import { BiosCard, DatCard, GetEmulators, PadSetupCard, RetroAchievementsCard, SavesCard } from './EmuTools';
import { RomOptions } from './RomOptions';

const PLATFORM_NAMES: Record<string, string> = { gba: 'Game Boy Advance', gb: 'Game Boy', gbc: 'Game Boy Color', nds: 'Nintendo DS', '3ds': 'Nintendo 3DS', nes: 'NES', snes: 'SNES', n64: 'Nintendo 64', gc: 'GameCube', wii: 'Wii', wiiu: 'Wii U', switch: 'Switch', genesis: 'Mega Drive', psx: 'PlayStation', ps2: 'PlayStation 2', psp: 'PSP', ps3: 'PlayStation 3' };

function RomCover({ rom }: { rom: Rom }) {
  const [failed, setFailed] = useState(false);
  const clean = rom.name.replace(/\s*\(.*?\)|\s*\[.*?\]/g, '');
  if (rom.cover && !failed) return <img src={rom.cover} alt="" loading="lazy" onError={() => setFailed(true)} className="h-full w-full object-contain p-2" />;
  return (
    <div className="flex h-full w-full flex-col justify-end bg-gradient-to-br from-surface-3 to-surface p-3">
      <span className="text-[11px] uppercase tracking-wider text-faint">{PLATFORM_NAMES[rom.platform] ?? rom.platform}</span>
      <span className="font-display text-[14px] font-semibold leading-tight text-fg">{clean}</span>
    </div>
  );
}

export function EmulatorsTab() {
  const [s, setS] = useState<EmuState | null>(null);
  const [busy, setBusy] = useState(false);
  const [platform, setPlatform] = useState('all');
  const [opts, setOpts] = useState<Rom | null>(null);
  const run = (p: Promise<EmuState>) => void p.then(setS, (e) => toast.error('Something went wrong', errorText(e)));
  useEffect(() => {
    void api.emulators.state().then((st) => {
      setS(st);
      if (st.roms.length === 0) {
        setBusy(true);
        void api.emulators.refresh().then(setS, () => undefined).finally(() => setBusy(false));
      }
    });
  }, []);
  const platforms = useMemo(() => [...new Set((s?.roms ?? []).map((r) => r.platform))], [s]);
  const roms = (s?.roms ?? []).filter((r) => platform === 'all' || r.platform === platform);
  const play = (r: Rom) => void api.emulators.launch(r.id).then(() => toast.success(`Starting ${r.name.replace(/\s*\(.*?\)/g, '')}`), (e) => toast.error('Couldn’t start it', errorText(e)));
  if (!s) return null;
  return (
    <div className="space-y-5">
      <Card className="space-y-3 p-4">
        <div className="flex flex-wrap items-center gap-2">
          <Cpu size={16} className="text-accent" />
          <span className="text-[14px] font-semibold text-fg">Emulators</span>
          <Button size="sm" icon={RefreshCw} loading={busy} className="ml-auto" onClick={() => (setBusy(true), void api.emulators.refresh().then(setS).finally(() => setBusy(false)))}>
            Look again
          </Button>
          <Button
            size="sm"
            icon={Plus}
            onClick={async () => {
              const [exe] = await api.app.pickFiles('Pick the emulator’s .exe');
              if (exe) run(api.emulators.add(exe));
            }}
          >
            Add one
          </Button>
        </div>
        {s.emulators.length === 0 ? (
          <p className="text-[12.5px] text-faint">None found yet. RetroArch, mGBA, melonDS, Dolphin, PCSX2, PPSSPP, DuckStation, Azahar, Ryujinx, Cemu and RPCS3 are found by themselves in the usual folders; add others by their .exe.</p>
        ) : (
          <div className="flex flex-wrap gap-2">
            {s.emulators.map((e) => (
              <div key={e.id} className="flex items-center gap-2 rounded-xl border border-line bg-surface px-3 py-1.5">
                <span className="text-[13px] font-medium text-fg">{e.name}</span>
                <span className="text-[11.5px] text-faint">{e.platforms.map((p) => PLATFORM_NAMES[p] ?? p).join(', ')}</span>
                <IconButton size="sm" icon={X} label={`Remove ${e.name}`} onClick={() => run(api.emulators.remove(e.id))} />
              </div>
            ))}
          </div>
        )}
        <div className="flex flex-wrap items-center gap-2 border-t border-line pt-3">
          <span className="text-[13px] font-semibold text-fg">ROM folders</span>
          {s.romDirs.map((d) => (
            <Badge key={d} className="max-w-[320px]">
              <span className="truncate">{d}</span>
              <button type="button" aria-label={`Remove ${d}`} onClick={() => run(api.emulators.setDirs(s.romDirs.filter((x) => x !== d)))} className="ml-1 text-faint hover:text-fg">
                <X size={11} />
              </button>
            </Badge>
          ))}
          <Button
            size="sm"
            icon={FolderPlus}
            onClick={async () => {
              const dir = await api.app.pickFolder('Pick a folder with your ROMs');
              if (dir) run(api.emulators.setDirs([...s.romDirs, dir]));
            }}
          >
            Add a folder
          </Button>
        </div>
        {platforms.length > 0 && (
          <div className="flex flex-wrap gap-3 border-t border-line pt-3">
            {platforms.map((p) => {
              const can = s.emulators.filter((e) => e.platforms.includes(p));
              return (
                <label key={p} className="flex items-center gap-2 text-[12.5px] text-dim">
                  {PLATFORM_NAMES[p] ?? p}
                  <Select value={s.chosen[p] ?? ''} onChange={(e) => run(api.emulators.choose(p, e.target.value))} className="w-40">
                    <option value="">{can[0] ? `${can[0].name} (first)` : 'None yet'}</option>
                    {s.emulators.map((e) => (
                      <option key={e.id} value={e.id}>
                        {e.name}
                      </option>
                    ))}
                  </Select>
                </label>
              );
            })}
          </div>
        )}
      </Card>

      {s.roms.length === 0 ? (
        <EmptyState icon={Cpu} title="No ROMs yet" description="Add the folder where you keep your games (.gba, .nds, .iso…). Box art comes from libretro’s collection when the file has its usual name." />
      ) : (
        <>
          {platforms.length > 1 && (
            <Segmented
              label="Platform"
              value={platform}
              onChange={setPlatform}
              options={[{ value: 'all', label: 'All' }, ...platforms.map((p) => ({ value: p, label: PLATFORM_NAMES[p] ?? p }))]}
            />
          )}
          <div className="grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-4">
            {roms.map((r) => (
              <div key={r.id} className="group overflow-hidden rounded-xl border border-line bg-surface">
                <button type="button" onClick={() => play(r)} className="relative block aspect-square w-full bg-surface-2" aria-label={`Play ${r.name}`}>
                  <RomCover rom={r} />
                  <span className="absolute inset-0 grid place-items-center bg-black/50 opacity-0 transition-opacity group-hover:opacity-100">
                    <Play size={34} className="text-white" fill="currentColor" />
                  </span>
                </button>
                <div className="space-y-1 p-2">
                  <div className="truncate text-[12.5px] font-medium text-fg" title={r.name}>
                    {r.name.replace(/\s*\(.*?\)/g, '')}
                  </div>
                  <div className="flex items-center gap-1">
                    <Badge>{PLATFORM_NAMES[r.platform] ?? r.platform}</Badge>
                    {r.save && r.platform === 'gba' && (
                      <IconButton size="sm" icon={Pencil} label="Edit the save (Pokémon save editor)" onClick={() => navigate('tools', { tab: 'pokemon', open: r.save ?? '' })} />
                    )}
                    {r.save && r.platform !== 'gba' && <Badge tone="good">Save</Badge>}
                    <IconButton size="sm" icon={Settings2} label="Options, cheats and netplay" className="ml-auto" onClick={() => setOpts(r)} />
                  </div>
                </div>
              </div>
            ))}
          </div>
        </>
      )}
      <p className="flex items-center gap-1.5 text-[11.5px] text-faint">
        <Trash2 size={11} /> OmniHub doesn’t download games: it plays the ones you already have.
      </p>
      {opts && <RomOptions rom={opts} state={s} onClose={() => setOpts(null)} onState={setS} />}
      <GetEmulators onState={setS} />
      <div className="grid gap-5 xl:grid-cols-2">
        <BiosCard />
        <DatCard onState={setS} />
        <SavesCard />
        <PadSetupCard />
        <RetroAchievementsCard />
      </div>
    </div>
  );
}
