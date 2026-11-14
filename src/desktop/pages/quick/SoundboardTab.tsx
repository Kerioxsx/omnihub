// The soundboard: your sound files on buttons, with keys that work inside
// games. Pick an output (a virtual audio cable puts sounds into your mic).

import type { SoundboardSettings } from '@shared/types';
import { FolderOpen, Play, Square, Trash2, Volume2 } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Card, CardHeader } from '../../components/ui/Card';
import { Select, Switch, TextInput } from '../../components/ui/Form';
import { toast } from '../../state/toasts';
import { KEYS } from '../counters/SpeedrunCard';

export function SoundboardTab() {
  const [s, setS] = useState<SoundboardSettings | null>(null);
  const [outputs, setOutputs] = useState<string[]>([]);
  useEffect(() => {
    void api.soundboard.get().then(setS, () => undefined);
    void navigator.mediaDevices
      ?.enumerateDevices()
      .then((d) => setOutputs(d.filter((x) => x.kind === 'audiooutput' && x.label && x.deviceId !== 'default').map((x) => x.label)))
      .catch(() => undefined);
  }, []);
  if (!s) return null;
  const save = async (next: SoundboardSettings) => {
    setS(next);
    try {
      setS(await api.soundboard.save(next));
    } catch (e) {
      toast.error('Couldn’t save the soundboard', errorText(e));
    }
  };
  const add = async () => {
    const files = await api.app.pickFiles('Pick sound files (MP3, WAV, OGG)');
    if (files.length) void save({ ...s, sounds: [...s.sounds, ...files.map((path) => ({ id: '', name: '', path, key: 0, volume: 100 }))] });
  };
  return (
    <div className="space-y-4">
      <Card className="p-5">
        <CardHeader
          icon={Volume2}
          title="Soundboard"
          subtitle="Click a sound, press its key (works inside games), or put it on the deck."
          actions={
            <div className="flex gap-2">
              <Button size="sm" variant="ghost" icon={Square} onClick={() => void api.soundboard.stop()}>
                Stop all
              </Button>
              <Button size="sm" variant="primary" icon={FolderOpen} onClick={() => void add()}>
                Add sounds
              </Button>
            </div>
          }
        />
        <div className="mt-4 grid grid-cols-2 gap-2 sm:grid-cols-4 lg:grid-cols-6">
          {s.sounds.map((x) => (
            <button key={x.id} type="button" onClick={() => void api.soundboard.play(x.id).catch((e: unknown) => toast.error('Didn’t play', errorText(e)))} className="flex aspect-[4/3] flex-col items-center justify-center gap-1.5 rounded-2xl border border-line bg-surface-2/50 p-2 text-center transition-transform hover:border-accent/50 active:scale-95">
              <Play size={20} className="text-accent" />
              <span className="line-clamp-2 text-[12.5px] font-medium text-fg">{x.name}</span>
              {x.key !== 0 && <span className="text-[10.5px] text-faint">{KEYS.find((k) => k[0] === x.key)?.[1]}</span>}
            </button>
          ))}
        </div>
        {s.sounds.length > 0 && (
          <div className="mt-4 divide-y divide-line rounded-xl border border-line">
            {s.sounds.map((x, i) => (
              <div key={x.id} className="flex flex-wrap items-center gap-2 px-3 py-2">
                <TextInput value={x.name} maxLength={40} onChange={(e) => setS({ ...s, sounds: s.sounds.map((y, k) => (k === i ? { ...y, name: e.target.value } : y)) })} onBlur={() => void save(s)} className="h-8 min-w-[140px] flex-1" inputSize="sm" aria-label="Name" />
                <Select value={x.key} onChange={(e) => void save({ ...s, sounds: s.sounds.map((y, k) => (k === i ? { ...y, key: Number(e.target.value) } : y)) })} className="h-8 w-[130px] py-0 text-[12.5px]" aria-label="Key">
                  <option value={0}>No key</option>
                  {KEYS.map(([vk, name]) => (
                    <option key={vk} value={vk}>
                      {name}
                    </option>
                  ))}
                </Select>
                <input type="range" min={10} max={100} step={5} value={x.volume} onChange={(e) => setS({ ...s, sounds: s.sounds.map((y, k) => (k === i ? { ...y, volume: Number(e.target.value) } : y)) })} onMouseUp={() => void save(s)} aria-label="Volume" className="w-24 accent-[var(--accent)]" />
                <IconButton icon={Trash2} label="Remove" onClick={() => void save({ ...s, sounds: s.sounds.filter((_, k) => k !== i) })} />
              </div>
            ))}
          </div>
        )}
      </Card>
      <Card className="flex flex-wrap items-center gap-4 p-5">
        <div className="min-w-0 flex-1">
          <div className="text-[14px] font-semibold text-fg">Where sounds play</div>
          <div className="text-[12.5px] text-dim">To play into your microphone (Discord, games), install a virtual audio cable such as VB-CABLE, pick “CABLE Input” here, and use “CABLE Output” as your microphone there.</div>
        </div>
        <Select value={s.output} onChange={(e) => void save({ ...s, output: e.target.value })} className="w-auto" aria-label="Output">
          <option value="">Default speakers</option>
          {outputs.map((o) => (
            <option key={o} value={o}>
              {o}
            </option>
          ))}
        </Select>
        <label className="flex items-center gap-2 text-[13px] text-dim">
          Keys on <Switch checked={s.keysOn} onChange={(v) => void save({ ...s, keysOn: v })} label="Soundboard keys" />
        </label>
      </Card>
    </div>
  );
}
