// The speedrun timer's setup: game, category, splits (with your personal
// best and best segments), and the keys that start, split, pause, undo and
// reset it from inside a game. The timer opens in its own window.

import type { SpeedrunSettings, SpeedrunState } from '@shared/types';
import { ArrowDown, ArrowUp, Plus, Timer, Trash2 } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Card, CardHeader } from '../../components/ui/Card';
import { Field, Select, TextInput } from '../../components/ui/Form';
import { toast } from '../../state/toasts';
import { runTime } from '../floats/SpeedrunPopout';

export const KEYS: [number, string][] = [
  ...Array.from({ length: 10 }, (_, i) => [0x60 + i, `Numpad ${i}`] as [number, string]),
  ...Array.from({ length: 12 }, (_, i) => [0x70 + i, `F${i + 1}`] as [number, string]),
  [0x24, 'Home'],
  [0x23, 'End'],
  [0x21, 'Page Up'],
  [0x22, 'Page Down'],
  [0x2d, 'Insert'],
  [0x2e, 'Delete'],
];

function KeyPick({ label, value, onChange }: { label: string; value: number; onChange: (v: number) => void }) {
  return (
    <Field label={label}>
      <Select value={value} onChange={(e) => onChange(Number(e.target.value))}>
        {KEYS.map(([vk, name]) => (
          <option key={vk} value={vk}>
            {name}
          </option>
        ))}
      </Select>
    </Field>
  );
}

export function SpeedrunCard() {
  const [st, setSt] = useState<SpeedrunState | null>(null);
  const [draft, setDraft] = useState<SpeedrunSettings | null>(null);
  useEffect(() => {
    void api.speedrun.state().then((s) => {
      setSt(s);
      setDraft(s.settings);
    }, () => undefined);
  }, []);
  if (!st || !draft) return null;
  const dirty = JSON.stringify(draft) !== JSON.stringify(st.settings);
  const save = async () => {
    try {
      const s = await api.speedrun.save(draft);
      setSt(s);
      setDraft(s.settings);
      toast.saved();
    } catch (e) {
      toast.error('Couldn’t save the splits', errorText(e));
    }
  };
  const setSplit = (i: number, name: string) => setDraft({ ...draft, splits: draft.splits.map((s, k) => (k === i ? { ...s, name } : s)) });
  const move = (i: number, d: -1 | 1) => {
    const j = i + d;
    if (j < 0 || j >= draft.splits.length) return;
    const splits = [...draft.splits];
    [splits[i], splits[j]] = [splits[j], splits[i]];
    setDraft({ ...draft, splits });
  };
  const sumOfBest = draft.splits.every((s) => s.gold != null) ? draft.splits.reduce((a, s) => a + (s.gold ?? 0), 0) : null;
  return (
    <Card className="mt-4 p-5">
      <CardHeader
        icon={Timer}
        title="Speedrun timer"
        subtitle="Splits with your personal best and best segments, in a small window over the game. The keys work inside games while the timer is open."
        actions={
          <div className="flex gap-2">
            {dirty && (
              <Button variant="primary" onClick={() => void save()}>
                Save splits
              </Button>
            )}
            <Button icon={Timer} onClick={() => void api.speedrun.window(true).catch((e: unknown) => toast.error('The timer didn’t open', errorText(e)))}>
              Open timer
            </Button>
          </div>
        }
      />
      <div className="mt-4 grid gap-3 sm:grid-cols-2">
        <Field label="Game">
          <TextInput value={draft.game} maxLength={80} onChange={(e) => setDraft({ ...draft, game: e.target.value })} placeholder="Celeste" />
        </Field>
        <Field label="Category">
          <TextInput value={draft.category} maxLength={60} onChange={(e) => setDraft({ ...draft, category: e.target.value })} placeholder="Any%" />
        </Field>
      </div>
      <div className="mt-4 rounded-xl border border-line">
        <div className="flex items-center gap-2 border-b border-line px-3 py-2 text-[11.5px] uppercase tracking-wide text-faint">
          <span className="flex-1">Split</span>
          <span className="w-20 text-right">Best</span>
          <span className="w-20 text-right">PB</span>
          <span className="w-[100px]" />
        </div>
        {draft.splits.map((s, i) => (
          <div key={i} className="flex items-center gap-2 border-b border-line px-3 py-1.5 last:border-b-0">
            <TextInput value={s.name} maxLength={60} onChange={(e) => setSplit(i, e.target.value)} className="h-8 flex-1" inputSize="sm" aria-label={`Split ${i + 1}`} />
            <span className="w-20 text-right text-[12.5px] tabular-nums text-warn">{s.gold != null ? runTime(s.gold) : '—'}</span>
            <span className="w-20 text-right text-[12.5px] tabular-nums text-dim">{s.pb != null ? runTime(s.pb) : '—'}</span>
            <span className="flex w-[100px] justify-end">
              <IconButton size="sm" icon={ArrowUp} label="Earlier" disabled={i === 0} onClick={() => move(i, -1)} />
              <IconButton size="sm" icon={ArrowDown} label="Later" disabled={i === draft.splits.length - 1} onClick={() => move(i, 1)} />
              <IconButton size="sm" icon={Trash2} label="Remove" disabled={draft.splits.length === 1} onClick={() => setDraft({ ...draft, splits: draft.splits.filter((_, k) => k !== i) })} />
            </span>
          </div>
        ))}
      </div>
      <div className="mt-2 flex items-center justify-between">
        <Button size="sm" variant="ghost" icon={Plus} onClick={() => setDraft({ ...draft, splits: [...draft.splits, { name: `Split ${draft.splits.length + 1}`, pb: null, gold: null }] })}>
          Add a split
        </Button>
        <span className="text-[12px] text-faint">
          {draft.attempts} attempts{sumOfBest != null ? ` · sum of best ${runTime(sumOfBest)}` : ''}
        </span>
      </div>
      <div className="mt-4 grid grid-cols-2 gap-3 sm:grid-cols-4">
        <KeyPick label="Start / split" value={draft.keySplit} onChange={(v) => setDraft({ ...draft, keySplit: v })} />
        <KeyPick label="Pause" value={draft.keyPause} onChange={(v) => setDraft({ ...draft, keyPause: v })} />
        <KeyPick label="Undo split" value={draft.keyUndo} onChange={(v) => setDraft({ ...draft, keyUndo: v })} />
        <KeyPick label="Reset" value={draft.keyReset} onChange={(v) => setDraft({ ...draft, keyReset: v })} />
      </div>
    </Card>
  );
}
