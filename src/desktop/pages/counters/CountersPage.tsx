// Counters: count anything with a click or a key that works from anywhere
// (the game in front still gets the key), with a picture, a timer, a
// pop-out window and shiny hunts with their odds.

import type { Counter, CounterInput, CountersState, EventKind, Found, Hunt } from '@shared/types';
import { Clock, Copy, ExternalLink, Flag, Hash, ListOrdered, Undo2, Zap, Image as ImageIcon, Minus, Pause, Pencil, Play, Plus, RotateCcw, Sparkles, Trash2, Trophy } from 'lucide-react';
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

const EVENT_PRESETS: { name: string; phase: boolean }[] = [
  { name: 'Wrong shiny', phase: true },
  { name: 'Fainted', phase: false },
  { name: 'Ran away', phase: false },
];

function CounterEditor({ open, onClose, counter, huntMode }: { open: boolean; onClose: () => void; counter: Counter | null; huntMode: boolean }) {
  const blankForm = (): CounterInput => ({ name: '', step: 1, key: null, minusKey: null, hunt: null, events: [], goals: [], oddsAlerts: true });
  const [form, setForm] = useState<CounterInput>(blankForm);
  const [goals, setGoals] = useState('');
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    if (!open) return;
    setForm(counter ? { id: counter.id, name: counter.name, step: counter.step, key: counter.key, minusKey: counter.minusKey, hunt: counter.hunt, events: counter.events, goals: counter.goals, oddsAlerts: counter.oddsAlerts } : { ...blankForm(), hunt: huntMode ? blankHunt() : null, events: huntMode ? [{ id: '', name: 'Wrong shiny', key: null, phase: true }] : [] });
    setGoals(counter?.goals.join(', ') ?? '');
  }, [open, counter, huntMode]);
  const hunt = form.hunt;
  const setEvent = (i: number, patch: Partial<EventKind>) => setForm((f) => ({ ...f, events: f.events.map((e, j) => (j === i ? { ...e, ...patch } : e)) }));
  const save = async () => {
    setBusy(true);
    try {
      const name = form.name.trim() || (hunt?.name ? `Shiny ${hunt.name}` : '');
      const g = goals
        .split(/[,\s]+/)
        .map((x) => Math.floor(Number(x)))
        .filter((x) => x > 0);
      await api.counters.save({ ...form, name, goals: g });
      onClose();
    } catch (e) {
      toast.error('Couldn’t save the counter', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  const ready = (form.name.trim() || hunt?.name) && (!hunt || hunt.species > 0) && form.events.every((e) => e.name.trim());
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
      <div className="max-h-[64vh] space-y-4 overflow-y-auto pr-1">
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
        <p className="text-[12px] text-faint">Keys (and controller buttons) work from anywhere, even with a game or emulator in front, and the game still gets them. Holding one counts once.</p>

        <div className="space-y-2 rounded-xl border border-line bg-surface p-3">
          <div className="flex items-center justify-between">
            <div>
              <div className="text-[13px] font-semibold text-fg">Events</div>
              <div className="text-[11.5px] text-faint">Log what happens with its own key. A phase starts a new phase count; the total keeps going.</div>
            </div>
            <div className="flex gap-1">
              {EVENT_PRESETS.filter((p) => !form.events.some((e) => e.name === p.name)).map((p) => (
                <Button key={p.name} size="sm" variant="ghost" onClick={() => setForm((f) => ({ ...f, events: [...f.events, { id: '', name: p.name, key: null, phase: p.phase }] }))}>
                  + {p.name}
                </Button>
              ))}
              <Button size="sm" icon={Plus} onClick={() => setForm((f) => ({ ...f, events: [...f.events, { id: '', name: '', key: null, phase: false }] }))}>
                Event
              </Button>
            </div>
          </div>
          {form.events.map((e, i) => (
            <div key={i} className="flex flex-wrap items-center gap-2">
              <TextInput aria-label="Event name" value={e.name} onChange={(x) => setEvent(i, { name: x.target.value })} placeholder="Name" className="w-44" />
              <KeyPicker label={`Key for ${e.name || 'the event'}`} value={e.key} onChange={(k) => setEvent(i, { key: k })} />
              <Checkbox checked={e.phase} onChange={(v) => setEvent(i, { phase: v })} label="Starts a phase" />
              <IconButton size="sm" icon={Trash2} label="Remove the event" onClick={() => setForm((f) => ({ ...f, events: f.events.filter((_, j) => j !== i) }))} />
            </div>
          ))}
        </div>

        <div className="grid grid-cols-2 gap-3">
          <Field label="Tell me at (counts, comma-separated)" htmlFor="ctr-goals">
            <TextInput id="ctr-goals" value={goals} onChange={(e) => setGoals(e.target.value)} placeholder="1000, 5000" />
          </Field>
          {hunt && (
            <Field label="Odds alerts">
              <div className="flex h-9 items-center">
                <Checkbox checked={form.oddsAlerts} onChange={(v) => setForm({ ...form, oddsAlerts: v })} label="At 1×, 2×, 3×… the odds" />
              </div>
            </Field>
          )}
        </div>

        <Checkbox checked={!!hunt} onChange={(v) => setForm({ ...form, hunt: v ? (hunt ?? blankHunt()) : null })} label="It’s a shiny hunt" />
        {hunt && <HuntFields hunt={hunt} onChange={(h) => setForm((f) => ({ ...f, hunt: h }))} />}
      </div>
    </Modal>
  );
}

/** A short chime made on the spot (no sound files). */
function chime() {
  try {
    const ctx = new AudioContext();
    [880, 1175, 1568, 2093].forEach((f, i) => {
      const o = ctx.createOscillator();
      const g = ctx.createGain();
      o.type = 'triangle';
      o.frequency.value = f;
      const t = ctx.currentTime + i * 0.11;
      g.gain.setValueAtTime(0.0001, t);
      g.gain.exponentialRampToValueAtTime(0.18, t + 0.02);
      g.gain.exponentialRampToValueAtTime(0.0001, t + 0.6);
      o.connect(g).connect(ctx.destination);
      o.start(t);
      o.stop(t + 0.65);
    });
    setTimeout(() => void ctx.close(), 1500);
  } catch {
    /* no sound: fine */
  }
}

/** The shiny moment: the Pokémon, sparkles and a chime. */
function Celebration({ shiny, onDone }: { shiny: { name: string; species: number | null; count: number } | null; onDone: () => void }) {
  const sparks = useMemo(() => Array.from({ length: 70 }, (_, i) => ({ i, x: Math.random() * 100, d: 1.6 + Math.random() * 1.8, s: 6 + Math.random() * 10, delay: Math.random() * 0.6, hue: [48, 330, 190, 280, 140][i % 5] })), [shiny]);
  useEffect(() => {
    if (!shiny) return;
    chime();
    const t = setTimeout(onDone, 4200);
    return () => clearTimeout(t);
  }, [shiny, onDone]);
  if (!shiny) return null;
  return (
    <div className="fixed inset-0 z-[90] grid place-items-center bg-black/60 backdrop-blur-sm" onClick={onDone} role="status" aria-live="polite">
      <style>{`@keyframes shiny-fall{from{transform:translateY(-10vh) rotate(0)}to{transform:translateY(110vh) rotate(540deg)}}@keyframes shiny-pop{0%{transform:scale(.2);opacity:0}60%{transform:scale(1.15);opacity:1}100%{transform:scale(1)}}`}</style>
      {sparks.map((p) => (
        <span key={p.i} className="pointer-events-none absolute top-0 motion-reduce:hidden" style={{ left: `${p.x}%`, width: p.s, height: p.s, background: `hsl(${p.hue} 95% 65%)`, clipPath: 'polygon(50% 0,61% 39%,100% 50%,61% 61%,50% 100%,39% 61%,0 50%,39% 39%)', animation: `shiny-fall ${p.d}s linear ${p.delay}s forwards` }} />
      ))}
      <div className="text-center" style={{ animation: 'shiny-pop .7s cubic-bezier(.2,1.4,.4,1) both' }}>
        {shiny.species ? <img src={spriteUrl(shiny.species, true)} alt="" className="mx-auto h-56 w-56 drop-shadow-[0_0_40px_rgba(255,220,120,0.6)] [image-rendering:pixelated]" /> : <Trophy size={120} className="mx-auto text-warn" />}
        <div className="mt-2 font-display text-[40px] font-extrabold text-white">{shiny.species ? `Shiny ${shiny.name}!` : `${shiny.name} done!`}</div>
        <div className="text-[16px] text-white/80">{shiny.count.toLocaleString()} {shiny.species ? 'encounters' : 'counted'}</div>
      </div>
    </div>
  );
}

function CounterCard({ counter, onEdit, onFound }: { counter: Counter; onEdit: () => void; onFound: (c: Counter) => void }) {
  const now = useNow(counter.timerSince ? 500 : 60_000);
  const [editing, setEditing] = useState(false);
  const [showLog, setShowLog] = useState(false);
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
      onFound(counter);
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
            {counter.phases > 0 && <Badge tone="info">Phase {counter.phases + 1}</Badge>}
          </div>
        </div>
        <div className="flex shrink-0">
          <IconButton icon={Undo2} label="Undo the last change" onClick={() => run(api.counters.undo(counter.id))} />
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
            {counter.phases > 0 && <span className="mt-1 block text-[12px] font-medium text-dim">this phase: {(counter.count - counter.phaseBase).toLocaleString()}</span>}
          </button>
        )}
        <Button size="lg" variant="primary" icon={Plus} aria-label="Add one" onClick={() => run(api.counters.bump(counter.id, 1))} />
      </div>
      {counter.events.length > 0 && (
        <div className="flex flex-wrap gap-1.5">
          {counter.events.map((e) => (
            <Button key={e.id} size="sm" icon={e.phase ? Flag : Zap} onClick={() => run(api.counters.event(counter.id, e.id))}>
              {e.name}
              {e.key && <span className="ml-1 text-[11px] text-faint">{e.key.label}</span>}
            </Button>
          ))}
          {counter.log.length > 0 && (
            <Button size="sm" variant="ghost" icon={ListOrdered} onClick={() => setShowLog(!showLog)}>
              {showLog ? 'Hide log' : `Log (${counter.log.length})`}
            </Button>
          )}
        </div>
      )}
      {showLog && (
        <div className="max-h-40 overflow-y-auto rounded-xl bg-surface px-3 py-2 text-[12px]">
          {counter.log.slice(0, 50).map((l, i) => (
            <div key={i} className="flex justify-between gap-2 py-0.5">
              <span className={l.phase ? 'font-medium text-fg' : 'text-dim'}>{l.name}</span>
              <span className="tabular-nums text-faint">
                at {l.count.toLocaleString()} · {new Date(l.at).toLocaleString(undefined, { dateStyle: 'short', timeStyle: 'short' })}
              </span>
            </div>
          ))}
          <button type="button" onClick={() => run(api.counters.clearLog(counter.id))} className="mt-1 text-[11.5px] text-faint underline">
            Clear the log
          </button>
        </div>
      )}
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

/** Your shiny statistics, from the history. */
function HuntStats({ found }: { found: Found[] }) {
  const shinies = found.filter((f) => f.species);
  if (shinies.length === 0) return null;
  const total = shinies.reduce((a, f) => a + f.count, 0);
  const time = shinies.reduce((a, f) => a + f.timerMs, 0);
  const oddsOf = (f: Found) => Number(f.odds?.match(/1\/([\d,]+)/)?.[1]?.replace(/,/g, '')) || 4096;
  const luckiest = [...shinies].sort((a, b) => a.count / oddsOf(a) - b.count / oddsOf(b))[0];
  const longest = [...shinies].sort((a, b) => b.count - a.count)[0];
  const months = Array.from({ length: 12 }, (_, i) => {
    const d = new Date();
    d.setMonth(d.getMonth() - (11 - i), 1);
    return { label: d.toLocaleDateString(undefined, { month: 'short' }), n: shinies.filter((f) => new Date(f.at).getMonth() === d.getMonth() && new Date(f.at).getFullYear() === d.getFullYear()).length };
  });
  const max = Math.max(1, ...months.map((m) => m.n));
  const tile = (label: string, value: string, sub?: string) => (
    <div className="rounded-xl bg-surface px-3 py-2">
      <div className="text-[11.5px] text-faint">{label}</div>
      <div className="font-display text-[18px] font-semibold tabular-nums text-fg">{value}</div>
      {sub && <div className="truncate text-[11.5px] text-dim">{sub}</div>}
    </div>
  );
  return (
    <section className="mt-8">
      <h2 className="mb-3 font-display text-[15px] font-semibold text-fg">Shiny stats</h2>
      <Card className="space-y-4 p-4">
        <div className="grid grid-cols-2 gap-2 lg:grid-cols-5">
          {tile('Shinies', shinies.length.toString())}
          {tile('Average encounters', Math.round(total / shinies.length).toLocaleString())}
          {tile('Hunting time', formatTimer(time))}
          {tile('Luckiest', luckiest.count.toLocaleString(), luckiest.name)}
          {tile('Longest', longest.count.toLocaleString(), longest.name)}
        </div>
        <div>
          <div className="mb-1 text-[11.5px] text-faint">Finds by month</div>
          <div className="flex h-20 items-end gap-1.5">
            {months.map((m, i) => (
              <div key={i} className="flex flex-1 flex-col items-center gap-1" title={`${m.label}: ${m.n}`}>
                <div className="w-full rounded-t bg-accent/70" style={{ height: `${(m.n / max) * 56}px`, minHeight: m.n ? 4 : 1, opacity: m.n ? 1 : 0.25 }} />
                <span className="text-[10px] text-faint">{m.label}</span>
              </div>
            ))}
          </div>
        </div>
      </Card>
    </section>
  );
}

/** The OBS page for counters, served on this PC. */
function StreamOverlay({ state }: { state: CountersState }) {
  const [port, setPort] = useState(String(state.overlayPort));
  const base = `http://127.0.0.1:${state.overlayPort}`;
  const set = (on: boolean) => void api.counters.overlay(on, Number(port) || 47823).catch((e) => toast.error('Couldn’t start the stream page', errorText(e)));
  return (
    <section className="mt-8">
      <h2 className="mb-3 font-display text-[15px] font-semibold text-fg">On your stream</h2>
      <Card className="space-y-3 p-4">
        <div className="flex flex-wrap items-center gap-3">
          <Switch checked={state.overlayOn} onChange={set} label="Serve the stream page" />
          <span className="text-[13px] text-fg">Show counters in OBS (Browser source)</span>
          <TextInput aria-label="Port" type="number" min={1024} max={65535} value={port} onChange={(e) => setPort(e.target.value)} onBlur={() => state.overlayOn && set(true)} className="ml-auto w-28" />
        </div>
        <p className="text-[12px] text-faint">A see-through page that updates twice a second. Only this PC can open it. Add “?size=96&color=ffd166&timer=0” to change its look.</p>
        {state.overlayOn &&
          state.counters.map((c) => (
            <div key={c.id} className="flex items-center gap-2 text-[12.5px]">
              <span className="w-40 truncate text-dim">{c.name}</span>
              <code className="min-w-0 flex-1 truncate rounded bg-surface px-2 py-1 text-fg">{`${base}/counter/${c.id}`}</code>
              <IconButton
                size="sm"
                icon={Copy}
                label="Copy the address"
                onClick={() => void navigator.clipboard.writeText(`${base}/counter/${c.id}`).then(() => toast.success('Copied', 'Paste it into an OBS Browser source.'))}
              />
            </div>
          ))}
      </Card>
    </section>
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
  const [party, setParty] = useState<{ name: string; species: number | null; count: number } | null>(null);
  useEffect(() => {
    void api.counters.state().then(setState);
  }, []);
  useEvent<CountersState>('counters:changed', setState);
  const counters = state?.counters ?? [];
  return (
    <Page
      title="Counters"
      subtitle="Count with a click, a key or a controller button from anywhere, log events, time it, pop it out over your game, and hunt shinies."
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
            <CounterCard key={c.id} counter={c} onEdit={() => setEditor({ counter: c, hunt: !!c.hunt })} onFound={(x) => setParty({ name: x.hunt?.name ?? x.name, species: x.hunt?.species ?? null, count: x.count })} />
          ))}
        </div>
      )}
      {state && !state.keysOn && counters.some((c) => c.key || c.minusKey) && (
        <Callout tone="warn" className="mt-4">
          Keys are paused: the buttons still work.
        </Callout>
      )}
      <HuntStats found={state?.found ?? []} />
      {state && counters.length > 0 && <StreamOverlay state={state} />}
      <History found={state?.found ?? []} />
      <CounterEditor open={editor !== null} onClose={() => setEditor(null)} counter={editor?.counter ?? null} huntMode={editor?.hunt ?? false} />
      <Celebration shiny={party} onDone={() => setParty(null)} />
    </Page>
  );
}
