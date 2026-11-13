// The Minecraft hub inside the Client tab: get any launcher or client, and
// install any Minecraft version with Forge, NeoForge, Fabric, Quilt or Legacy
// Fabric as a Minecraft Launcher profile (each in its own folder), then add
// mods for it from Modrinth.

import type { McPackSearchHit } from '@shared/types';
import { Download, ExternalLink, Globe, Hammer, Package, Rocket } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Badge, Card, CardHeader } from '../../components/ui/Card';
import { SearchInput, Segmented, Select } from '../../components/ui/Form';
import { useDebounced } from '../../lib/hooks';
import { toast } from '../../state/toasts';

interface App {
  name: string;
  by: string;
  note: string;
  url: string;
  winget?: string;
  /** Opens Vesper's own section instead. */
  vesper?: boolean;
}

const LAUNCHERS: App[] = [
  { name: 'Minecraft Launcher', by: 'Mojang', note: 'The official launcher for Java and Bedrock; sign in with Microsoft.', url: 'https://www.minecraft.net/download', winget: 'Mojang.MinecraftLauncher' },
  { name: 'Prism Launcher', by: 'Open source', note: 'Many separate instances, any loader, modpacks from Modrinth and CurseForge.', url: 'https://prismlauncher.org/download/', winget: 'PrismLauncher.PrismLauncher' },
  { name: 'Modrinth App', by: 'Modrinth', note: 'Modrinth’s own launcher for its mods and modpacks.', url: 'https://modrinth.com/app', winget: 'Modrinth.ModrinthApp' },
  { name: 'CurseForge', by: 'Overwolf', note: 'CurseForge modpacks and mods.', url: 'https://www.curseforge.com/download/app', winget: 'Overwolf.CurseForge' },
  { name: 'ATLauncher', by: 'Open source', note: 'Modpack launcher with its own pack list.', url: 'https://atlauncher.com/downloads', winget: 'ATLauncher.ATLauncher' },
  { name: 'MultiMC', by: 'Open source', note: 'The original multi-instance launcher.', url: 'https://multimc.org/#Download' },
  { name: 'GDLauncher', by: 'Open source', note: 'A modern modpack launcher.', url: 'https://gdlauncher.com/download' },
];

const CLIENTS: App[] = [
  { name: 'Vesper Client', by: 'OmniHub', note: 'OmniHub’s own 1.8.9 PvP client (Forge).', url: '', vesper: true },
  { name: 'Lunar Client', by: 'Moonsworth', note: 'PvP client for many versions (Badlion now launches through it too).', url: 'https://www.lunarclient.com/download', winget: 'Moonsworth.LunarClient' },
  { name: 'Badlion Client', by: 'Moonsworth', note: 'PvP client with its own anti-cheat.', url: 'https://www.badlion.net/download' },
  { name: 'LabyMod', by: 'LabyMedia', note: 'Client with cosmetics, addons and a PvP mode.', url: 'https://www.labymod.net/en/download' },
  { name: 'Feather Client', by: 'Feather', note: 'Performance client with mods built in.', url: 'https://feathermc.com/' },
];

function AppRow({ app, onVesper }: { app: App; onVesper: () => void }) {
  const [busy, setBusy] = useState(false);
  const get = async () => {
    if (app.vesper) return onVesper();
    if (!app.winget) return void api.app.openUrl(app.url);
    setBusy(true);
    try {
      await api.mcHub.winget(app.winget);
      toast.success(`${app.name} is installed`, 'Find it in the Start menu.');
    } catch (e) {
      toast.info(`Opening ${app.name}’s download page`, errorText(e));
      void api.app.openUrl(app.url);
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="flex items-center gap-3 rounded-xl border border-line p-3">
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2 text-[13.5px] font-semibold text-fg">
          {app.name} <span className="text-[11.5px] font-normal text-dim">{app.by}</span>
        </div>
        <div className="text-[12.5px] text-dim">{app.note}</div>
      </div>
      {!app.vesper && (
        <Button variant="ghost" size="sm" icon={ExternalLink} onClick={() => void api.app.openUrl(app.url)}>
          Website
        </Button>
      )}
      <Button variant="secondary" size="sm" icon={Download} loading={busy} onClick={() => void get()}>
        {app.vesper ? 'Open' : app.winget ? 'Install' : 'Download'}
      </Button>
    </div>
  );
}

export function LaunchersCard({ onVesper }: { onVesper: () => void }) {
  return (
    <div className="grid gap-5 xl:grid-cols-2">
      <Card className="p-5">
        <CardHeader icon={Rocket} title="Launchers" subtitle="Install with one click (Windows' winget, from the publisher), or open the official download page — also for a Mac or Linux PC." />
        <div className="mt-4 grid gap-2">{LAUNCHERS.map((a) => <AppRow key={a.name} app={a} onVesper={onVesper} />)}</div>
      </Card>
      <Card className="p-5">
        <CardHeader icon={Globe} title="Clients" subtitle="PvP and performance clients. Only official downloads; check each server's rules for client mods." />
        <div className="mt-4 grid gap-2">{CLIENTS.map((a) => <AppRow key={a.name} app={a} onVesper={onVesper} />)}</div>
      </Card>
    </div>
  );
}

const LOADERS = [
  { value: 'fabric', label: 'Fabric' },
  { value: 'forge', label: 'Forge' },
  { value: 'neoforge', label: 'NeoForge' },
  { value: 'quilt', label: 'Quilt' },
  { value: 'legacyfabric', label: 'Legacy Fabric' },
  { value: 'vanilla', label: 'Vanilla' },
];

export function LoadersCard() {
  const [loader, setLoader] = useState('fabric');
  const [versions, setVersions] = useState<string[]>([]);
  const [mc, setMc] = useState('');
  const [busy, setBusy] = useState(false);
  const [made, setMade] = useState<string | null>(null);
  const [query, setQuery] = useState('');
  const q = useDebounced(query, 350);
  const [hits, setHits] = useState<McPackSearchHit[]>([]);
  const [have, setHave] = useState<string[]>([]);
  const [adding, setAdding] = useState<string | null>(null);

  useEffect(() => {
    setVersions([]);
    setMc('');
    void api.mcHub.versions(loader).then(
      (v) => {
        setVersions(v);
        setMc(v[0] ?? '');
      },
      (e) => toast.error('Couldn’t load versions', errorText(e)),
    );
  }, [loader]);
  useEffect(() => {
    if (!mc) return;
    void api.mcHub.modsIn(loader, mc).then(setHave, () => setHave([]));
  }, [loader, mc, made]);
  useEffect(() => {
    if (!mc || loader === 'vanilla') return setHits([]);
    let live = true;
    void api.mcHub.searchMods(loader, mc, q).then(
      (h) => live && setHits(h),
      () => live && setHits([]),
    );
    return () => {
      live = false;
    };
  }, [loader, mc, q]);

  const install = async () => {
    setBusy(true);
    try {
      const m = await api.mcHub.install(loader, mc);
      setMade(m.profile);
      toast.success(`${m.profile} is in the Minecraft Launcher`, m.loaderVersion ? `Loader ${m.loaderVersion}. Pick the profile and press Play.` : 'Pick the profile and press Play.');
    } catch (e) {
      toast.error('Not installed', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  const addMod = async (h: McPackSearchHit) => {
    setAdding(h.project);
    try {
      const files = await api.mcHub.installMod(loader, mc, h.project);
      toast.success(`Added ${h.title}`, files.length > 1 ? `Plus what it needs: ${files.slice(1).join(', ')}` : files[0]);
      setHave(await api.mcHub.modsIn(loader, mc));
    } catch (e) {
      toast.error(`Couldn’t add ${h.title}`, errorText(e));
    } finally {
      setAdding(null);
    }
  };

  return (
    <div className="grid gap-5 xl:grid-cols-[1fr_1.2fr]">
      <Card className="p-5">
        <CardHeader icon={Hammer} title="Loaders for any version" subtitle="Forge, NeoForge, Fabric, Quilt or Legacy Fabric, installed as a profile in the official Minecraft Launcher with its own folder. Java for Forge comes along when needed." />
        <div className="mt-4 grid gap-3">
          <Segmented size="sm" label="Loader" value={loader} onChange={setLoader} options={LOADERS} />
          <div className="flex items-center gap-2">
            <Select value={mc} onChange={(e) => setMc(e.target.value)} aria-label="Minecraft version" className="w-44">
              {versions.length === 0 && <option value="">Loading…</option>}
              {versions.map((v) => (
                <option key={v} value={v}>
                  Minecraft {v}
                </option>
              ))}
            </Select>
            <Button variant="primary" icon={Download} loading={busy} disabled={!mc} onClick={() => void install()}>
              Install
            </Button>
          </div>
          <p className="text-[12.5px] leading-relaxed text-dim">
            Open the Minecraft Launcher once before installing. Forge and NeoForge run their own installer (about a minute). Forge older than 1.12.2 needs its own installer window; for 1.8.9 use Vesper&rsquo;s one-click install.
          </p>
          {made && <Badge tone="good">Installed: {made}</Badge>}
          {have.length > 0 && (
            <div>
              <div className="mb-1 text-[12px] font-medium text-dim">Mods in this version ({have.length})</div>
              <div className="flex flex-wrap gap-1.5">
                {have.map((m) => (
                  <Badge key={m}>{m.replace(/\.jar$/, '')}</Badge>
                ))}
              </div>
            </div>
          )}
        </div>
      </Card>
      <Card className="p-5">
        <CardHeader icon={Package} title="Mods" subtitle={loader === 'vanilla' ? 'Pick a loader to add mods.' : `From Modrinth for ${LOADERS.find((l) => l.value === loader)?.label} ${mc}; what a mod needs comes with it. Install the loader first.`} />
        {loader !== 'vanilla' && (
          <>
            <SearchInput value={query} onChange={setQuery} placeholder="Search mods (Sodium, JEI, minimap…)" className="mt-4" />
            <div className="mt-3 grid max-h-[460px] gap-2 overflow-y-auto pr-1">
              {hits.map((h) => (
                <div key={h.project} className="flex items-center gap-3 rounded-xl border border-line p-2.5">
                  {h.icon ? <img src={h.icon} alt="" className="h-9 w-9 rounded-lg" /> : <div className="h-9 w-9 rounded-lg bg-line" />}
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-[13px] font-semibold text-fg">{h.title}</div>
                    <div className="truncate text-[12px] text-dim">{h.description}</div>
                  </div>
                  <Button variant="secondary" size="sm" loading={adding === h.project} onClick={() => void addMod(h)}>
                    Add
                  </Button>
                </div>
              ))}
              {hits.length === 0 && <div className="py-6 text-center text-[12.5px] text-dim">No mods found for this version.</div>}
            </div>
          </>
        )}
      </Card>
    </div>
  );
}
