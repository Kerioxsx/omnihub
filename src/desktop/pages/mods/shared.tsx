// Pieces the Mods page's views share: game art (with a drawn stand-in when
// the picture can't load), the Claude settings window, and progress.

import type { ClaudeSettings, ModGame, ModProgress } from '@shared/types';
import { Box, ExternalLink, KeyRound, Sparkles } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Field, Select, Switch, TextInput } from '../../components/ui/Form';
import { Modal } from '../../components/ui/Overlay';
import { ProgressBar } from '../../components/ui/Progress';
import { Callout } from '../../components/ui/States';
import { useEvent } from '../../lib/hooks';
import { cx } from '../../lib/cx';
import { toast } from '../../state/toasts';

function hue(s: string): number {
  let h = 0;
  for (const c of s) h = (h * 31 + c.charCodeAt(0)) % 360;
  return h;
}

/** A game's header picture over a gradient with its initials (shown until it loads, or if it can't). */
export function GameArt({ game, className }: { game: Pick<ModGame, 'name' | 'image'>; className?: string }) {
  const [state, setState] = useState<'loading' | 'shown' | 'failed'>('loading');
  const h = hue(game.name);
  const initials = game.name
    .split(/[\s:.-]+/)
    .filter((w) => /[A-Za-z0-9]/.test(w))
    .slice(0, 2)
    .map((w) => w[0].toUpperCase())
    .join('');
  return (
    <div className={cx('relative overflow-hidden', className)} style={{ background: `linear-gradient(135deg, hsl(${h} 70% 32%), hsl(${(h + 50) % 360} 75% 18%))` }}>
      {state !== 'shown' && (
        <div className="absolute inset-0 grid place-items-center">
          <span className="font-display text-[28px] font-semibold tracking-tight text-white/85 drop-shadow">{initials || <Box size={26} />}</span>
        </div>
      )}
      {game.image && state !== 'failed' && (
        <img src={game.image} alt="" className={cx('relative h-full w-full object-cover transition-opacity duration-300', state === 'shown' ? 'opacity-100' : 'opacity-0')} onLoad={() => setState('shown')} onError={() => setState('failed')} loading="lazy" referrerPolicy="no-referrer" />
      )}
    </div>
  );
}

/** Progress of installs, from `mods:progress`, for one game (or Minecraft). */
export function useModProgress(game: string): ModProgress | null {
  const [p, setP] = useState<ModProgress | null>(null);
  useEvent<ModProgress>('mods:progress', (e) => {
    if (e.game !== game) return;
    setP(e.done >= e.total || !e.text ? null : e);
  });
  return p;
}

export function ProgressLine({ p }: { p: ModProgress | null }) {
  if (!p) return null;
  return (
    <div className="space-y-1.5 rounded-xl border border-accent/30 bg-accent-soft/40 px-4 py-3">
      <div className="text-[12.5px] font-medium text-fg">{p.text}</div>
      <ProgressBar value={p.total ? (p.done / p.total) * 100 : null} />
    </div>
  );
}

const MODELS: { id: string; label: string }[] = [
  { id: 'claude-opus-5-5', label: 'Claude Opus 5.5 (recommended)' },
  { id: 'claude-sonnet-5-5', label: 'Claude Sonnet 5.5 (faster, cheaper)' },
  { id: 'claude-haiku-5-5', label: 'Claude Haiku 5.5 (cheapest)' },
  { id: 'claude-fable-5-1', label: 'Claude Fable 5.1 (most capable, priciest)' },
];

const EFFORTS: { id: ClaudeSettings['effort']; label: string }[] = [
  { id: 'low', label: 'Quick' },
  { id: 'medium', label: 'Balanced (recommended)' },
  { id: 'high', label: 'Thorough' },
  { id: 'xhigh', label: 'Very thorough' },
  { id: 'max', label: 'Everything it has' },
];

/** Claude's settings: the API key (kept encrypted, never shown), model, effort, web. */
export function ClaudeSettingsModal({ open, onClose, onChange }: { open: boolean; onClose: () => void; onChange?: (s: ClaudeSettings) => void }) {
  const [s, setS] = useState<ClaudeSettings | null>(null);
  const [key, setKey] = useState('');
  const [testing, setTesting] = useState(false);
  useEffect(() => {
    if (open) void api.claude.settings().then(setS, () => undefined);
  }, [open]);
  const update = async (patch: Partial<ClaudeSettings>) => {
    try {
      const n = await api.claude.update(patch);
      setS(n);
      onChange?.(n);
    } catch (e) {
      toast.error('Not saved', errorText(e));
    }
  };
  const saveKey = async (k: string) => {
    try {
      await api.claude.setKey(k);
      setKey('');
      const n = await api.claude.settings();
      setS(n);
      onChange?.(n);
      toast.info(k ? 'Key saved' : 'Key removed', k ? 'Encrypted for your Windows account.' : undefined);
    } catch (e) {
      toast.error('Not saved', errorText(e));
    }
  };
  const test = async () => {
    setTesting(true);
    try {
      await api.claude.test();
      toast.success('Claude answered', 'Your key works.');
    } catch (e) {
      toast.error('Claude didn’t answer', errorText(e));
    } finally {
      setTesting(false);
    }
  };
  return (
    <Modal open={open} onClose={onClose} title="Claude" description="Claude makes mods with your own Anthropic API key. You pay Anthropic for what it uses; OmniHub shows an estimate per mod." icon={Sparkles} size="md">
      {s && (
        <div className="space-y-4">
          <Field label="API key" hint={s.hasKey ? 'A key is saved (encrypted for your Windows account, shown to no one, sent only to Anthropic).' : 'Create one at console.anthropic.com → API keys. It starts with sk-ant-.'}>
            <div className="flex gap-2">
              <TextInput type="password" value={key} onChange={(e) => setKey(e.target.value)} placeholder={s.hasKey ? '•••••••• (saved)' : 'sk-ant-…'} icon={KeyRound} className="flex-1" aria-label="Anthropic API key" autoComplete="off" />
              <Button size="sm" variant="primary" disabled={!key.trim()} onClick={() => void saveKey(key.trim())}>
                Save
              </Button>
              {s.hasKey && (
                <Button size="sm" variant="ghost" onClick={() => void saveKey('')}>
                  Remove
                </Button>
              )}
            </div>
          </Field>
          <div className="flex flex-wrap gap-2">
            <Button size="sm" variant="secondary" disabled={!s.hasKey || testing} onClick={() => void test()}>
              {testing ? 'Checking…' : 'Check the key'}
            </Button>
            <Button size="sm" variant="ghost" icon={ExternalLink} onClick={() => void api.app.openUrl('https://console.anthropic.com/settings/keys')}>
              Get a key
            </Button>
            <Button size="sm" variant="ghost" icon={ExternalLink} onClick={() => void api.app.openUrl('https://console.anthropic.com/settings/usage')}>
              Your usage
            </Button>
          </div>
          <Field label="Model">
            <Select value={s.model} onChange={(e) => void update({ model: e.target.value })}>
              {MODELS.map((m) => (
                <option key={m.id} value={m.id}>
                  {m.label}
                </option>
              ))}
            </Select>
          </Field>
          <Field label="How hard it thinks" hint="More thinking makes better mods for hard requests, and costs more.">
            <Select value={s.effort} onChange={(e) => void update({ effort: e.target.value as ClaudeSettings['effort'] })}>
              {EFFORTS.map((m) => (
                <option key={m.id} value={m.id}>
                  {m.label}
                </option>
              ))}
            </Select>
          </Field>
          <label className="flex items-center justify-between gap-3 text-[13px] text-fg">
            <span>
              Let Claude search the web
              <span className="block text-[11.5px] text-faint">For mod loaders’ documentation. Applies to new mods.</span>
            </span>
            <Switch checked={s.web} onChange={(v) => void update({ web: v })} label="Web search" />
          </label>
          <Callout tone="info">Claude only writes in the mod’s own folder and reads the game’s files without changing them. It makes single-player and offline mods, never cheats for online games.</Callout>
        </div>
      )}
    </Modal>
  );
}

const MODEL_NAMES: Record<string, string> = Object.fromEntries(MODELS.map((m) => [m.id, m.label.replace(/ \(.*\)$/, '')]));

export function modelName(id: string): string {
  return MODEL_NAMES[id] ?? id;
}

export function money(n: number): string {
  return n < 0.01 ? '<$0.01' : `$${n.toFixed(2)}`;
}

export function count(n: number): string {
  return n >= 1_000_000 ? `${(n / 1_000_000).toFixed(1)}M` : n >= 1000 ? `${(n / 1000).toFixed(n >= 10_000 ? 0 : 1)}k` : `${n}`;
}
