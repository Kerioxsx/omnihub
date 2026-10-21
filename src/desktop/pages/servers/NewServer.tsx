// Making a server: a fresh world of any type and version, or one made from
// a Modrinth modpack, one of the user's mixes, or a .mrpack file. The EULA
// is the user's to accept; nothing starts until they do.

import type { McMix, McPackHit, McPackVersion, SrvDifficulty, SrvFlavor, SrvFrom, SrvGamemode, SrvNew } from '@shared/types';
import { Blocks, ExternalLink, FileArchive, Globe, Package, Server, Sparkles } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Badge, Spinner } from '../../components/ui/Card';
import { Checkbox, Field, SearchInput, Segmented, Select, Switch, TextInput } from '../../components/ui/Form';
import { Modal } from '../../components/ui/Overlay';
import { Callout } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { useDebounced } from '../../lib/hooks';
import { toast } from '../../state/toasts';
import { EVERYONE_NOTE, FLAVORS, JoinChoice, flavorLabel } from './shared';

type Source = 'fresh' | 'modpack' | 'mix' | 'file';

const LOADER_FLAVOR: Record<string, SrvFlavor> = { fabric: 'fabric', quilt: 'quilt', forge: 'forge', neoforge: 'neoForge' };

function memoryHint(mb: number, flavor: SrvFlavor, modded: boolean): string {
  if (modded || flavor === 'fabric' || flavor === 'forge' || flavor === 'neoForge' || flavor === 'quilt') return mb < 4096 ? 'Modded servers usually need 4 GB or more.' : 'Enough for most modpacks; big ones like 8 GB.';
  return mb < 2048 ? 'Fine for a few friends.' : 'Room for a busy server.';
}

export function NewServerModal({ open, onClose, onMade, preset }: { open: boolean; onClose: () => void; onMade: (id: string) => void; preset?: SrvFrom | null }) {
  const [source, setSource] = useState<Source>('fresh');
  const [name, setName] = useState('');
  const [flavor, setFlavor] = useState<SrvFlavor>('paper');
  const [versions, setVersions] = useState<string[] | null>(null);
  const [mc, setMc] = useState('');
  const [memory, setMemory] = useState(4096);
  const [maxPlayers, setMaxPlayers] = useState(10);
  const [gamemode, setGamemode] = useState<SrvGamemode>('survival');
  const [difficulty, setDifficulty] = useState<SrvDifficulty>('normal');
  const [seed, setSeed] = useState('');
  const [onlineMode, setOnlineMode] = useState(true);
  const [whitelist, setWhitelist] = useState(true);
  const [eula, setEula] = useState(false);
  const [start, setStart] = useState(true);
  const [saving, setSaving] = useState(false);
  // Modpacks.
  const [query, setQuery] = useState('');
  const q = useDebounced(query, 300);
  const [hits, setHits] = useState<McPackHit[] | null>(null);
  const [pack, setPack] = useState<McPackHit | null>(null);
  const [packVersions, setPackVersions] = useState<McPackVersion[] | null>(null);
  const [packVersion, setPackVersion] = useState('');
  const [mixes, setMixes] = useState<McMix[] | null>(null);
  const [mixId, setMixId] = useState('');
  const [file, setFile] = useState('');

  useEffect(() => {
    if (!open) return;
    setEula(false);
    setSaving(false);
    if (preset?.type === 'mix') {
      setSource('mix');
      setMixId(preset.id);
    } else if (preset?.type === 'file') {
      setSource('file');
      setFile(preset.path);
    }
  }, [open, preset]);

  useEffect(() => {
    if (!open || source !== 'fresh') return;
    setVersions(null);
    void api.servers.versions(flavor).then(
      (v) => {
        setVersions(v);
        setMc((cur) => (v.includes(cur) ? cur : (v[0] ?? '')));
      },
      (e: unknown) => {
        setVersions([]);
        toast.error('Versions not loaded', errorText(e));
      },
    );
  }, [open, source, flavor]);

  useEffect(() => {
    if (!open || source !== 'modpack') return;
    void api.minecraft.search(q).then((r) => setHits(r.hits), () => setHits([]));
  }, [open, source, q]);

  useEffect(() => {
    if (!pack) return;
    setPackVersions(null);
    void api.minecraft.versions(pack.project).then((v) => {
      setPackVersions(v);
      setPackVersion(v[0]?.id ?? '');
    }, () => setPackVersions([]));
  }, [pack]);

  useEffect(() => {
    if (open && source === 'mix') void api.minecraft.mixes().then((m) => {
      setMixes(m);
      setMixId((cur) => cur || m[0]?.id || '');
    }, () => setMixes([]));
  }, [open, source]);

  const pv = packVersions?.find((v) => v.id === packVersion);
  const mix = mixes?.find((m) => m.id === mixId);
  const modded = source !== 'fresh';
  useEffect(() => {
    setMemory(modded ? 6144 : flavor === 'vanilla' || flavor === 'paper' ? 3072 : 6144);
  }, [modded, flavor]);

  const target = useMemo((): { flavor: SrvFlavor; mc: string } | null => {
    if (source === 'fresh') return mc ? { flavor, mc } : null;
    if (source === 'modpack' && pv) {
      const l = pv.loaders.find((x) => LOADER_FLAVOR[x]);
      return l ? { flavor: LOADER_FLAVOR[l], mc: pv.mc[0] ?? '' } : null;
    }
    if (source === 'mix' && mix) return LOADER_FLAVOR[mix.loader] ? { flavor: LOADER_FLAVOR[mix.loader], mc: mix.mc } : null;
    if (source === 'file' && file) return { flavor: 'fabric', mc: '' };
    return null;
  }, [source, mc, flavor, pv, mix, file]);

  const defaultName = source === 'modpack' ? pack?.title : source === 'mix' ? mix?.name : source === 'file' ? file.split(/[\\/]/).pop()?.replace(/\.mrpack$/i, '') : undefined;

  const make = async () => {
    if (!target) return;
    const from: SrvFrom | undefined = source === 'modpack' && pack ? { type: 'modrinth', project: pack.project, version: packVersion } : source === 'mix' ? { type: 'mix', id: mixId } : source === 'file' ? { type: 'file', path: file } : undefined;
    const req: SrvNew = { name: (name.trim() || defaultName || 'My server').slice(0, 60), flavor: target.flavor, mc: target.mc, memoryMb: memory, eula, onlineMode, maxPlayers, whitelist, gamemode, difficulty, seed: seed.trim() || undefined, from, start };
    setSaving(true);
    try {
      const v = await api.servers.create(req);
      toast.success('Setting up the server', start ? 'It starts by itself when it’s ready.' : 'Press Start when it’s ready.');
      onMade(v.id);
      onClose();
    } catch (e) {
      toast.error('Not made', errorText(e));
    } finally {
      setSaving(false);
    }
  };

  const pickFile = async () => {
    const p = await api.app.pickFiles('Pick a Modrinth modpack (.mrpack)');
    if (p?.[0]) setFile(p[0]);
  };

  return (
    <Modal
      open={open}
      onClose={onClose}
      title="New Minecraft server"
      description="It runs on this PC. OmniHub downloads the server and the Java it needs, checks every download, and keeps it going."
      icon={Server}
      size="lg"
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="primary" icon={Server} disabled={!target || !eula || saving} loading={saving} onClick={() => void make()}>
            Make the server
          </Button>
        </>
      }
    >
      <div className="space-y-5">
        <Segmented
          value={source}
          onChange={setSource}
          label="Start from"
          options={[
            { value: 'fresh', label: 'A new world', icon: Globe },
            { value: 'modpack', label: 'A modpack', icon: Package },
            { value: 'mix', label: 'One of my mixes', icon: Blocks },
            { value: 'file', label: 'A .mrpack file', icon: FileArchive },
          ]}
        />

        {source === 'fresh' && (
          <div className="space-y-4">
            <div className="grid gap-2 sm:grid-cols-3">
              {FLAVORS.map((f) => (
                <button key={f.id} type="button" onClick={() => setFlavor(f.id)} className={cx('rounded-xl border px-3 py-2.5 text-left transition', flavor === f.id ? 'border-accent bg-accent-soft' : 'border-line hover:bg-surface-2')}>
                  <div className="text-[13px] font-semibold text-fg">{f.label}</div>
                  <div className="text-[11.5px] leading-snug text-dim">{f.hint}</div>
                </button>
              ))}
            </div>
            <div className="grid gap-3 sm:grid-cols-2">
              <Field label="Minecraft version">
                {versions ? (
                  <Select value={mc} onChange={(e) => setMc(e.target.value)}>
                    {versions.map((v) => (
                      <option key={v} value={v}>
                        {v}
                      </option>
                    ))}
                  </Select>
                ) : (
                  <div className="flex h-9 items-center gap-2 text-[12.5px] text-dim">
                    <Spinner size={14} /> Getting {flavorLabel(flavor)}’s versions…
                  </div>
                )}
              </Field>
              <Field label="World seed" hint="Leave empty for a random world.">
                <TextInput value={seed} onChange={(e) => setSeed(e.target.value)} placeholder="Random" />
              </Field>
              <Field label="Game mode">
                <Select value={gamemode} onChange={(e) => setGamemode(e.target.value as SrvGamemode)}>
                  <option value="survival">Survival</option>
                  <option value="creative">Creative</option>
                  <option value="adventure">Adventure</option>
                </Select>
              </Field>
              <Field label="Difficulty">
                <Select value={difficulty} onChange={(e) => setDifficulty(e.target.value as SrvDifficulty)}>
                  <option value="peaceful">Peaceful</option>
                  <option value="easy">Easy</option>
                  <option value="normal">Normal</option>
                  <option value="hard">Hard</option>
                </Select>
              </Field>
            </div>
          </div>
        )}

        {source === 'modpack' && (
          <div className="space-y-3">
            <SearchInput value={query} onChange={setQuery} placeholder="Search Modrinth’s modpacks" />
            <div className="grid max-h-64 gap-2 overflow-y-auto sm:grid-cols-2">
              {hits === null && <Spinner />}
              {hits?.map((h) => (
                <button key={h.project} type="button" onClick={() => setPack(h)} className={cx('flex items-center gap-2.5 rounded-xl border px-3 py-2 text-left', pack?.project === h.project ? 'border-accent bg-accent-soft' : 'border-line hover:bg-surface-2')}>
                  <div className="h-9 w-9 shrink-0 overflow-hidden rounded-lg bg-surface-2">{h.icon && <img src={h.icon} alt="" className="h-full w-full object-cover" />}</div>
                  <div className="min-w-0">
                    <div className="truncate text-[13px] font-medium text-fg">{h.title}</div>
                    <div className="truncate text-[11.5px] text-dim">{h.loaders.join(', ')} · {h.versions.at(-1)}</div>
                  </div>
                </button>
              ))}
            </div>
            {pack && (
              <Field label={`${pack.title} version`}>
                {packVersions ? (
                  <Select value={packVersion} onChange={(e) => setPackVersion(e.target.value)}>
                    {packVersions.map((v) => (
                      <option key={v.id} value={v.id}>
                        {v.number} · Minecraft {v.mc.join(', ')} · {v.loaders.join(', ')}
                      </option>
                    ))}
                  </Select>
                ) : (
                  <Spinner />
                )}
              </Field>
            )}
          </div>
        )}

        {source === 'mix' &&
          (mixes === null ? (
            <Spinner />
          ) : mixes.length === 0 ? (
            <Callout tone="info" icon={Blocks}>
              Mix modpacks on the Mods page (Minecraft), then host the mix here.
            </Callout>
          ) : (
            <Field label="Mix" hint={mix ? `Minecraft ${mix.mc} · ${mix.loader} ${mix.loaderVersion} · ${mix.entries.filter((e) => e.file).length} mods (the ones that only run in the game itself stay off the server)` : undefined}>
              <Select value={mixId} onChange={(e) => setMixId(e.target.value)}>
                {mixes.map((m) => (
                  <option key={m.id} value={m.id}>
                    {m.name}
                  </option>
                ))}
              </Select>
            </Field>
          ))}

        {source === 'file' && (
          <div className="flex items-center gap-2">
            <TextInput value={file} readOnly placeholder="No file picked" className="flex-1" />
            <Button variant="secondary" icon={FileArchive} onClick={() => void pickFile()}>
              Pick a .mrpack
            </Button>
          </div>
        )}

        {target && source !== 'fresh' && target.mc && (
          <div className="flex flex-wrap items-center gap-1.5 text-[12.5px] text-dim">
            Runs as <Badge tone="accent">{flavorLabel(target.flavor)}</Badge> <Badge tone="neutral">Minecraft {target.mc}</Badge>
          </div>
        )}

        <div className="grid gap-3 sm:grid-cols-2">
          <Field label="Name">
            <TextInput value={name} onChange={(e) => setName(e.target.value)} placeholder={defaultName || 'My server'} maxLength={60} />
          </Field>
          <Field label="Most players at once">
            <TextInput type="number" min={1} max={500} value={maxPlayers} onChange={(e) => setMaxPlayers(Math.max(1, Math.min(500, Number(e.target.value) || 1)))} />
          </Field>
          <Field label={`Memory: ${(memory / 1024).toFixed(memory % 1024 ? 1 : 0)} GB`} hint={memoryHint(memory, target?.flavor ?? flavor, modded)} className="sm:col-span-2">
            <input type="range" min={1024} max={16384} step={512} value={memory} onChange={(e) => setMemory(Number(e.target.value))} className="w-full accent-[var(--accent)]" aria-label="Memory" />
          </Field>
        </div>

        <div className="space-y-2.5 rounded-xl border border-line px-4 py-3">
          <label className="flex items-center justify-between gap-3 text-[13px] text-fg">
            <span>
              Only friends on the whitelist
              <span className="block text-[11.5px] text-faint">Add their Minecraft names in Players. Strongly recommended for a server open to the internet.</span>
            </span>
            <Switch checked={whitelist} onChange={setWhitelist} label="Whitelist" />
          </label>
          <label className="flex items-center justify-between gap-3 text-[13px] text-fg">
            <span>Start it when it’s ready</span>
            <Switch checked={start} onChange={setStart} label="Start when ready" />
          </label>
        </div>
        <div className="space-y-2">
          <div className="text-[12.5px] font-medium text-fg">Who can join</div>
          <JoinChoice everyone={!onlineMode} onChange={(e) => (setOnlineMode(!e), e && setWhitelist(true))} />
          {!onlineMode && <Callout tone="info">{EVERYONE_NOTE}</Callout>}
        </div>

        <div className="flex items-start justify-between gap-3 rounded-xl border border-line bg-surface-2/50 px-4 py-3">
          <Checkbox checked={eula} onChange={setEula} label={<span className="text-[13px] text-fg">I accept the Minecraft End User License Agreement (EULA)</span>} />
          <Button size="sm" variant="ghost" icon={ExternalLink} onClick={() => void api.app.openUrl('https://aka.ms/MinecraftEULA')}>
            Read it
          </Button>
        </div>
        <div className="flex items-center gap-1.5 text-[11.5px] text-faint">
          <Sparkles size={12} /> Claude can make data packs (and Paper plugins) for it later, in Content.
        </div>
      </div>
    </Modal>
  );
}
