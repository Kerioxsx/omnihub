// Quick actions: the macro deck (here and on the phone), the quick-launch
// wheel (a hotkey shows it at the mouse) and the clipboard history.

import type { AudioOutput, ClipItem, ClipList, Counter, GoveePreset, LibraryGame, ObsInfo, QuickAction, QuickButton, QuickState, Scene, ScreenModes } from '@shared/types';
import { QUICK_COLORS, QUICK_ICONS } from '@shared/quickIcons';
import { ArrowDown, ArrowUp, ClipboardList, Clapperboard, Copy, Monitor, FolderOpen, Grid3x3, Image as ImageIcon, Pause, Pencil, Pin, PinOff, Play, Plus, Radio, Smartphone, Trash2, Disc3 } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { api, errorText } from '../../api';
import { Page } from '../../components/Page';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Card, Kbd } from '../../components/ui/Card';
import { Field, SearchInput, Select, Switch, Tabs, TextInput, Textarea } from '../../components/ui/Form';
import { Modal } from '../../components/ui/Overlay';
import { Callout, EmptyState } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { useEvent } from '../../lib/hooks';
import { navigate, useRoute } from '../../lib/router';
import { confirm } from '../../state/dialogs';
import { toast } from '../../state/toasts';
import { DesktopTab } from './DesktopTab';
import { ACTION_KINDS, ButtonFace, ChordInput, FAN_LABEL, MEDIA_LABEL, OBS_LABEL, STEP_KINDS, ago, blankAction, describe } from './shared';

type Tab = 'deck' | 'scenes' | 'desktop' | 'wheel' | 'clipboard';

// ---------- the button editor (deck and wheel) ----------

function ActionFields({ action, onChange, games, counters, extra }: { action: QuickAction; onChange: (a: QuickAction) => void; games: LibraryGame[]; counters: Counter[]; extra?: Extra }) {
  switch (action.kind) {
    case 'open':
      return (
        <Field label="What to open" hint="A program, file, folder, or a web address starting with https://">
          <div className="flex gap-2">
            <TextInput value={action.target} onChange={(e) => onChange({ ...action, target: e.target.value })} placeholder="C:\Program Files\Discord\Discord.exe" className="flex-1" />
            <Button variant="secondary" icon={FolderOpen} onClick={() => void api.app.pickFiles('Pick what to open').then((f: string[]) => f[0] && onChange({ ...action, target: f[0] }))}>
              Pick
            </Button>
          </div>
        </Field>
      );
    case 'game':
      return (
        <Field label="Game">
          <Select value={action.key} onChange={(e) => onChange({ ...action, key: e.target.value })}>
            <option value="">Pick a game…</option>
            {games.map((g) => (
              <option key={g.key} value={g.key}>
                {g.name}
              </option>
            ))}
          </Select>
        </Field>
      );
    case 'keys':
      return (
        <Field label="Keys to press" hint="Like a Discord mute key, an OBS scene key, or F13–F24 for your own hotkeys.">
          <ChordInput label="Keys" value={action.keys} onChange={(keys) => onChange({ ...action, keys })} />
        </Field>
      );
    case 'text':
      return (
        <Field label="Text to type" hint="Typed into whatever is in front, up to 2000 characters.">
          <Textarea rows={3} value={action.text} maxLength={2000} onChange={(e) => onChange({ ...action, text: e.target.value })} placeholder="gg wp" />
        </Field>
      );
    case 'media':
      return (
        <Field label="Media key">
          <Select value={action.what} onChange={(e) => onChange({ ...action, what: e.target.value as typeof action.what })}>
            {Object.entries(MEDIA_LABEL).map(([v, l]) => (
              <option key={v} value={v}>
                {l}
              </option>
            ))}
          </Select>
        </Field>
      );
    case 'counter': {
      const c = counters.find((x) => x.id === action.id);
      return (
        <div className="grid grid-cols-2 gap-3">
          <Field label="Counter">
            <Select value={action.id} onChange={(e) => onChange({ ...action, id: e.target.value, event: null })}>
              <option value="">Pick a counter…</option>
              {counters.map((x) => (
                <option key={x.id} value={x.id}>
                  {x.name}
                </option>
              ))}
            </Select>
          </Field>
          <Field label="Does">
            <Select value={action.event ?? ''} onChange={(e) => onChange({ ...action, event: e.target.value || null })} disabled={!c}>
              <option value="">Count one</option>
              {c?.events.map((ev) => (
                <option key={ev.id} value={ev.id}>
                  Log “{ev.name}”
                </option>
              ))}
            </Select>
          </Field>
        </div>
      );
    }
    case 'obs':
      return <ObsFields action={action} onChange={onChange} />;
    case 'scene':
      return (
        <Field label="Scene">
          <Select value={action.id} onChange={(e) => onChange({ ...action, id: e.target.value })}>
            <option value="">Pick a scene…</option>
            {(extra?.scenes ?? []).map((x) => (
              <option key={x.id} value={x.id}>
                {x.name}
              </option>
            ))}
          </Select>
        </Field>
      );
    case 'lights':
      return (
        <div className="grid grid-cols-3 gap-3">
          <Field label="Lights">
            <Select value={action.on ? 'on' : 'off'} onChange={(e) => onChange({ ...action, on: e.target.value === 'on' })}>
              <option value="on">On</option>
              <option value="off">Off</option>
            </Select>
          </Field>
          <Field label="Preset">
            <Select value={action.preset ?? ''} disabled={!action.on} onChange={(e) => onChange({ ...action, preset: e.target.value || null })}>
              <option value="">Keep the colours</option>
              {(extra?.presets ?? []).map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </Select>
          </Field>
          <Field label="Brightness">
            <Select value={String(action.brightness ?? '')} disabled={!action.on} onChange={(e) => onChange({ ...action, brightness: e.target.value ? Number(e.target.value) : null })}>
              <option value="">Keep it</option>
              {[10, 15, 25, 40, 50, 70, 85, 100].map((b) => (
                <option key={b} value={b}>
                  {b}%
                </option>
              ))}
            </Select>
          </Field>
        </div>
      );
    case 'fans':
      return (
        <Field label="Fan profile">
          <Select value={action.profile} onChange={(e) => onChange({ ...action, profile: e.target.value as typeof action.profile })}>
            {Object.entries(FAN_LABEL).map(([v, l]) => (
              <option key={v} value={v}>
                {l}
              </option>
            ))}
          </Select>
        </Field>
      );
    case 'notifications':
    case 'discord':
      return (
        <Field label={action.kind === 'discord' ? 'Discord status' : 'Windows notifications'}>
          <Select value={action.on ? 'on' : 'off'} onChange={(e) => onChange({ ...action, on: e.target.value === 'on' })}>
            <option value="on">On</option>
            <option value="off">{action.kind === 'discord' ? 'Off' : 'Off (do not disturb)'}</option>
          </Select>
        </Field>
      );
    case 'music':
      return (
        <Field label="Music to play" hint="In your music app (Spotify or Apple Music). Set a scene to run with a game for music per game.">
          <TextInput value={action.query} maxLength={200} onChange={(e) => onChange({ ...action, query: e.target.value })} placeholder="Lo-fi beats, or a song name" />
        </Field>
      );
    case 'input':
      return (
        <Field label="Microphone">
          <Select value={action.input} onChange={(e) => onChange({ ...action, input: e.target.value })}>
            <option value="">Pick a microphone…</option>
            {(extra?.inputs ?? []).map((o) => (
              <option key={o.id} value={o.id}>
                {o.name}
              </option>
            ))}
          </Select>
        </Field>
      );
    case 'screen': {
      const key = `${action.width}x${action.height}@${action.hz}`;
      return (
        <Field label="Main screen" hint="Until the game closes (for a game's scene), or until Windows restarts.">
          <Select
            value={key}
            onChange={(e) => {
              const [w, rest] = e.target.value.split('x');
              const [h, hz] = rest.split('@');
              onChange({ ...action, width: Number(w), height: Number(h), hz: Number(hz) });
            }}
          >
            {!(extra?.screen?.modes ?? []).some((m) => `${m.width}x${m.height}@${m.hz}` === key) && <option value={key}>{`${action.width}×${action.height} at ${action.hz} Hz`}</option>}
            {(extra?.screen?.modes ?? []).map((m) => (
              <option key={`${m.width}x${m.height}@${m.hz}`} value={`${m.width}x${m.height}@${m.hz}`}>
                {`${m.width}×${m.height} at ${m.hz} Hz`}
              </option>
            ))}
          </Select>
        </Field>
      );
    }
    case 'hdr':
      return (
        <Field label="HDR" hint={extra?.screen && extra.screen.hdr === null ? 'None of your screens supports HDR.' : 'Goes back as it was when the game closes (for a game’s scene).'}>
          <Select value={action.on ? 'on' : 'off'} onChange={(e) => onChange({ ...action, on: e.target.value === 'on' })}>
            <option value="on">On</option>
            <option value="off">Off</option>
          </Select>
        </Field>
      );
    case 'sound':
      return (
        <Field label="Sound comes out of">
          <Select value={action.output} onChange={(e) => onChange({ ...action, output: e.target.value })}>
            <option value="">Pick a device…</option>
            {(extra?.outputs ?? []).map((o) => (
              <option key={o.id} value={o.id}>
                {o.name}
              </option>
            ))}
          </Select>
        </Field>
      );
    default:
      return null;
  }
}

/** Lists the step editors need (scenes, light presets, sound devices). */
interface Extra {
  scenes: Scene[];
  presets: GoveePreset[];
  outputs: AudioOutput[];
  inputs: AudioOutput[];
  screen: ScreenModes | null;
}

function useExtra(open: boolean): Extra {
  const [x, setX] = useState<Extra>({ scenes: [], presets: [], outputs: [], inputs: [], screen: null });
  useEffect(() => {
    if (!open) return;
    void Promise.all([api.quick.state().then((s) => s.scenes, () => []), api.govee.presets().catch(() => []), api.audioOutputs().catch(() => []), api.audioInputs().catch(() => []), api.screenModes().catch(() => null)]).then(([scenes, presets, outputs, inputs, screen]) => setX({ scenes, presets, outputs, inputs, screen }));
  }, [open]);
  return x;
}

function ObsFields({ action, onChange }: { action: Extract<QuickAction, { kind: 'obs' }>; onChange: (a: QuickAction) => void }) {
  const [lists, setLists] = useState<ObsInfo | null>(null);
  const [busy, setBusy] = useState(false);
  const needs = action.what === 'scene' || action.what === 'mute';
  const load = async () => {
    setBusy(true);
    try {
      setLists(await api.quick.obsInfo(true));
    } catch (e) {
      toast.error('Couldn’t reach OBS', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  const options = action.what === 'scene' ? lists?.scenes : lists?.inputs;
  return (
    <div className="space-y-3">
      <Field label="OBS does">
        <Select value={action.what} onChange={(e) => onChange({ ...action, what: e.target.value as typeof action.what, target: null })}>
          {Object.entries(OBS_LABEL).map(([v, l]) => (
            <option key={v} value={v}>
              {l}
            </option>
          ))}
        </Select>
      </Field>
      {needs && (
        <Field label={action.what === 'scene' ? 'Scene' : 'Source'} hint="Type its name as OBS shows it, or load the list from OBS (it must be open).">
          <div className="flex gap-2">
            {options?.length ? (
              <Select value={action.target ?? ''} onChange={(e) => onChange({ ...action, target: e.target.value })} className="flex-1">
                <option value="">Pick…</option>
                {options.map((o) => (
                  <option key={o}>{o}</option>
                ))}
              </Select>
            ) : (
              <TextInput value={action.target ?? ''} onChange={(e) => onChange({ ...action, target: e.target.value })} placeholder={action.what === 'scene' ? 'Gameplay' : 'Mic/Aux'} className="flex-1" />
            )}
            <Button variant="secondary" loading={busy} onClick={() => void load()}>
              Load from OBS
            </Button>
          </div>
        </Field>
      )}
    </div>
  );
}

/** OBS's WebSocket server: its address and password (kept encrypted). */
function ObsCard({ state, save }: { state: QuickState; save: (s: QuickState) => Promise<boolean> }) {
  const [url, setUrl] = useState(state.obsUrl);
  const [password, setPassword] = useState('');
  const [has, setHas] = useState(false);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    void api.quick.obsInfo(false).then((i) => setHas(i.hasPassword), () => undefined);
  }, []);
  const test = async () => {
    setBusy(true);
    try {
      if (url !== state.obsUrl && !(await save({ ...state, obsUrl: url }))) return;
      if (password) {
        await api.quick.obsPassword(password);
        setPassword('');
        setHas(true);
      }
      const i = await api.quick.obsInfo(true);
      toast.success('Connected to OBS', `${i.scenes.length} scene${i.scenes.length === 1 ? '' : 's'}, ${i.inputs.length} source${i.inputs.length === 1 ? '' : 's'}.`);
    } catch (e) {
      toast.error('Couldn’t reach OBS', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Card className="space-y-3 p-4">
      <div className="flex items-center gap-2">
        <Radio size={16} className="text-accent" />
        <div className="font-medium">OBS Studio</div>
        {has && <Badge tone="good">Password saved</Badge>}
      </div>
      <p className="text-[12.5px] text-dim">For OBS buttons: in OBS open Tools → WebSocket Server Settings, enable it, and copy the password from Show Connect Info.</p>
      <div className="grid gap-2 sm:grid-cols-[1fr_1fr_auto]">
        <TextInput value={url} onChange={(e) => setUrl(e.target.value)} placeholder="ws://127.0.0.1:4455" aria-label="OBS address" />
        <TextInput type="password" value={password} onChange={(e) => setPassword(e.target.value)} placeholder={has ? 'Saved (type to change)' : 'Password'} aria-label="OBS password" />
        <Button variant="secondary" loading={busy} onClick={() => void test()}>
          {password || url !== state.obsUrl ? 'Save & test' : 'Test'}
        </Button>
      </div>
    </Card>
  );
}

function ButtonEditor({ open, initial, onClose, onSave, forDeck }: { open: boolean; initial: QuickButton | null; onClose: () => void; onSave: (b: QuickButton) => void; forDeck: boolean }) {
  const [b, setB] = useState<QuickButton>({ id: '', label: '', icon: 'zap', color: QUICK_COLORS[0], action: blankAction('media') });
  const [games, setGames] = useState<LibraryGame[]>([]);
  const [counters, setCounters] = useState<Counter[]>([]);
  useEffect(() => {
    if (!open) return;
    setB(initial ?? { id: '', label: '', icon: 'zap', color: QUICK_COLORS[0], action: blankAction('media') });
    void api.library.games().then(setGames, () => setGames([]));
    void api.counters.state().then((s) => setCounters(s.counters), () => setCounters([]));
  }, [open, initial]);
  const kinds = ACTION_KINDS.filter((k) => !(forDeck && k.value === 'clipboard'));
  const action = b.action ?? blankAction('media');
  const extra = useExtra(open);
  return (
    <Modal
      open={open}
      onClose={onClose}
      title={initial ? 'Edit button' : 'New button'}
      size="lg"
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button onClick={() => onSave({ ...b, label: b.label.trim() || describe(action) })}>Save</Button>
        </>
      }
    >
      <div className="space-y-4">
        <div className="flex items-start gap-3">
          <ButtonFace button={b} size="lg" />
          <Field label="Name" className="flex-1">
            <TextInput value={b.label} maxLength={40} onChange={(e) => setB({ ...b, label: e.target.value })} placeholder={describe(action)} autoFocus />
          </Field>
        </div>
        <Field label="Does">
          <Select
            value={action.kind}
            onChange={(e) => {
              const kind = e.target.value as QuickAction['kind'];
              const icon = ({ media: 'play', open: 'app-window', game: 'gamepad-2', keys: 'keyboard', text: 'message-square', mic: 'mic-off', counter: 'sparkles', screenshot: 'camera', lock: 'lock', clipboard: 'clipboard-list', obs: 'radio', quiet: 'moon', capriDesktop: 'bot', scene: 'zap', lights: 'lightbulb', fans: 'fan', notifications: 'bell-off', discord: 'message-circle', sound: 'headphones', pinWindow: 'monitor', music: 'music' } as Record<string, string>)[kind];
              setB({ ...b, action: blankAction(kind), icon: b.icon === 'zap' || !initial ? icon : b.icon });
            }}
          >
            {kinds.map((k) => (
              <option key={k.value} value={k.value}>
                {k.label}
              </option>
            ))}
          </Select>
        </Field>
        <ActionFields action={action} onChange={(a) => setB({ ...b, action: a })} games={games} counters={counters} extra={extra} />
        <Field label="Icon">
          <div className="flex flex-wrap gap-1">
            {Object.entries(QUICK_ICONS).map(([name, Icon]) => (
              <button key={name} type="button" aria-label={name} aria-pressed={b.icon === name} onClick={() => setB({ ...b, icon: name })} className={cx('grid h-8 w-8 place-items-center rounded-lg border transition-colors', b.icon === name ? 'border-accent bg-accent-soft text-accent' : 'border-transparent text-dim hover:bg-surface-2 hover:text-fg')}>
                <Icon size={15} />
              </button>
            ))}
          </div>
        </Field>
        <Field label="Colour">
          <div className="flex flex-wrap gap-1.5">
            {QUICK_COLORS.map((c) => (
              <button key={c} type="button" aria-label={`Colour ${c}`} aria-pressed={b.color === c} onClick={() => setB({ ...b, color: c })} className={cx('h-7 w-7 rounded-full border-2 transition-transform hover:scale-110', b.color === c ? 'border-fg' : 'border-transparent')} style={{ background: c }} />
            ))}
          </div>
        </Field>
      </div>
    </Modal>
  );
}

function useQuick() {
  const [state, setState] = useState<QuickState | null>(null);
  useEffect(() => {
    void api.quick.state().then(setState);
  }, []);
  useEvent<QuickState>('quick:changed', setState);
  const save = async (next: QuickState) => {
    try {
      setState(await api.quick.save(next));
      return true;
    } catch (e) {
      toast.error("Couldn't save", errorText(e));
      return false;
    }
  };
  return { state, save };
}

async function run(b: QuickButton) {
  if (!b.action) return;
  try {
    const said = await api.quick.run(b.action);
    if (said) toast.success(b.label, said, { key: 'quick-run', duration: 1800 });
  } catch (e) {
    toast.error(b.label, errorText(e));
  }
}

/** A list of buttons to edit (deck or wheel). */
function ButtonList({ buttons, onEdit, onRemove, onMove }: { buttons: QuickButton[]; onEdit: (b: QuickButton) => void; onRemove: (b: QuickButton) => void; onMove: (i: number, d: -1 | 1) => void }) {
  return (
    <div className="divide-y divide-line rounded-xl border border-line">
      {buttons.map((b, i) => (
        <div key={b.id || i} className="flex items-center gap-3 px-3 py-2">
          <ButtonFace button={b} size="sm" />
          <div className="min-w-0 flex-1">
            <div className="truncate text-[13.5px] font-medium">{b.label}</div>
            <div className="truncate text-[12px] text-faint">{describe(b.action)}</div>
          </div>
          <IconButton icon={ArrowUp} label="Move up" size="sm" variant="ghost" disabled={i === 0} onClick={() => onMove(i, -1)} />
          <IconButton icon={ArrowDown} label="Move down" size="sm" variant="ghost" disabled={i === buttons.length - 1} onClick={() => onMove(i, 1)} />
          <IconButton icon={Play} label={`Try ${b.label}`} size="sm" variant="ghost" onClick={() => void run(b)} />
          <IconButton icon={Pencil} label={`Edit ${b.label}`} size="sm" variant="ghost" onClick={() => onEdit(b)} />
          <IconButton icon={Trash2} label={`Remove ${b.label}`} size="sm" variant="ghost" onClick={() => onRemove(b)} />
        </div>
      ))}
    </div>
  );
}

function useEditing(state: QuickState | null, save: (s: QuickState) => Promise<boolean>, which: 'deck' | 'wheel') {
  const [editing, setEditing] = useState<QuickButton | null | undefined>(undefined);
  const list = state?.[which] ?? [];
  const put = async (b: QuickButton) => {
    if (!state) return;
    const next = b.id && list.some((x) => x.id === b.id) ? list.map((x) => (x.id === b.id ? b : x)) : [...list, b];
    if (await save({ ...state, [which]: next })) setEditing(undefined);
  };
  const remove = (b: QuickButton) => state && void save({ ...state, [which]: list.filter((x) => x.id !== b.id) });
  const move = (i: number, d: -1 | 1) => {
    if (!state) return;
    const next = [...list];
    [next[i], next[i + d]] = [next[i + d], next[i]];
    void save({ ...state, [which]: next });
  };
  return { editing, setEditing, list, put, remove, move };
}

function DeckTab({ state, save }: { state: QuickState; save: (s: QuickState) => Promise<boolean> }) {
  const e = useEditing(state, save, 'deck');
  return (
    <div className="grid gap-6 xl:grid-cols-[minmax(0,1fr)_380px]">
      <div className="space-y-4">
        <Card className="p-5">
          <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
            <div>
              <div className="font-medium">Your deck</div>
              <div className="text-[12.5px] text-dim">Tap a button to run it. On your phone: More → Deck.</div>
            </div>
            <div className="flex items-center gap-2 text-[13px] text-dim">
              Columns
              <Select value={state.deckColumns} onChange={(ev) => void save({ ...state, deckColumns: Number(ev.target.value) })} className="w-20">
                {[2, 3, 4, 5, 6].map((n) => (
                  <option key={n}>{n}</option>
                ))}
              </Select>
            </div>
          </div>
          {state.deck.length ? (
            <div className="grid gap-3" style={{ gridTemplateColumns: `repeat(${state.deckColumns}, minmax(0, 1fr))` }}>
              {state.deck.map((b) => (
                <button key={b.id} type="button" onClick={() => void run(b)} className="group flex aspect-square flex-col items-center justify-center gap-2 rounded-2xl border border-line bg-surface-2 p-2 text-center transition-all hover:-translate-y-0.5 hover:border-line-strong active:scale-95" style={{ boxShadow: `inset 0 -3px 0 ${b.color}55` }}>
                  <ButtonFace button={b} size="lg" />
                  <span className="line-clamp-2 text-[12.5px] font-medium leading-tight">{b.label}</span>
                </button>
              ))}
            </div>
          ) : (
            <EmptyState compact icon={Grid3x3} title="No buttons yet" description="Add media keys, apps, games, key presses, text or counters." />
          )}
        </Card>
        <ObsCard state={state} save={save} />
        <Callout icon={Smartphone} title="On your phone">
          Pair your phone (Phone page), then open More → Deck. The phone only sees each button's name, icon and colour, and can only run the buttons you made here.
        </Callout>
      </div>
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <div className="text-[13px] font-medium text-dim">
            Buttons ({state.deck.length}/30)
          </div>
          <Button size="sm" icon={Plus} onClick={() => e.setEditing(null)} disabled={state.deck.length >= 30}>
            Add button
          </Button>
        </div>
        <ButtonList buttons={e.list} onEdit={(b) => e.setEditing(b)} onRemove={e.remove} onMove={e.move} />
      </div>
      <ButtonEditor open={e.editing !== undefined} initial={e.editing ?? null} onClose={() => e.setEditing(undefined)} onSave={(b) => void e.put(b)} forDeck />
    </div>
  );
}

/** The wheel as it shows at the mouse (also the preview). */
export function Wheel({ buttons, active, onPick, onHover, size = 400 }: { buttons: QuickButton[]; active: number; onPick: (i: number) => void; onHover?: (i: number) => void; size?: number }) {
  const n = Math.max(buttons.length, 1);
  const r = size * 0.36;
  return (
    <div className="relative" style={{ width: size, height: size }}>
      <div className="absolute inset-[6%] rounded-full border border-white/10 bg-[rgba(14,14,22,0.82)] shadow-[0_30px_80px_-20px_rgba(0,0,0,0.7)] backdrop-blur-xl" />
      <div className="absolute inset-[34%] grid place-items-center rounded-full border border-white/10 bg-white/5 text-center">
        <div className="px-2 text-[13px] font-medium text-white">{buttons[active]?.label ?? 'Quick launch'}</div>
      </div>
      {buttons.map((b, i) => {
        const a = (i / n) * Math.PI * 2 - Math.PI / 2;
        const x = size / 2 + Math.cos(a) * r;
        const y = size / 2 + Math.sin(a) * r;
        return (
          <button
            key={b.id || i}
            type="button"
            onClick={() => onPick(i)}
            onMouseEnter={() => onHover?.(i)}
            aria-label={`${i + 1}: ${b.label}`}
            className={cx('absolute flex -translate-x-1/2 -translate-y-1/2 flex-col items-center gap-1 rounded-2xl p-1.5 transition-transform duration-150', i === active && 'scale-[1.18]')}
            style={{ left: x, top: y }}
          >
            <ButtonFace button={b} size="lg" className={cx('transition-shadow', i === active && 'ring-2 ring-white/70')} />
            <span className="rounded bg-black/40 px-1 text-[10px] font-medium text-white/80">{i + 1}</span>
          </button>
        );
      })}
    </div>
  );
}

function WheelTab({ state, save }: { state: QuickState; save: (s: QuickState) => Promise<boolean> }) {
  const e = useEditing(state, save, 'wheel');
  const [active, setActive] = useState(0);
  return (
    <div className="grid gap-6 xl:grid-cols-[minmax(0,1fr)_380px]">
      <Card className="flex flex-col items-center gap-4 p-6">
        <div className="flex w-full flex-wrap items-center justify-between gap-3">
          <div className="flex items-center gap-3">
            <Switch checked={state.wheelOn} onChange={(on) => void save({ ...state, wheelOn: on })} label="Quick-launch wheel" />
            <div>
              <div className="font-medium">Quick-launch wheel</div>
              <div className="text-[12.5px] text-dim">Press the hotkey anywhere: the wheel opens at the mouse. Click, or press 1–8; Esc closes it.</div>
            </div>
          </div>
          <ChordInput hotkey label="Wheel hotkey" value={state.wheelHotkey} onChange={(wheelHotkey) => void save({ ...state, wheelHotkey })} />
        </div>
        <div className="rounded-3xl bg-[radial-gradient(circle_at_center,rgba(124,58,237,0.25),transparent_65%)] p-2">
          <Wheel buttons={state.wheel} active={active} onHover={setActive} onPick={(i) => void run(state.wheel[i])} size={380} />
        </div>
        <Button variant="secondary" size="sm" onClick={() => void api.quick.open('wheel')}>
          Show it now
        </Button>
      </Card>
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <div className="text-[13px] font-medium text-dim">Slots ({state.wheel.length}/8)</div>
          <Button size="sm" icon={Plus} onClick={() => e.setEditing(null)} disabled={state.wheel.length >= 8}>
            Add slot
          </Button>
        </div>
        <ButtonList buttons={e.list} onEdit={(b) => e.setEditing(b)} onRemove={e.remove} onMove={e.move} />
      </div>
      <ButtonEditor open={e.editing !== undefined} initial={e.editing ?? null} onClose={() => e.setEditing(undefined)} onSave={(b) => void e.put(b)} forDeck={false} />
    </div>
  );
}

// ---------- clipboard ----------

export function useClipboard() {
  const [list, setList] = useState<ClipList | null>(null);
  const load = () => void api.clipboard.list().then(setList);
  useEffect(load, []);
  useEvent('clipboard:changed', load);
  return { list, reload: load };
}

export function ClipRow({ item, onPaste, compact }: { item: ClipItem; onPaste?: () => void; compact?: boolean }) {
  const act = (p: Promise<unknown>) => void p.catch((e) => toast.error("Couldn't do that", errorText(e)));
  return (
    <div className="group flex items-start gap-3 rounded-xl px-3 py-2.5 hover:bg-surface-2">
      <button type="button" onClick={onPaste ?? (() => act(api.clipboard.copy(item.id).then(() => toast.success('Copied', undefined, { key: 'clip', duration: 1200 }))))} className="min-w-0 flex-1 text-left">
        {item.thumb ? (
          <span className="flex items-center gap-3">
            <img src={item.thumb} alt="" className="h-14 max-w-[120px] rounded-md border border-line object-cover" />
            <span className="text-[12.5px] text-dim">
              <ImageIcon size={12} className="mr-1 inline" />
              {item.width}×{item.height}
            </span>
          </span>
        ) : (
          <span className={cx('block whitespace-pre-wrap break-words text-[13px]', compact ? 'line-clamp-2' : 'line-clamp-3')}>{item.text}</span>
        )}
        <span className="mt-1 block text-[11.5px] text-faint">
          {item.app ? `${item.app.replace(/\.exe$/, '')} · ` : ''}
          {ago(item.at)}
        </span>
      </button>
      <div className={cx('flex shrink-0 items-center gap-0.5', !item.pinned && 'opacity-0 group-hover:opacity-100 group-focus-within:opacity-100')}>
        {!item.thumb && <IconButton icon={item.pinned ? PinOff : Pin} label={item.pinned ? 'Unpin' : 'Pin (kept after a restart)'} size="sm" variant="ghost" className={cx(item.pinned && 'text-accent')} onClick={() => act(api.clipboard.pin(item.id, !item.pinned))} />}
        {!compact && <IconButton icon={Copy} label="Copy" size="sm" variant="ghost" onClick={() => act(api.clipboard.copy(item.id))} />}
        <IconButton icon={Trash2} label="Remove" size="sm" variant="ghost" onClick={() => act(api.clipboard.remove(item.id))} />
      </div>
    </div>
  );
}

function ClipboardTab({ state, save }: { state: QuickState; save: (s: QuickState) => Promise<boolean> }) {
  const { list } = useClipboard();
  const [q, setQ] = useState('');
  const [ignore, setIgnore] = useState('');
  const items = useMemo(() => {
    const n = q.trim().toLowerCase();
    const all = list?.items ?? [];
    const hits = n ? all.filter((i) => i.text.toLowerCase().includes(n) || i.app?.includes(n)) : all;
    return [...hits.filter((i) => i.pinned), ...hits.filter((i) => !i.pinned)];
  }, [list, q]);
  return (
    <div className="grid gap-6 xl:grid-cols-[minmax(0,1fr)_360px]">
      <Card className="flex min-h-[420px] flex-col p-4">
        <div className="mb-3 flex flex-wrap items-center gap-2">
          <SearchInput value={q} onChange={setQ} placeholder="Search what you copied" className="min-w-[200px] flex-1" />
          <Button variant="secondary" size="sm" icon={list?.paused ? Play : Pause} onClick={() => void api.clipboard.pause(!list?.paused)}>
            {list?.paused ? 'Resume' : 'Pause'}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            icon={Trash2}
            onClick={async () => {
              if (await confirm({ title: 'Clear the clipboard history?', description: 'Pinned items stay.', confirmLabel: 'Clear', tone: 'danger' })) void api.clipboard.clear();
            }}
          >
            Clear
          </Button>
        </div>
        {list?.paused && <Callout tone="warn" className="mb-2" title="Paused">Nothing new is kept until you resume.</Callout>}
        {items.length ? (
          <div className="-mx-1 flex-1 space-y-0.5 overflow-y-auto">
            {items.map((i) => (
              <ClipRow key={i.id} item={i} />
            ))}
          </div>
        ) : (
          <EmptyState compact icon={ClipboardList} title={q ? 'Nothing matches' : 'Nothing copied yet'} description={q ? undefined : `Copy something anywhere; press ${state.clipHotkey} to paste from the history.`} />
        )}
      </Card>
      <div className="space-y-4">
        <Card className="space-y-4 p-4">
          <div className="flex items-center justify-between gap-3">
            <div>
              <div className="font-medium">Keep what I copy</div>
              <div className="text-[12.5px] text-dim">In memory only; pinned items are kept, encrypted.</div>
            </div>
            <Switch checked={state.clipOn} onChange={(on) => void save({ ...state, clipOn: on })} label="Keep what I copy" />
          </div>
          <Field label="Hotkey" hint="Opens the history at the mouse; pick one to paste it.">
            <ChordInput hotkey label="Clipboard hotkey" value={state.clipHotkey} onChange={(clipHotkey) => void save({ ...state, clipHotkey })} />
          </Field>
          <Field label="Keep up to">
            <Select value={state.clipMax} onChange={(e) => void save({ ...state, clipMax: Number(e.target.value) })}>
              {[50, 100, 200, 500, 1000].map((n) => (
                <option key={n} value={n}>
                  {n} items
                </option>
              ))}
            </Select>
          </Field>
        </Card>
        <Card className="space-y-3 p-4">
          <div>
            <div className="font-medium">Never kept</div>
            <div className="text-[12.5px] text-dim">Passwords from OmniHub's vault and from password managers that mark them private are skipped, and so is anything copied from these apps.</div>
          </div>
          <div className="flex flex-wrap gap-1.5">
            {state.clipIgnore.map((a) => (
              <Badge key={a} className="gap-1 pr-1">
                {a}
                <button type="button" aria-label={`Stop ignoring ${a}`} onClick={() => void save({ ...state, clipIgnore: state.clipIgnore.filter((x) => x !== a) })} className="rounded px-1 text-faint hover:text-fg">
                  ×
                </button>
              </Badge>
            ))}
          </div>
          <form
            className="flex gap-2"
            onSubmit={(e) => {
              e.preventDefault();
              const v = ignore.trim().toLowerCase();
              if (!v) return;
              void save({ ...state, clipIgnore: [...state.clipIgnore, v.endsWith('.exe') ? v : `${v}.exe`] }).then((ok) => ok && setIgnore(''));
            }}
          >
            <TextInput value={ignore} onChange={(e) => setIgnore(e.target.value)} placeholder="App, like keepass.exe" className="flex-1" />
            <Button type="submit" variant="secondary" size="md">
              Add
            </Button>
          </form>
        </Card>
      </div>
    </div>
  );
}

export function QuickPage() {
  const route = useRoute();
  const tab = (['deck', 'scenes', 'desktop', 'wheel', 'clipboard'] as Tab[]).find((t) => t === route.params.get('tab')) ?? 'deck';
  const { state, save } = useQuick();
  return (
    <Page title="Quick actions" subtitle="A macro deck for your PC and phone, scenes that set everything in one tap, a quick-launch wheel at your mouse, and clipboard history." actions={state && <Kbd>{tab === 'clipboard' ? state.clipHotkey : state.wheelHotkey}</Kbd>}>
      <Tabs
        className="mb-5"
        value={tab}
        onChange={(t) => navigate('quick', t === 'deck' ? {} : { tab: t })}
        items={[
          { value: 'deck', label: 'Deck', icon: Grid3x3 },
          { value: 'scenes', label: 'Scenes', icon: Clapperboard },
          { value: 'desktop', label: 'Desktop', icon: Monitor },
          { value: 'wheel', label: 'Wheel', icon: Disc3 },
          { value: 'clipboard', label: 'Clipboard', icon: ClipboardList },
        ]}
      />
      {!state ? null : tab === 'deck' ? <DeckTab state={state} save={save} /> : tab === 'scenes' ? <ScenesTab state={state} save={save} /> : tab === 'desktop' ? <DesktopTab /> : tab === 'wheel' ? <WheelTab state={state} save={save} /> : <ClipboardTab state={state} save={save} />}
    </Page>
  );
}



// ---------- scenes ----------

function SceneEditor({ open, initial, onClose, onSave }: { open: boolean; initial: Scene | null; onClose: () => void; onSave: (s: Scene) => void }) {
  const blank: Scene = { id: '', name: '', icon: 'zap', color: QUICK_COLORS[0], steps: [blankAction('lights')] };
  const [sc, setSc] = useState<Scene>(blank);
  const [games, setGames] = useState<LibraryGame[]>([]);
  const [counters, setCounters] = useState<Counter[]>([]);
  const extra = useExtra(open);
  useEffect(() => {
    if (!open) return;
    setSc(initial ?? blank);
    void api.library.games().then(setGames, () => setGames([]));
    void api.counters.state().then((s) => setCounters(s.counters), () => setCounters([]));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, initial]);
  const setStep = (i: number, a: QuickAction) => setSc({ ...sc, steps: sc.steps.map((x, k) => (k === i ? a : x)) });
  const move = (i: number, d: -1 | 1) => {
    const j = i + d;
    if (j < 0 || j >= sc.steps.length) return;
    const steps = [...sc.steps];
    [steps[i], steps[j]] = [steps[j], steps[i]];
    setSc({ ...sc, steps });
  };
  return (
    <Modal
      open={open}
      onClose={onClose}
      title={initial ? 'Edit scene' : 'New scene'}
      description="Steps run one after another when you tap the scene, here, on the deck or on your phone."
      size="lg"
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button disabled={!sc.name.trim() || sc.steps.length === 0} onClick={() => onSave({ ...sc, name: sc.name.trim() })}>
            Save
          </Button>
        </>
      }
    >
      <div className="space-y-4">
        <div className="flex items-start gap-3">
          <ButtonFace button={{ icon: sc.icon, color: sc.color, label: sc.name }} size="lg" />
          <Field label="Name" className="flex-1">
            <TextInput value={sc.name} maxLength={30} onChange={(e) => setSc({ ...sc, name: e.target.value })} placeholder="Gaming" autoFocus />
          </Field>
        </div>
        <div className="space-y-3">
          {sc.steps.map((st, i) => (
            <div key={i} className="rounded-xl border border-line bg-surface-2/40 p-3">
              <div className="mb-2 flex items-center gap-2">
                <span className="grid h-6 w-6 shrink-0 place-items-center rounded-full bg-surface-3 text-[11.5px] font-semibold text-dim">{i + 1}</span>
                <Select value={st.kind} onChange={(e) => setStep(i, blankAction(e.target.value as QuickAction['kind']))} className="h-8 flex-1 py-0 text-[13px]" aria-label={`Step ${i + 1}`}>
                  {STEP_KINDS.map((k) => (
                    <option key={k.value} value={k.value}>
                      {k.label}
                    </option>
                  ))}
                </Select>
                <IconButton size="sm" icon={ArrowUp} label="Earlier" disabled={i === 0} onClick={() => move(i, -1)} />
                <IconButton size="sm" icon={ArrowDown} label="Later" disabled={i === sc.steps.length - 1} onClick={() => move(i, 1)} />
                <IconButton size="sm" icon={Trash2} label="Remove step" onClick={() => setSc({ ...sc, steps: sc.steps.filter((_, k) => k !== i) })} />
              </div>
              <ActionFields action={st} onChange={(a) => setStep(i, a)} games={games} counters={counters} extra={extra} />
            </div>
          ))}
          <Button size="sm" variant="ghost" icon={Plus} disabled={sc.steps.length >= 20} onClick={() => setSc({ ...sc, steps: [...sc.steps, blankAction('fans')] })}>
            Add a step
          </Button>
        </div>
        <div className="grid grid-cols-2 gap-3 rounded-xl border border-line p-3">
          <Field label="Also runs by itself when this game starts" hint="Display profiles per game: a screen mode or HDR step goes back when the game closes.">
            <Select value={sc.game ?? ''} onChange={(e) => setSc({ ...sc, game: e.target.value })}>
              <option value="">Only when I tap it</option>
              {games.map((g) => (
                <option key={g.key} value={g.key}>
                  {g.name}
                </option>
              ))}
            </Select>
          </Field>
          <Field label="When the game closes, run">
            <Select value={sc.after ?? ''} disabled={!sc.game} onChange={(e) => setSc({ ...sc, after: e.target.value })}>
              <option value="">Nothing else</option>
              {extra.scenes
                .filter((x) => x.id !== sc.id)
                .map((x) => (
                  <option key={x.id} value={x.id}>
                    {x.name}
                  </option>
                ))}
            </Select>
          </Field>
        </div>
        <Field label="Icon">
          <div className="flex flex-wrap gap-1">
            {Object.entries(QUICK_ICONS).map(([name, Icon]) => (
              <button key={name} type="button" aria-label={name} aria-pressed={sc.icon === name} onClick={() => setSc({ ...sc, icon: name })} className={cx('grid h-8 w-8 place-items-center rounded-lg border transition-colors', sc.icon === name ? 'border-accent bg-accent-soft text-accent' : 'border-transparent text-dim hover:bg-surface-2 hover:text-fg')}>
                <Icon size={15} />
              </button>
            ))}
          </div>
        </Field>
        <Field label="Colour">
          <div className="flex flex-wrap gap-1.5">
            {QUICK_COLORS.map((c) => (
              <button key={c} type="button" aria-label={`Colour ${c}`} aria-pressed={sc.color === c} onClick={() => setSc({ ...sc, color: c })} className={cx('h-7 w-7 rounded-full border-2 transition-transform hover:scale-110', sc.color === c ? 'border-fg' : 'border-transparent')} style={{ background: c }} />
            ))}
          </div>
        </Field>
      </div>
    </Modal>
  );
}

function ScenesTab({ state, save }: { state: QuickState; save: (s: QuickState) => Promise<boolean> }) {
  const [editing, setEditing] = useState<Scene | null>(null);
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState<string | null>(null);
  const run = async (sc: Scene) => {
    setBusy(sc.id);
    try {
      toast.success(await api.quick.run({ kind: 'scene', id: sc.id }));
    } catch (e) {
      toast.error('Not everything worked', errorText(e));
    } finally {
      setBusy(null);
    }
  };
  const store = async (sc: Scene) => {
    const scenes = sc.id ? state.scenes.map((x) => (x.id === sc.id ? sc : x)) : [...state.scenes, sc];
    if (await save({ ...state, scenes })) setOpen(false);
  };
  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between gap-3">
        <p className="text-[13px] text-dim">One tap sets everything: lights, fans, notifications, Discord, where the sound comes out, a game, keys. Tap them here, on the deck (add a “Run a scene” button) or on your phone’s Deck page.</p>
        <Button
          icon={Plus}
          disabled={state.scenes.length >= 24}
          onClick={() => {
            setEditing(null);
            setOpen(true);
          }}
        >
          New scene
        </Button>
      </div>
      {state.scenes.length === 0 ? (
        <EmptyState icon={Clapperboard} title="No scenes yet" description="Make one for gaming, a movie or bedtime." />
      ) : (
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
          {state.scenes.map((sc) => (
            <Card key={sc.id} className="flex flex-col gap-3 p-4">
              <div className="flex items-center gap-3">
                <ButtonFace button={{ icon: sc.icon, color: sc.color, label: sc.name }} />
                <div className="min-w-0 flex-1">
                  <div className="truncate text-[14.5px] font-semibold text-fg">{sc.name}</div>
                  <div className="text-[12px] text-faint">
                    {sc.steps.length} {sc.steps.length === 1 ? 'step' : 'steps'}
                    {sc.game ? ' · runs with a game' : ''}
                  </div>
                </div>
                <IconButton
                  icon={Pencil}
                  label="Edit"
                  onClick={() => {
                    setEditing(sc);
                    setOpen(true);
                  }}
                />
                <IconButton
                  icon={Trash2}
                  label="Delete"
                  onClick={() =>
                    void confirm({ title: `Delete “${sc.name}”?`, confirmLabel: 'Delete', tone: 'danger' }).then((ok) => ok && save({ ...state, scenes: state.scenes.filter((x) => x.id !== sc.id) }))
                  }
                />
              </div>
              <ul className="space-y-0.5 text-[12.5px] text-dim">
                {sc.steps.slice(0, 5).map((st, i) => (
                  <li key={i} className="truncate">
                    · {describe(st)}
                  </li>
                ))}
                {sc.steps.length > 5 && <li className="text-faint">and {sc.steps.length - 5} more</li>}
              </ul>
              <Button variant="primary" icon={Play} loading={busy === sc.id} onClick={() => void run(sc)} className="mt-auto">
                Run
              </Button>
            </Card>
          ))}
        </div>
      )}
      <SceneEditor open={open} initial={editing} onClose={() => setOpen(false)} onSave={(sc) => void store(sc)} />
    </div>
  );
}
