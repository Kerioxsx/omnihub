// Counters: count anything with a click or a key that works from anywhere
// (the game in front still gets the key), with a picture, a timer, a
// pop-out window and shiny hunts with their odds.

import type { Counter, CounterInput, CountersState, Found, Hunt } from '@shared/types';
import { Clock, ExternalLink, Hash, Image as ImageIcon, Minus, Pause, Pencil, Play, Plus, RotateCcw, Sparkles, Trash2, Trophy } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { api, errorText } from '../../api';
import { Page } from '../../components/Page';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Card } from '../../components/ui/Card';
import { Checkbox, Field, Select, Switch, TextInput } from '../../components/ui/Form';
import { Modal } from '../../components/ui/Overlay';
import { Callout, EmptyState } from '../../components/ui/States';
import { useEvent, useNow } from '../../lib/hooks';
import { confirm } from '../../state/dialogs';
import { toast } from '../../state/toasts';
import { POKEMON, generationOf, spriteUrl } from './pokemon';
import { CounterPicture, KeyPicker, elapsedMs, formatTimer, foundByNow, oddsLabel, shinyChance } from './shared';

const GENERATIONS: [number, string][] = [
  [2, 'Gold, Silver, Crystal'],
  [3, 'Ruby, Sapphire, Emerald, FireRed, LeafGreen'],
  [4, 'Diamond, Pearl, Platinum, HeartGold, SoulSilver'],
  [5, 'Black, White, Black 2, White 2'],
  [6, 'X, Y, Omega Ruby, Alpha Sapphire'],
  [7, 'Sun, Moon, Ultra Sun, Ultra Moon, Let’s Go'],
  [8, 'Sword, Shield, Brilliant Diamond, Shining Pearl, Legends: Arceus'],
  [9, 'Scarlet, Violet'],
];

/** Methods; `odds` for ones with odds of their own (charm: with the Shiny Charm). */
const METHODS: { name: string; odds?: number; charmOdds?: number; masuda?: boolean }[] = [
  { name: 'Random encounters' },
  { name: 'Soft resets' },
  { name: 'Masuda method', masuda: true },
  { name: 'Dynamax Adventures', odds: 300, charmOdds: 100 },
  { name: 'Poké Radar' },
  { name: 'Chain fishing' },
  { name: 'SOS chain' },
  { name: 'DexNav' },
  { name: 'Mass outbreak' },
  { name: 'Sandwich (Sparkling Power)' },
  { name: 'Other' },
];
const SIMPLE = new Set(['Random encounters', 'Soft resets', 'Masuda method']);

function HuntFields({ hunt, onChange }: { hunt: Hunt; onChange: (h: Hunt) => void }) {
  const [query, setQuery] = useState(hunt.name);
  const matches = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q || POKEMON[hunt.species - 1]?.toLowerCase() === q) return [];
    const n = Number(q);
    return POKEMON.map((name, i) => ({ name, dex: i + 1 }))
      .filter((p) => (n ? p.dex === n : p.name.toLowerCase().includes(q)))
      .slice(0, 8);
  }, [query, hunt.species]);
  const method = METHODS.find((m) => m.name === hunt.method) ?? METHODS[METHODS.length - 1];
  const setMethod = (name: string) => {
    const m = METHODS.find((x) => x.name === name) ?? METHODS[METHODS.length - 1];
    const odds = m.odds ? (hunt.charm && m.charmOdds ? m.charmOdds : m.odds) : SIMPLE.has(m.name) ? null : (hunt.customOdds ?? null);
    // Eggs are the Masuda method; resets and wild encounters never are.
    const masuda = m.masuda ? true : m.name === 'Random encounters' || m.name === 'Soft resets' ? false : hunt.masuda;
    onChange({ ...hunt, method: m.name, masuda, customOdds: odds });
  };
  const p = shinyChance(hunt);
  return (
    <div className="space-y-3 rounded-xl border border-line bg-surface p-3">
      <div className="flex gap-3">
        <div className="grid h-20 w-20 shrink-0 place-items-center rounded-xl bg-surface-2">{hunt.species > 0 && <img src={spriteUrl(hunt.species, true)} alt="" className="h-20 w-20 [image-rendering:pixelated]" />}</div>
        <div className="relative min-w-0 flex-1">
          <Field label="Pokémon (name or Pokédex number)" htmlFor="hunt-mon">
            <TextInput id="hunt-mon" value={query} onChange={(e) => setQuery(e.target.value)} placeholder="Charizard, 6…" autoComplete="off" />
          </Field>
          {matches.length > 0 && (
            <ul className="absolute z-10 mt-1 max-h-64 w-full overflow-auto rounded-xl border border-line-strong bg-elev p-1 shadow-xl" role="listbox" aria-label="Pokémon">
              {matches.map((m) => (
                <li key={m.dex}>
                  <button
                    type="button"
                    role="option"
                    aria-selected={m.dex === hunt.species}
                    onClick={() => {
                      setQuery(m.name);
                      onChange({ ...hunt, species: m.dex, name: m.name, generation: Math.max(hunt.generation, Math.min(9, Math.max(2, generationOf(m.dex)))) });
                    }}
                    className="flex w-full items-center gap-2 rounded-lg px-2 py-1 text-left text-[13px] hover:bg-surface-2"
                  >
                    <img src={spriteUrl(m.dex, true)} alt="" className="h-8 w-8 [image-rendering:pixelated]" loading="lazy" />
                    <span className="flex-1">{m.name}</span>
                    <span className="text-[11.5px] text-faint">#{m.dex}</span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
      <div className="grid grid-cols-2 gap-3">
        <Field label="Game" htmlFor="hunt-gen">
          <Select id="hunt-gen" value={hunt.generation} onChange={(e) => onChange({ ...hunt, generation: Number(e.target.value) })}>
            {GENERATIONS.map(([g, games]) => (
              <option key={g} value={g}>
                Gen {g}: {games}
              </option>
            ))}
          </Select>
        </Field>
        <Field label="Method" htmlFor="hunt-method">
          <Select id="hunt-method" value={method.name} onChange={(e) => setMethod(e.target.value)}>
            {METHODS.map((m) => (
              <option key={m.name}>{m.name}</option>
            ))}
          </Select>
        </Field>
      </div>
      <div className="flex flex-wrap items-center gap-4">
        <Checkbox checked={hunt.charm} disabled={hunt.generation < 5} onChange={(v) => onChange({ ...hunt, charm: v, customOdds: method.odds ? (v && method.charmOdds ? method.charmOdds : method.odds) : hunt.customOdds })} label="Shiny Charm" />
        <Checkbox checked={hunt.masuda} disabled={hunt.generation < 4 || !!hunt.customOdds} onChange={(v) => onChange({ ...hunt, masuda: v })} label="Masuda (foreign parent)" />
      </div>
      {!SIMPLE.has(method.name) && (
        <Field label="This method’s odds: 1 in…" htmlFor="hunt-odds" hint="They depend on the game and your chain or bonus; fill in the odds you’re going for, or leave empty for full odds.">
          <TextInput id="hunt-odds" type="number" min={1} value={hunt.customOdds ?? ''} onChange={(e) => onChange({ ...hunt, customOdds: e.target.value ? Math.max(1, Math.floor(Number(e.target.value))) : null })} placeholder={String(Math.round(1 / shinyChance({ ...hunt, customOdds: null })))} />
        </Field>
      )}
      <p className="text-[12.5px] text-dim">
        Odds: <b className="text-fg">{oddsLabel(p)}</b> per encounter.
      </p>
    </div>
  );
}

const blankHunt = (): Hunt => ({ species: 0, name: '', generation: 9, masuda: false, charm: false, customOdds: null, method: 'Random encounters' });

function CounterEditor({ open, onClose, counter, huntMode }: { open: boolean; onClose: () => void; counter: Counter | null; huntMode: boolean }) {
  const [form, setForm] = useState<CounterInput>({ name: '', step: 1, key: null, minusKey: null, hunt: null });
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    if (!open) return;
    setForm(counter ? { id: counter.id, name: counter.name, step: counter.step, key: counter.key, minusKey: counter.minusKey, hunt: counter.hunt } : { name: '', step: 1, key: null, minusKey: null, hunt: huntMode ? blankHunt() : null });
  }, [open, counter, huntMode]);
  const hunt = form.hunt;
  const save = async () => {
    setBusy(true);
    try {
      const name = form.name.trim() || (hunt?.name ? `Shiny ${hunt.name}` : '');
      await api.counters.save({ ...form, name });
      onClose();
    } catch (e) {
      toast.error('Couldn’t save the counter', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  const ready = (form.name.trim() || hunt?.name) && (!hunt || hunt.species > 0);
  return (
    <Modal
      open={open}
      onClose={onClose}
      icon={hunt ? Sparkles : Plus}
      title={counter ? 'Edit counter' : hunt ? 'New shiny hunt' : 'New counter'}
      size="lg"
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="primary" loading={busy} disabled={!ready} onClick={() => void save()}>
            {counter ? 'Save' : 'Create'}
          </Button>
        </>
      }
    >
      <div className="space-y-4">
        <div className="grid grid-cols-[1fr_120px] gap-3">
          <Field label="Name" htmlFor="ctr-name">
            <TextInput id="ctr-name" value={form.name} onChange={(e) => setForm({ ...form, name: e.target.value })} placeholder={hunt?.name ? `Shiny ${hunt.name}` : 'Deaths, push-ups, resets…'} autoFocus />
          </Field>
          <Field label="Step" htmlFor="ctr-step">
            <TextInput id="ctr-step" type="number" min={1} value={form.step} onChange={(e) => setForm({ ...form, step: Math.max(1, Math.floor(Number(e.target.value) || 1)) })} />
          </Field>
        </div>
        <div className="grid grid-cols-2 gap-3">
          <Field label="Key that adds">
            <KeyPicker label="Key that adds" value={form.key} onChange={(k) => setForm((f) => ({ ...f, key: k }))} />
          </Field>
          <Field label="Key that takes away">
            <KeyPicker label="Key that takes away" value={form.minusKey} onChange={(k) => setForm((f) => ({ ...f, minusKey: k }))} />
          </Field>
        </div>
        <p className="text-[12px] text-faint">Keys work from anywhere, even with a game or emulator in front, and the game still gets the key. Holding it down counts once.</p>
        <Checkbox checked={!!hunt} onChange={(v) => setForm({ ...form, hunt: v ? (hunt ?? blankHunt()) : null })} label="It’s a shiny hunt" />
        {hunt && <HuntFields hunt={hunt} onChange={(h) => setForm((f) => ({ ...f, hunt: h }))} />}
      </div>
    </Modal>
  );
}

function CounterCard({ counter, onEdit }: { counter: Counter; onEdit: () => void }) {
  const now = useNow(counter.timerSince ? 500 : 60_000);
  const [editing, setEditing] = useState(false);
  const [value, setValue] = useState(String(counter.count));
  const run = (p: Promise<unknown>) => void p.catch((e) => toast.error('Something went wrong', errorText(e)));
  const hunt = counter.hunt;
  const p = hunt ? shinyChance(hunt) : 0;
  const ms = elapsedMs(counter, now);
  const chooseImage = async () => {
    const [path] = await api.app.pickFiles('Pick a picture for the counter');
    if (path) run(api.counters.setImage(counter.id, path));
  };
  const found = async () => {
    const ok = await confirm({ title: hunt ? `Found shiny ${hunt.name}?` : 'Mark as done?', description: `${counter.count.toLocaleString()} ${hunt ? 'encounters' : 'counted'} and ${formatTimer(ms)} go into your history, and the counter starts again from 0.`, confirmLabel: hunt ? 'Found it!' : 'Done' });
    if (!ok) return;
    try {
      await api.counters.found(counter.id, true);
      toast.success(hunt ? `Shiny ${hunt.name}! 🎉` : 'Saved to history', `${counter.count.toLocaleString()} ${hunt ? 'encounters' : ''}`);
    } catch (e) {
      toast.error('Couldn’t save it', errorText(e));
    }
  };
  return (
    <Card className="flex flex-col gap-3 p-4">
      <div className="flex items-start gap-3">
        <div className="grid h-16 w-16 shrink-0 place-items-center overflow-hidden rounded-xl bg-surface-2">
          {counter.image || counter.hunt?.species ? <CounterPicture counter={counter} className="h-16 w-16" /> : <Hash size={26} className="text-faint" aria-hidden />}
        </div>
        <div className="min-w-0 flex-1">
          <div className="truncate font-display text-[16px] font-semibold text-fg">{counter.name}</div>
          <div className="mt-1 flex flex-wrap gap-1.5">
            {counter.key && <Badge tone="accent">+ {counter.key.label}</Badge>}
            {counter.minusKey && <Badge>− {counter.minusKey.label}</Badge>}
            {counter.step > 1 && <Badge>Step {counter.step}</Badge>}
            {hunt && <Badge tone="warn">{oddsLabel(p)}</Badge>}
          </div>
        </div>
        <div className="flex shrink-0">
          <IconButton icon={ExternalLink} label="Pop out" onClick={() => run(api.counters.popout(counter.id))} />
          <IconButton icon={ImageIcon} label={counter.image ? 'Change the picture' : 'Add a picture'} onClick={() => void chooseImage()} />
          <IconButton icon={Pencil} label="Edit" onClick={onEdit} />
          <IconButton
            icon={Trash2}
            label="Delete"
            onClick={() =>
              void confirm({ title: `Delete “${counter.name}”?`, description: 'The count and timer go. Finds already in your history stay.', confirmLabel: 'Delete', tone: 'danger' }).then((ok) => ok && run(api.counters.remove(counter.id)))
            }
          />
        </div>
      </div>
      <div className="flex items-center gap-3">
        <Button size="lg" icon={Minus} aria-label="Take one away" onClick={() => run(api.counters.bump(counter.id, -1))} />
        {editing ? (
          <form
            className="flex-1"
            onSubmit={(e) => {
              e.preventDefault();
              setEditing(false);
              const n = Number(value);
              if (Number.isFinite(n)) run(api.counters.set(counter.id, n));
            }}
          >
            <TextInput autoFocus type="number" min={0} value={value} onChange={(e) => setValue(e.target.value)} onBlur={() => setEditing(false)} aria-label="Count" className="text-center text-[28px]" />
          </form>
        ) : (
          <button type="button" onClick={() => (setValue(String(counter.count)), setEditing(true))} title="Click to type a number" className="flex-1 rounded-xl py-1 text-center font-display text-[40px] font-bold tabular-nums leading-none text-fg hover:bg-surface-2">
            {counter.count.toLocaleString()}
          </button>
        )}
        <Button size="lg" variant="primary" icon={Plus} aria-label="Add one" onClick={() => run(api.counters.bump(counter.id, 1))} />
      </div>
      {hunt && (
        <div className="grid grid-cols-3 gap-2 rounded-xl bg-surface px-3 py-2 text-[12px]">
          <div>
            <div className="text-faint">Chance by now</div>
            <div className="font-semibold tabular-nums text-fg">{(foundByNow(p, counter.count) * 100).toFixed(1)}%</div>
          </div>
          <div>
            <div className="text-faint">Past odds</div>
            <div className="font-semibold tabular-nums text-fg">{(counter.count * p).toFixed(2)}×</div>
          </div>
          <div>
            <div className="text-faint">Per hour</div>
            <div className="font-semibold tabular-nums text-fg">{ms > 60_000 ? Math.round(counter.count / (ms / 3_600_000)).toLocaleString() : '—'}</div>
          </div>
        </div>
      )}
      <div className="flex items-center gap-2">
        <Clock size={14} className="text-faint" aria-hidden />
        <span className="font-mono text-[14px] tabular-nums text-fg">{formatTimer(ms)}</span>
        <IconButton size="sm" icon={counter.timerSince ? Pause : Play} label={counter.timerSince ? 'Pause the timer' : 'Start the timer'} onClick={() => run(api.counters.timer(counter.id, !counter.timerSince))} />
        <IconButton size="sm" icon={RotateCcw} label="Reset the timer" onClick={() => run(api.counters.timer(counter.id, false, true))} />
        <Button size="sm" variant={hunt ? 'primary' : 'secondary'} icon={hunt ? Sparkles : Trophy} className="ml-auto" onClick={() => void found()}>
          {hunt ? 'Found it!' : 'Done'}
        </Button>
      </div>
    </Card>
  );
}

function History({ found }: { found: Found[] }) {
  if (found.length === 0) return null;
  return (
    <section className="mt-8">
      <h2 className="mb-3 font-display text-[15px] font-semibold text-fg">History</h2>
      <Card className="divide-y divide-line">
        {found.map((f) => (
          <div key={f.id} className="flex items-center gap-3 px-4 py-2.5">
            <div className="grid h-10 w-10 shrink-0 place-items-center">{f.species ? <img src={spriteUrl(f.species, true)} alt="" className="h-10 w-10 [image-rendering:pixelated]" loading="lazy" /> : <Trophy size={18} className="text-accent" />}</div>
            <div className="min-w-0 flex-1">
              <div className="truncate text-[13.5px] font-medium text-fg">{f.species ? `Shiny ${f.name}` : f.name}</div>
              <div className="text-[12px] text-faint">
                {f.count.toLocaleString()} · {formatTimer(f.timerMs)}
                {f.odds ? ` · ${f.odds}` : ''}
              </div>
            </div>
            <div className="text-[12px] text-faint">{new Date(f.at).toLocaleDateString()}</div>
            <IconButton size="sm" icon={Trash2} label="Remove from history" onClick={() => void api.counters.forgetFound(f.id)} />
          </div>
        ))}
      </Card>
    </section>
  );
}

export function CountersPage() {
  const [state, setState] = useState<CountersState | null>(null);
  const [editor, setEditor] = useState<{ counter: Counter | null; hunt: boolean } | null>(null);
  useEffect(() => {
    void api.counters.state().then(setState);
  }, []);
  useEvent<CountersState>('counters:changed', setState);
  const counters = state?.counters ?? [];
  return (
    <Page
      title="Counters"
      subtitle="Count with a click or a key from anywhere, time it, pop it out over your game, and hunt shinies."
      actions={
        <>
          <Switch checked={state?.keysOn ?? true} onChange={(v) => void api.counters.keysOn(v)} label="Keys count" />
          <span className="text-[12.5px] text-dim">Keys count</span>
          <Button icon={Plus} onClick={() => setEditor({ counter: null, hunt: false })}>
            New counter
          </Button>
          <Button variant="primary" icon={Sparkles} onClick={() => setEditor({ counter: null, hunt: true })}>
            New shiny hunt
          </Button>
        </>
      }
    >
      {state && counters.length === 0 ? (
        <EmptyState icon={Sparkles} title="Nothing counted yet" description="Make a counter for anything (deaths, resets, push-ups), or a shiny hunt with its odds. Give it a key and it counts even while a game is in front." />
      ) : (
        <div className="grid grid-cols-1 gap-4 xl:grid-cols-2">
          {counters.map((c) => (
            <CounterCard key={c.id} counter={c} onEdit={() => setEditor({ counter: c, hunt: !!c.hunt })} />
          ))}
        </div>
      )}
      {state && !state.keysOn && counters.some((c) => c.key || c.minusKey) && (
        <Callout tone="warn" className="mt-4">
          Keys are paused: the buttons still work.
        </Callout>
      )}
      <History found={state?.found ?? []} />
      <CounterEditor open={editor !== null} onClose={() => setEditor(null)} counter={editor?.counter ?? null} huntMode={editor?.hunt ?? false} />
    </Page>
  );
}
