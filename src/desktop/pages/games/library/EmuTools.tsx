// Emulator tools under the ROM grid: install and update emulators from
// their official releases (checksums checked), check your BIOS files, and
// tidy ROMs against a DAT.

import { formatBytes } from '@shared/format';
import type { BiosFile, DatReport, EmuOffer, EmuState, RaSummary, SavePlace } from '@shared/types';
import { Archive, CheckCircle2, Gamepad2, Trophy, Copy, Cpu, Download, ExternalLink, FileSearch, FolderOpen, RefreshCw, TriangleAlert } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../../api';
import { Button } from '../../../components/ui/Button';
import { TextInput } from '../../../components/ui/Form';
import { Card, CardHeader } from '../../../components/ui/Card';
import { useEvent } from '../../../lib/hooks';
import { toast } from '../../../state/toasts';

const ELSEWHERE: [string, string][] = [
  ['Dolphin', 'https://dolphin-emu.org/download/'],
  ['PPSSPP', 'https://www.ppsspp.org/download/'],
  ['RetroArch', 'https://www.retroarch.com/?page=platforms'],
];

export function GetEmulators({ onState }: { onState: (s: EmuState) => void }) {
  const [offers, setOffers] = useState<EmuOffer[]>([]);
  const [checking, setChecking] = useState(false);
  const [busy, setBusy] = useState<string | null>(null);
  const [pct, setPct] = useState(0);
  useEffect(() => void api.emulators.offers().then(setOffers, () => undefined), []);
  useEvent<{ id: string; done: number; total: number }>('emu:install', (p) => setPct(p.total ? Math.round((p.done / p.total) * 100) : 0));
  const check = async () => {
    setChecking(true);
    try {
      setOffers(await api.emulators.offers(true));
    } catch (e) {
      toast.error('Couldn’t reach GitHub', errorText(e));
    } finally {
      setChecking(false);
    }
  };
  const install = async (o: EmuOffer) => {
    setBusy(o.id);
    setPct(0);
    try {
      onState(await api.emulators.install(o.id));
      setOffers(await api.emulators.offers());
      toast.success(`${o.name} is ready`, 'Added to your emulators.');
    } catch (e) {
      toast.error(`Couldn’t install ${o.name}`, errorText(e));
    } finally {
      setBusy(null);
    }
  };
  return (
    <Card className="p-5">
      <CardHeader
        icon={Download}
        title="Get emulators"
        subtitle="From each emulator’s official GitHub releases only; a download whose checksum doesn’t match GitHub’s is deleted. Games and BIOS files are never downloaded — use your own."
        actions={
          <Button size="sm" variant="ghost" icon={RefreshCw} loading={checking} onClick={() => void check()}>
            Check for updates
          </Button>
        }
      />
      <div className="mt-4 grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
        {offers.map((o) => {
          const update = o.installed && o.latest && o.installed !== o.latest;
          return (
            <div key={o.id} className="flex items-center gap-3 rounded-xl border border-line px-3 py-2">
              <div className="min-w-0 flex-1">
                <div className="text-[13.5px] font-medium text-fg">{o.name}</div>
                <div className="truncate text-[11.5px] text-faint">{o.installed ? `Installed ${o.installed}${update ? ` · ${o.latest} out` : ''}` : o.latest ? `Newest ${o.latest}` : o.platforms.join(', ').toUpperCase()}</div>
              </div>
              <Button size="sm" variant={update || !o.installed ? 'primary' : 'ghost'} disabled={!!busy || (!!o.installed && !update)} loading={busy === o.id} onClick={() => void install(o)}>
                {busy === o.id ? `${pct}%` : update ? 'Update' : o.installed ? 'Up to date' : 'Install'}
              </Button>
            </div>
          );
        })}
      </div>
      <div className="mt-3 flex flex-wrap items-center gap-x-4 gap-y-1 text-[12.5px] text-dim">
        <span>From their own sites:</span>
        {ELSEWHERE.map(([n, url]) => (
          <button key={n} type="button" onClick={() => void api.app.openUrl(url)} className="inline-flex items-center gap-1 text-accent hover:underline">
            {n} <ExternalLink size={11} />
          </button>
        ))}
      </div>
    </Card>
  );
}

const name = (p: string) => p.split(/[\\/]/).pop() ?? p;

export function BiosCard() {
  const [files, setFiles] = useState<BiosFile[] | null>(null);
  const pick = async () => {
    const dir = await api.app.pickFolder('Pick the folder with your BIOS files');
    if (!dir) return;
    try {
      setFiles(await api.emulators.checkBios(dir));
    } catch (e) {
      toast.error('Couldn’t read that folder', errorText(e));
    }
  };
  return (
    <Card className="p-5">
      <CardHeader
        icon={Cpu}
        title="BIOS checker"
        subtitle="Some emulators need a BIOS dumped from your own console. Pick the folder where you keep them: each file is checked against the known good dump. OmniHub never downloads a BIOS."
        actions={
          <Button size="sm" onClick={() => void pick()}>
            Check a folder
          </Button>
        }
      />
      {files && (
        <div className="mt-3 divide-y divide-line rounded-xl border border-line">
          {files.map((f) => (
            <div key={f.platform + f.name} className="flex items-start gap-2.5 px-3 py-2 text-[12.5px]">
              {f.status === 'good' || f.status === 'ps2' ? <CheckCircle2 size={15} className="mt-0.5 shrink-0 text-good" /> : f.status === 'different' ? <TriangleAlert size={15} className="mt-0.5 shrink-0 text-warn" /> : <span className="mt-1 h-2.5 w-2.5 shrink-0 rounded-full border border-line" />}
              <span className={f.status === 'missing' ? 'text-faint' : 'text-fg'}>{f.detail}</span>
            </div>
          ))}
        </div>
      )}
    </Card>
  );
}

export function DatCard({ onState }: { onState: (s: EmuState) => void }) {
  const [report, setReport] = useState<DatReport | null>(null);
  const [busy, setBusy] = useState(false);
  const check = async () => {
    const [dat] = await api.app.pickFiles('Pick a No-Intro or Redump .dat file');
    if (!dat) return;
    setBusy(true);
    try {
      setReport(await api.emulators.checkDat(dat));
    } catch (e) {
      toast.error('Couldn’t check against it', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  const apply = async () => {
    try {
      const r = await api.emulators.applyDat();
      onState(r.state);
      setReport(null);
      toast.success(`Renamed ${r.renamed} ROM${r.renamed === 1 ? '' : 's'}`, r.skipped.length ? `Skipped: ${r.skipped.slice(0, 3).join('; ')}` : 'Their saves were renamed with them.');
    } catch (e) {
      toast.error('Couldn’t rename', errorText(e));
    }
  };
  const count = (st: string) => report?.checked.filter((c) => c.status === st).length ?? 0;
  return (
    <Card className="p-5">
      <CardHeader
        icon={FileSearch}
        title="ROM organizer"
        subtitle="Check your ROM folders against a DAT file (from No-Intro or Redump): good dumps get their proper names, duplicates are found. Renames stay in the same folder and take the save file along."
        actions={
          <Button size="sm" loading={busy} onClick={() => void check()}>
            Check against a DAT
          </Button>
        }
      />
      {report && (
        <div className="mt-3 space-y-3">
          <div className="flex flex-wrap gap-x-5 gap-y-1 text-[12.5px] text-dim">
            <span>
              <b className="text-fg">{count('ok')}</b> already right
            </span>
            <span>
              <b className="text-fg">{count('rename')}</b> to rename
            </span>
            <span>
              <b className="text-fg">{count('unknown')}</b> not in the DAT (hacks, bad dumps, other regions)
            </span>
            <span>
              <b className="text-fg">{report.duplicates.length}</b> duplicate{report.duplicates.length === 1 ? '' : 's'}
            </span>
          </div>
          {count('rename') > 0 && (
            <div className="max-h-48 divide-y divide-line overflow-y-auto rounded-xl border border-line">
              {report.checked
                .filter((c) => c.status === 'rename')
                .map((c) => (
                  <div key={c.path} className="px-3 py-1.5 text-[12px]">
                    <span className="text-faint">{name(c.path)}</span> → <span className="text-fg">{c.proper}</span>
                  </div>
                ))}
            </div>
          )}
          {report.duplicates.length > 0 && (
            <div className="rounded-xl border border-line px-3 py-2 text-[12px] text-dim">
              <div className="mb-1 flex items-center gap-1.5 font-semibold text-fg">
                <Copy size={12} /> Same game twice
              </div>
              {report.duplicates.slice(0, 8).map((d) => (
                <div key={d.join('|')} className="truncate">
                  {d.map(name).join('  =  ')}
                </div>
              ))}
            </div>
          )}
          {count('rename') > 0 && (
            <Button variant="primary" onClick={() => void apply()}>
              Rename {count('rename')}
            </Button>
          )}
        </div>
      )}
    </Card>
  );
}

export function SavesCard() {
  const [places, setPlaces] = useState<SavePlace[] | null>(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => void api.emulators.saves().then(setPlaces, () => setPlaces([])), []);
  const backup = async () => {
    setBusy(true);
    try {
      const zip = await api.emulators.backupSaves();
      toast.success('Saves backed up', zip, { action: { label: 'Show', run: () => void api.app.revealPath(zip) } });
    } catch (e) {
      toast.error('Couldn’t back up', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Card className="p-5">
      <CardHeader
        icon={Archive}
        title="Saves and memory cards"
        subtitle="Where each emulator keeps saves, memory cards and save states on this PC, with one backup of all of them (Documents → OmniHub Backups)."
        actions={
          <Button size="sm" loading={busy} onClick={() => void backup()}>
            Back up all
          </Button>
        }
      />
      {places && places.length === 0 && <p className="mt-3 text-[12.5px] text-faint">No emulator saves found yet.</p>}
      {places && places.length > 0 && (
        <div className="mt-3 divide-y divide-line rounded-xl border border-line">
          {places.map((p) => (
            <div key={p.path} className="flex items-center gap-3 px-3 py-2 text-[12.5px]">
              <span className="w-24 shrink-0 font-medium text-fg">{p.emulator}</span>
              <span className="min-w-0 flex-1 truncate text-dim" title={p.path}>
                {p.what} · {p.files} file{p.files === 1 ? '' : 's'} · {formatBytes(p.bytes)}
              </span>
              <button type="button" onClick={() => void api.app.openPath(p.path)} className="text-faint hover:text-fg" aria-label={`Open ${p.path}`}>
                <FolderOpen size={14} />
              </button>
            </div>
          ))}
        </div>
      )}
    </Card>
  );
}

export function PadSetupCard() {
  const [busy, setBusy] = useState(false);
  const go = async () => {
    setBusy(true);
    try {
      const r = await api.emulators.padSetup();
      if (r.length === 0) toast.info('Nothing to set up', 'Install DuckStation or PCSX2 first.');
      for (const x of r) {
        if (x.status === 'set') toast.success(`${x.emulator}: controller mapped`, 'Port 1, the usual layout. The old settings were kept beside it.');
        else if (x.status === 'already') toast.info(`${x.emulator}: already mapped`, 'Your own mapping stays.');
        else toast.info(`${x.emulator}: start it once first`, 'Then come back and set the controller up.');
      }
    } catch (e) {
      toast.error('Couldn’t set it up', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Card className="p-5">
      <CardHeader
        icon={Gamepad2}
        title="Controller setup"
        subtitle="Maps an Xbox-style controller (or a PlayStation one Windows sees through SDL) to port 1 in DuckStation and PCSX2, the same layout their automatic mapping uses. A mapping you made yourself stays."
        actions={
          <Button size="sm" loading={busy} onClick={() => void go()}>
            Set up
          </Button>
        }
      />
    </Card>
  );
}

export function RetroAchievementsCard() {
  const [user, setUser] = useState<string | null | undefined>(undefined);
  const [name, setName] = useState('');
  const [key, setKey] = useState('');
  const [sum, setSum] = useState<RaSummary | null>(null);
  const [busy, setBusy] = useState(false);
  const load = async () => {
    setBusy(true);
    try {
      setSum(await api.emulators.raSummary());
    } catch (e) {
      toast.error('Couldn’t load RetroAchievements', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  useEffect(() => {
    void api.emulators.raUser().then((u) => {
      setUser(u);
      if (u) void load();
    }, () => setUser(null));
  }, []);
  const connect = async () => {
    try {
      const u = await api.emulators.raConnect(name, key);
      setUser(u);
      setKey('');
      if (u) void load();
    } catch (e) {
      toast.error('Couldn’t connect', errorText(e));
    }
  };
  if (user === undefined) return null;
  return (
    <Card className="p-5">
      <CardHeader
        icon={Trophy}
        title="RetroAchievements"
        subtitle={user ? `Signed in as ${user}. Unlock achievements in emulators that support it (RetroArch, DuckStation, PCSX2, PPSSPP…).` : 'Your recent games and unlocks from retroachievements.org. Use your username and the web API key from your settings page there; it stays on this PC, encrypted.'}
        actions={
          user ? (
            <div className="flex gap-2">
              <Button size="sm" variant="ghost" icon={RefreshCw} loading={busy} onClick={() => void load()}>
                Refresh
              </Button>
              <Button size="sm" variant="ghost" onClick={() => void api.emulators.raConnect('', '').then(setUser).then(() => setSum(null))}>
                Disconnect
              </Button>
            </div>
          ) : undefined
        }
      />
      {!user && (
        <div className="mt-3 flex flex-wrap gap-2">
          <TextInput value={name} onChange={(e) => setName(e.target.value)} placeholder="Username" className="w-40" />
          <TextInput value={key} onChange={(e) => setKey(e.target.value)} placeholder="Web API key" type="password" className="min-w-[200px] flex-1" />
          <Button variant="primary" disabled={!name.trim() || !key.trim()} onClick={() => void connect()}>
            Connect
          </Button>
        </div>
      )}
      {sum && (
        <div className="mt-3 space-y-3">
          <div className="text-[12.5px] text-dim">
            <b className="text-fg">{sum.points.toLocaleString()}</b> points{sum.rank ? ` · rank ${sum.rank.toLocaleString()}` : ''}
          </div>
          <div className="space-y-1.5">
            {sum.games.slice(0, 6).map((g) => (
              <div key={g.id} className="flex items-center gap-3 text-[12.5px]">
                {g.icon ? <img src={g.icon} alt="" className="h-8 w-8 rounded" /> : <span className="h-8 w-8 rounded bg-surface-3" />}
                <div className="min-w-0 flex-1">
                  <div className="truncate text-fg">{g.title}</div>
                  <div className="text-faint">{g.console}</div>
                </div>
                <span className="tabular text-dim">
                  {g.achieved}/{g.total}
                </span>
              </div>
            ))}
          </div>
          {sum.recent.length > 0 && (
            <div className="border-t border-line pt-2 text-[12px] text-dim">
              Latest: <span className="text-fg">{sum.recent[0].title}</span> in {sum.recent[0].game}
              {sum.recent[0].hardcore ? ' (hardcore)' : ''}
            </div>
          )}
        </div>
      )}
    </Card>
  );
}
