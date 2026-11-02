// Pokémon save editor for the Game Boy Advance games (Ruby, Sapphire,
// Emerald, FireRed, LeafGreen): the trainer, the party and the 14 PC boxes.
// Every write keeps a copy of the old save next to it first.

import type { MonEdit, MonView, PkmSave, PkmSlot, PkmTables, PkmTrainer } from '@shared/types';
import { Download, FolderOpen, Plus, Save, Sparkles, Trash2, Upload, X } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Badge, Card } from '../../components/ui/Card';
import { Field, Select, Switch, TextInput } from '../../components/ui/Form';
import { Callout } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { confirm } from '../../state/dialogs';
import { toast } from '../../state/toasts';
import { gbaSpriteUrl } from '../counters/pokemon';

const STATS = ['HP', 'Atk', 'Def', 'Spe', 'SpA', 'SpD'];
const NATURE_STATS = ['Atk', 'Def', 'Spe', 'SpA', 'SpD'];

function natureLabel(name: string, n: number): string {
  const up = Math.floor(n / 5);
  const down = n % 5;
  return up === down ? `${name} (neutral)` : `${name} (+${NATURE_STATS[up]} −${NATURE_STATS[down]})`;
}

const sameSlot = (a: PkmSlot, b: PkmSlot) => a.box === b.box && a.index === b.index;

function Sprite({ mon, size = 48 }: { mon: MonView; size?: number }) {
  const [failed, setFailed] = useState(false);
  if (mon.egg || failed || !mon.dex) return <span className="text-[11px] text-faint">{mon.egg ? 'Egg' : '?'}</span>;
  return <img src={gbaSpriteUrl(mon.dex, mon.shiny)} alt="" width={size} height={size} loading="lazy" draggable={false} onError={() => setFailed(true)} className="[image-rendering:pixelated]" />;
}

function SlotTile({ mon, selected, onClick, label }: { mon: MonView | null; selected: boolean; onClick: () => void; label: string }) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-label={mon ? `${label}: ${mon.nickname || mon.speciesName}, level ${mon.level}${mon.shiny ? ', shiny' : ''}` : `${label}: empty`}
      aria-pressed={selected}
      className={cx('relative grid aspect-square place-items-center rounded-lg border transition-colors', selected ? 'border-accent bg-accent-soft' : 'border-line bg-surface hover:border-line-strong', !mon?.valid && mon && 'border-bad')}
    >
      {mon ? <Sprite mon={mon} /> : <Plus size={14} className="text-faint" aria-hidden />}
      {mon && <span className="absolute bottom-0.5 right-1 text-[10px] font-medium tabular-nums text-dim">{mon.level}</span>}
      {mon?.shiny && <Sparkles size={11} className="absolute left-1 top-1 text-warn" aria-hidden />}
    </button>
  );
}

function TrainerCard({ save, onSave }: { save: PkmSave; onSave: (s: PkmSave) => void }) {
  const [t, setT] = useState<PkmTrainer>(save.trainer);
  useEffect(() => setT(save.trainer), [save.trainer]);
  const dirty = JSON.stringify(t) !== JSON.stringify(save.trainer);
  const num = (v: string, max: number) => Math.max(0, Math.min(max, Math.floor(Number(v) || 0)));
  return (
    <Card className="space-y-3 p-4">
      <div className="text-[13.5px] font-semibold text-fg">Trainer</div>
      <div className="grid grid-cols-2 gap-3">
        <Field label="Name" htmlFor="tr-name">
          <TextInput id="tr-name" maxLength={7} value={t.name} onChange={(e) => setT({ ...t, name: e.target.value })} />
        </Field>
        <Field label="Gender" htmlFor="tr-gender">
          <Select id="tr-gender" value={t.gender} onChange={(e) => setT({ ...t, gender: Number(e.target.value) })}>
            <option value={0}>Boy</option>
            <option value={1}>Girl</option>
          </Select>
        </Field>
        <Field label="Trainer ID" htmlFor="tr-tid">
          <TextInput id="tr-tid" type="number" min={0} max={65535} value={t.tid} onChange={(e) => setT({ ...t, tid: num(e.target.value, 65535) })} />
        </Field>
        <Field label="Secret ID" htmlFor="tr-sid">
          <TextInput id="tr-sid" type="number" min={0} max={65535} value={t.sid} onChange={(e) => setT({ ...t, sid: num(e.target.value, 65535) })} />
        </Field>
        <Field label="Money" htmlFor="tr-money">
          <TextInput id="tr-money" type="number" min={0} max={999999} value={t.money} onChange={(e) => setT({ ...t, money: num(e.target.value, 999999) })} />
        </Field>
        <Field label="Play time (h : min)">
          <div className="flex gap-2">
            <TextInput aria-label="Hours" type="number" min={0} max={999} value={t.hours} onChange={(e) => setT({ ...t, hours: num(e.target.value, 999) })} />
            <TextInput aria-label="Minutes" type="number" min={0} max={59} value={t.minutes} onChange={(e) => setT({ ...t, minutes: num(e.target.value, 59) })} />
          </div>
        </Field>
      </div>
      <p className="text-[11.5px] text-faint">Changing the IDs makes the Pokémon you caught count as traded (and changes which ones are shiny).</p>
      <Button size="sm" disabled={!dirty} onClick={() => void api.pkm.trainer(t).then(onSave, (e) => toast.error('Couldn’t change the trainer', errorText(e)))}>
        Apply
      </Button>
    </Card>
  );
}

function MonEditor({ tables, save, slot, mon, onSave }: { tables: PkmTables; save: PkmSave; slot: PkmSlot; mon: MonView | null; onSave: (s: PkmSave) => void }) {
  const blank: MonView = useMemo(
    () => ({ species: 0, dex: 0, speciesName: '', nickname: '', level: 5, exp: 0, nature: 0, shiny: false, gender: 'none', abilitySlot: 0, ability: '', item: 0, friendship: 70, moves: [33, 0, 0, 0], pp: [0, 0, 0, 0], ppUps: [0, 0, 0, 0], ivs: [15, 15, 15, 15, 15, 15], evs: [0, 0, 0, 0, 0, 0], stats: [0, 0, 0, 0, 0, 0], otName: save.trainer.name, tid: save.trainer.tid, sid: save.trainer.sid, ball: 4, metLevel: 5, egg: false, pid: 0, valid: true }),
    [save.trainer],
  );
  const [f, setF] = useState<MonView>(mon ?? blank);
  const [busy, setBusy] = useState(false);
  useEffect(() => setF(mon ?? blank), [mon, blank]);
  const where = slot.box === null ? `Party ${slot.index + 1}` : `${save.boxes[slot.box]?.name ?? `Box ${slot.box + 1}`}, place ${slot.index + 1}`;
  const evTotal = f.evs.reduce((a, b) => a + b, 0);
  const abilities = tables.abilities[f.species] ?? ['', ''];
  const set = <K extends keyof MonView>(k: K, v: MonView[K]) => setF((x) => ({ ...x, [k]: v }));
  const setArr = (k: 'ivs' | 'evs' | 'moves' | 'ppUps', i: number, v: number) => setF((x) => ({ ...x, [k]: x[k].map((o, j) => (j === i ? v : o)) }));
  const n = (v: string, max: number) => Math.max(0, Math.min(max, Math.floor(Number(v) || 0)));

  const apply = async () => {
    if (!f.species) return toast.error('Pick a Pokémon first');
    if (evTotal > 510) return toast.error('EVs add up to more than 510');
    setBusy(true);
    const edit: MonEdit = { species: f.species, nickname: f.nickname, level: f.level, nature: f.nature, shiny: f.shiny, abilitySlot: f.abilitySlot, item: f.item, friendship: f.friendship, moves: f.moves, ppUps: f.ppUps, ivs: f.ivs, evs: f.evs, otName: f.otName, tid: f.tid, sid: f.sid, ball: f.ball };
    try {
      onSave(await api.pkm.edit(slot, edit));
      toast.success(mon ? 'Pokémon changed' : 'Pokémon added', 'Not written to the file yet: press Save changes.');
    } catch (e) {
      toast.error('Couldn’t change it', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  const exportPk3 = async () => {
    const dest = await api.app.saveFile('Save the Pokémon as a .pk3 file', `${(mon?.nickname || mon?.speciesName || 'pokemon').toLowerCase()}.pk3`);
    if (dest) await api.pkm.exportPk3(slot, dest).then(() => toast.success('Saved', dest), (e) => toast.error('Couldn’t save it', errorText(e)));
  };
  const importPk3 = async () => {
    const [path] = await api.app.pickFiles('Pick a .pk3 file');
    if (path) await api.pkm.importPk3(slot, path).then(onSave, (e) => toast.error('Couldn’t bring it in', errorText(e)));
  };
  const remove = async () => {
    if (!(await confirm({ title: `Delete ${mon?.nickname || mon?.speciesName}?`, description: 'It’s gone from the save once you press Save changes (a copy of the old save is kept).', confirmLabel: 'Delete', tone: 'danger' }))) return;
    await api.pkm.remove(slot).then(onSave, (e) => toast.error('Couldn’t delete it', errorText(e)));
  };

  return (
    <Card className="space-y-4 p-4">
      <div className="flex items-center gap-3">
        <div className="grid h-16 w-16 place-items-center rounded-xl bg-surface-2">{mon ? <Sprite mon={{ ...mon, shiny: f.shiny, dex: tables.species.find((s) => s.id === f.species)?.dex ?? mon.dex }} size={64} /> : <Plus className="text-faint" />}</div>
        <div className="min-w-0 flex-1">
          <div className="text-[12px] text-faint">{where}</div>
          <div className="truncate font-display text-[17px] font-semibold text-fg">{mon ? `${mon.nickname || mon.speciesName}` : 'Empty: add a Pokémon'}</div>
          {mon && (
            <div className="mt-1 flex flex-wrap gap-1.5">
              <Badge>Lv {mon.level}</Badge>
              <Badge>{mon.gender === 'none' ? 'No gender' : mon.gender === 'male' ? '♂' : '♀'}</Badge>
              {mon.ability && <Badge>{mon.ability}</Badge>}
              {mon.shiny && <Badge tone="warn" icon={Sparkles}>Shiny</Badge>}
              {!mon.valid && <Badge tone="bad">Bad egg</Badge>}
            </div>
          )}
        </div>
      </div>
      {mon && !mon.valid ? (
        <Callout tone="bad">This one’s data is damaged (a “bad egg”). You can delete it.</Callout>
      ) : (
        <>
          <div className="grid grid-cols-2 gap-3 xl:grid-cols-3">
            <Field label="Pokémon" htmlFor="pk-species">
              <Select id="pk-species" value={f.species} onChange={(e) => set('species', Number(e.target.value))}>
                {!f.species && <option value={0}>Pick one…</option>}
                {tables.species.map((s) => (
                  <option key={s.id} value={s.id}>
                    #{String(s.dex).padStart(3, '0')} {s.name}
                  </option>
                ))}
              </Select>
            </Field>
            <Field label="Nickname" htmlFor="pk-nick">
              <TextInput id="pk-nick" maxLength={10} value={f.nickname} onChange={(e) => set('nickname', e.target.value)} placeholder="Its species name" />
            </Field>
            <Field label="Level" htmlFor="pk-level">
              <TextInput id="pk-level" type="number" min={1} max={100} value={f.level} onChange={(e) => set('level', Math.max(1, n(e.target.value, 100)))} />
            </Field>
            <Field label="Nature" htmlFor="pk-nature">
              <Select id="pk-nature" value={f.nature} onChange={(e) => set('nature', Number(e.target.value))}>
                {tables.natures.map((name, i) => (
                  <option key={name} value={i}>
                    {natureLabel(name, i)}
                  </option>
                ))}
              </Select>
            </Field>
            <Field label="Ability" htmlFor="pk-ability">
              <Select id="pk-ability" value={abilities[1] ? f.abilitySlot : 0} onChange={(e) => set('abilitySlot', Number(e.target.value))}>
                <option value={0}>{abilities[0] || '—'}</option>
                {abilities[1] && <option value={1}>{abilities[1]}</option>}
              </Select>
            </Field>
            <Field label="Held item" htmlFor="pk-item">
              <Select id="pk-item" value={f.item} onChange={(e) => set('item', Number(e.target.value))}>
                {tables.items.map((it) => (
                  <option key={it.id} value={it.id}>
                    {it.name}
                  </option>
                ))}
              </Select>
            </Field>
            <Field label="Ball" htmlFor="pk-ball">
              <Select id="pk-ball" value={f.ball || 4} onChange={(e) => set('ball', Number(e.target.value))}>
                {tables.balls.map((b, i) => i > 0 && <option key={b} value={i}>{b}</option>)}
              </Select>
            </Field>
            <Field label="Friendship (0–255)" htmlFor="pk-friend">
              <TextInput id="pk-friend" type="number" min={0} max={255} value={f.friendship} onChange={(e) => set('friendship', n(e.target.value, 255))} />
            </Field>
            <Field label="Shiny">
              <div className="flex h-9 items-center">
                <Switch checked={f.shiny} onChange={(v) => set('shiny', v)} label="Shiny" />
              </div>
            </Field>
          </div>

          <div>
            <div className="mb-1.5 text-[12.5px] font-medium text-dim">Moves (and PP Ups)</div>
            <div className="grid grid-cols-2 gap-2">
              {f.moves.map((m, i) => (
                <div key={i} className="flex gap-1.5">
                  <Select aria-label={`Move ${i + 1}`} value={m} onChange={(e) => setArr('moves', i, Number(e.target.value))}>
                    {tables.moves.map((mv) => (
                      <option key={mv.id} value={mv.id}>
                        {mv.name}
                      </option>
                    ))}
                  </Select>
                  <Select aria-label={`PP Ups on move ${i + 1}`} value={f.ppUps[i]} onChange={(e) => setArr('ppUps', i, Number(e.target.value))} className="w-16">
                    {[0, 1, 2, 3].map((u) => (
                      <option key={u} value={u}>
                        +{u}
                      </option>
                    ))}
                  </Select>
                </div>
              ))}
            </div>
          </div>

          <div className="grid grid-cols-[auto_repeat(6,minmax(0,1fr))] items-center gap-x-2 gap-y-1.5 text-[12px]">
            <span />
            {STATS.map((s) => (
              <span key={s} className="text-center font-medium text-dim">
                {s}
              </span>
            ))}
            <span className="text-dim">IVs</span>
            {f.ivs.map((v, i) => (
              <TextInput key={i} aria-label={`${STATS[i]} IV`} type="number" min={0} max={31} value={v} onChange={(e) => setArr('ivs', i, n(e.target.value, 31))} className="px-1 text-center" />
            ))}
            <span className="text-dim">EVs</span>
            {f.evs.map((v, i) => (
              <TextInput key={i} aria-label={`${STATS[i]} EV`} type="number" min={0} max={255} value={v} onChange={(e) => setArr('evs', i, n(e.target.value, 255))} className="px-1 text-center" />
            ))}
            {mon && (
              <>
                <span className="text-dim">Stats</span>
                {mon.stats.map((v, i) => (
                  <span key={i} className="text-center font-semibold tabular-nums text-fg">
                    {v}
                  </span>
                ))}
              </>
            )}
          </div>
          <div className="flex flex-wrap items-center gap-2 text-[12px]">
            <Button size="sm" onClick={() => set('ivs', [31, 31, 31, 31, 31, 31])}>
              Perfect IVs
            </Button>
            <span className={cx('ml-auto', evTotal > 510 ? 'text-bad' : 'text-faint')}>EVs: {evTotal}/510</span>
          </div>

          <div className="grid grid-cols-3 gap-3">
            <Field label="Original trainer" htmlFor="pk-ot">
              <TextInput id="pk-ot" maxLength={7} value={f.otName} onChange={(e) => set('otName', e.target.value)} />
            </Field>
            <Field label="OT ID" htmlFor="pk-tid">
              <TextInput id="pk-tid" type="number" min={0} max={65535} value={f.tid} onChange={(e) => set('tid', n(e.target.value, 65535))} />
            </Field>
            <Field label="OT secret ID" htmlFor="pk-sid">
              <TextInput id="pk-sid" type="number" min={0} max={65535} value={f.sid} onChange={(e) => set('sid', n(e.target.value, 65535))} />
            </Field>
          </div>
        </>
      )}
      <div className="flex flex-wrap gap-2 border-t border-line pt-3">
        {(!mon || mon.valid) && (
          <Button variant="primary" loading={busy} disabled={!f.species} onClick={() => void apply()}>
            {mon ? 'Apply' : 'Add'}
          </Button>
        )}
        <Button icon={Upload} onClick={() => void importPk3()}>
          Bring in a .pk3
        </Button>
        {mon && (
          <>
            <Button icon={Download} onClick={() => void exportPk3()}>
              Save as .pk3
            </Button>
            <Button variant="danger" icon={Trash2} onClick={() => void remove()}>
              Delete
            </Button>
          </>
        )}
      </div>
      {f.species === 201 && <p className="text-[11.5px] text-faint">Unown’s letter comes from the same number as its nature and shininess, so changing those can change its letter.</p>}
    </Card>
  );
}

export function PokemonSaveTab() {
  const [tables, setTables] = useState<PkmTables | null>(null);
  const [save, setSave] = useState<PkmSave | null>(null);
  const [box, setBox] = useState(0);
  const [slot, setSlot] = useState<PkmSlot>({ box: null, index: 0 });
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    void api.pkm.tables().then(setTables);
    void api.pkm.view().then(setSave);
  }, []);

  const open = async () => {
    const [path] = await api.app.pickFiles('Open a Pokémon save (.sav)');
    if (!path) return;
    try {
      setSave(await api.pkm.open(path));
      setSlot({ box: null, index: 0 });
    } catch (e) {
      toast.error('Couldn’t open that save', errorText(e));
    }
  };
  const write = async () => {
    setBusy(true);
    try {
      const backup = await api.pkm.write();
      setSave(await api.pkm.view());
      toast.success('Save written', `The old one is kept as ${backup.split(/[\\/]/).pop()}`);
    } catch (e) {
      toast.error('Couldn’t write the save', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  const close = async () => {
    if (save?.changed && !(await confirm({ title: 'Close without saving?', description: 'Your changes aren’t in the file yet.', confirmLabel: 'Close', tone: 'danger' }))) return;
    await api.pkm.close();
    setSave(null);
  };

  if (!save || !tables) {
    return (
      <Card className="mx-auto max-w-[640px] space-y-4 p-6 text-center">
        <div className="mx-auto grid h-14 w-14 place-items-center rounded-2xl bg-accent-soft text-accent">
          <Sparkles size={26} aria-hidden />
        </div>
        <h2 className="font-display text-[19px] font-semibold text-fg">Pokémon save editor</h2>
        <p className="text-[13.5px] leading-relaxed text-dim">For the Game Boy Advance games: Ruby, Sapphire, Emerald, FireRed and LeafGreen (the .sav from your emulator). Change the trainer, and any Pokémon in the party or the 14 PC boxes: species, level, nature, shiny, IVs, EVs, moves, item and more. Bring in and save single Pokémon as .pk3 files.</p>
        <Callout tone="info" className="text-left">
          Close the game in your emulator first (it would write over your changes), and keep a copy of anything you care about. OmniHub keeps a copy of the old save next to it every time it writes.
        </Callout>
        <Button variant="primary" size="lg" icon={FolderOpen} onClick={() => void open()}>
          Open a save…
        </Button>
      </Card>
    );
  }

  const current = slot.box === null ? (save.party[slot.index] ?? null) : (save.boxes[slot.box]?.mons[slot.index] ?? null);
  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center gap-2">
        <Badge tone="accent">{save.game}</Badge>
        <span className="min-w-0 flex-1 truncate text-[12.5px] text-dim" title={save.path}>
          {save.path}
        </span>
        {!save.checksumsOk && <Badge tone="warn">Checksums were off; saving fixes them</Badge>}
        <Button variant="primary" icon={Save} loading={busy} disabled={!save.changed} onClick={() => void write()}>
          Save changes
        </Button>
        <Button icon={FolderOpen} onClick={() => void open()}>
          Open another
        </Button>
        <Button variant="ghost" icon={X} onClick={() => void close()}>
          Close
        </Button>
      </div>
      <div className="grid grid-cols-1 gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(0,1.25fr)]">
        <div className="space-y-4">
          <TrainerCard save={save} onSave={setSave} />
          <Card className="space-y-3 p-4">
            <div className="text-[13.5px] font-semibold text-fg">Party</div>
            <div className="grid grid-cols-6 gap-2">
              {Array.from({ length: 6 }, (_, i) => {
                const s = { box: null, index: Math.min(i, save.party.length) };
                return i <= save.party.length ? <SlotTile key={i} label={`Party ${i + 1}`} mon={save.party[i] ?? null} selected={sameSlot(slot, { box: null, index: i })} onClick={() => setSlot(s)} /> : <div key={i} className="aspect-square rounded-lg border border-dashed border-line" />;
              })}
            </div>
            <div className="flex items-center justify-between gap-2">
              <div className="text-[13.5px] font-semibold text-fg">PC</div>
              <Select aria-label="Box" value={box} onChange={(e) => setBox(Number(e.target.value))} className="w-44">
                {save.boxes.map((b, i) => (
                  <option key={i} value={i}>
                    {b.name} ({b.mons.filter(Boolean).length}/30)
                  </option>
                ))}
              </Select>
            </div>
            <div className="grid grid-cols-6 gap-2">
              {save.boxes[box].mons.map((m, i) => (
                <SlotTile key={i} label={`${save.boxes[box].name} ${i + 1}`} mon={m} selected={sameSlot(slot, { box, index: i })} onClick={() => setSlot({ box, index: i })} />
              ))}
            </div>
          </Card>
        </div>
        <MonEditor tables={tables} save={save} slot={slot} mon={current} onSave={setSave} />
      </div>
    </div>
  );
}
