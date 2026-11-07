// What the Quick actions page and its pop-ups share: describing actions,
// drawing a button, and recording a key chord.

import type { QuickAction, QuickButton } from '@shared/types';
import { quickIcon } from '@shared/quickIcons';
import { Keyboard, X } from 'lucide-react';
import { useEffect, useState } from 'react';
import { cx } from '../../lib/cx';

export const ACTION_KINDS: { value: QuickAction['kind']; label: string }[] = [
  { value: 'media', label: 'Media key' },
  { value: 'open', label: 'Open an app, file, folder or website' },
  { value: 'game', label: 'Start a game' },
  { value: 'keys', label: 'Press keys' },
  { value: 'text', label: 'Type text' },
  { value: 'mic', label: 'Mute / unmute the microphone' },
  { value: 'counter', label: 'Count (a counter or one of its events)' },
  { value: 'screenshot', label: 'Screenshot' },
  { value: 'lock', label: 'Lock the PC' },
  { value: 'quiet', label: 'Quiet mode (screens off, fans lowest)' },
  { value: 'capriDesktop', label: 'Capri Desktop (open or close)' },
  { value: 'obs', label: 'OBS: scene, stream, recording, replay, mute' },
  { value: 'clipboard', label: 'Clipboard history (PC only)' },
  { value: 'scene', label: 'Run a scene' },
  { value: 'lights', label: 'Lights (Govee): on, off, preset, brightness' },
  { value: 'fans', label: 'Fan profile' },
  { value: 'notifications', label: 'Windows notifications on / off' },
  { value: 'discord', label: 'Discord status on / off' },
  { value: 'sound', label: 'Where the sound comes out' },
];

/** What a scene's steps can be (not another scene or the clipboard). */
export const STEP_KINDS = ACTION_KINDS.filter((k) => k.value !== 'scene' && k.value !== 'clipboard');

export const FAN_LABEL: Record<string, string> = { lowest: 'Lowest (quiet mode)', silent: 'Silent', auto: 'Auto', fast: 'Fast' };

export const OBS_LABEL: Record<string, string> = { scene: 'Switch to a scene', stream: 'Start / stop streaming', record: 'Start / stop recording', replay: 'Save the replay buffer', mute: 'Mute / unmute a source' };

export const MEDIA_LABEL: Record<string, string> = { playPause: 'Play / pause', next: 'Next', previous: 'Previous', volumeUp: 'Volume up', volumeDown: 'Volume down', mute: 'Mute sound' };

export function blankAction(kind: QuickAction['kind']): QuickAction {
  switch (kind) {
    case 'open':
      return { kind, target: '' };
    case 'game':
      return { kind, key: '' };
    case 'keys':
      return { kind, keys: '' };
    case 'text':
      return { kind, text: '' };
    case 'media':
      return { kind, what: 'playPause' };
    case 'counter':
      return { kind, id: '', event: null };
    case 'obs':
      return { kind, what: 'stream', target: null };
    case 'scene':
      return { kind, id: '' };
    case 'lights':
      return { kind, on: true, preset: null, brightness: 80 };
    case 'fans':
      return { kind, profile: 'auto' };
    case 'notifications':
      return { kind, on: false };
    case 'discord':
      return { kind, on: true };
    case 'sound':
      return { kind, output: '' };
    default:
      return { kind } as QuickAction;
  }
}

/** One line on what a button does. */
export function describe(a: QuickAction | null): string {
  if (!a) return 'Does nothing yet';
  switch (a.kind) {
    case 'open':
      return a.target.split(/[\\/]/).pop() || 'Open';
    case 'game':
      return 'Starts a game';
    case 'keys':
      return a.keys;
    case 'text':
      return `Types “${a.text.length > 24 ? `${a.text.slice(0, 24)}…` : a.text}”`;
    case 'media':
      return MEDIA_LABEL[a.what] ?? 'Media';
    case 'mic':
      return 'Microphone on/off';
    case 'counter':
      return a.event ? 'Logs a counter event' : 'Counts one';
    case 'screenshot':
      return 'Whole screen';
    case 'lock':
      return 'Locks the PC';
    case 'clipboard':
      return 'Clipboard history';
    case 'quiet':
      return 'Screens off, fans lowest';
    case 'capriDesktop':
      return 'Opens or closes Capri';
    case 'obs':
      return a.what === 'scene' ? `OBS scene: ${a.target ?? ''}` : a.what === 'mute' ? `OBS: mute ${a.target ?? ''}` : `OBS: ${OBS_LABEL[a.what]?.toLowerCase() ?? a.what}`;
    case 'scene':
      return 'Runs a scene';
    case 'lights':
      return a.on ? `Lights on${a.brightness ? ` at ${a.brightness}%` : ''}${a.preset ? ' with a preset' : ''}` : 'Lights off';
    case 'fans':
      return `Fans: ${FAN_LABEL[a.profile] ?? a.profile}`;
    case 'notifications':
      return a.on ? 'Notifications on' : 'Notifications off (do not disturb)';
    case 'discord':
      return a.on ? 'Discord status on' : 'Discord status off';
    case 'sound':
      return 'Changes where the sound comes out';
  }
}

export function ButtonFace({ button, size = 'md', className }: { button: Pick<QuickButton, 'icon' | 'color' | 'label'>; size?: 'sm' | 'md' | 'lg'; className?: string }) {
  const Icon = quickIcon(button.icon);
  const px = { sm: 16, md: 22, lg: 28 }[size];
  return (
    <span className={cx('grid place-items-center rounded-xl', { sm: 'h-8 w-8', md: 'h-11 w-11', lg: 'h-14 w-14' }[size], className)} style={{ background: `${button.color || '#a78bfa'}26`, color: button.color || '#a78bfa' }}>
      <Icon size={px} />
    </span>
  );
}

function keyName(e: KeyboardEvent, hotkey: boolean): string | null {
  const c = e.code;
  if (/^Key[A-Z]$/.test(c)) return c.slice(3);
  if (/^Digit[0-9]$/.test(c)) return c.slice(5);
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(c)) return c;
  const map: Record<string, string> = hotkey
    ? { Space: 'Space', ArrowUp: 'Up', ArrowDown: 'Down', ArrowLeft: 'Left', ArrowRight: 'Right' }
    : { Space: 'Space', Enter: 'Enter', Escape: 'Esc', Tab: 'Tab', Backspace: 'Backspace', Delete: 'Delete', Insert: 'Insert', Home: 'Home', End: 'End', PageUp: 'PageUp', PageDown: 'PageDown', ArrowUp: 'Up', ArrowDown: 'Down', ArrowLeft: 'Left', ArrowRight: 'Right', PrintScreen: 'PrintScreen' };
  return map[c] ?? null;
}

/**
 * Record a key chord: `hotkey` ones need a modifier and use the names
 * Windows hotkeys take; others are pressed by OmniHub ("Ctrl+Shift+M", "F13").
 */
export function ChordInput({ value, onChange, hotkey, label }: { value: string; onChange: (v: string) => void; hotkey?: boolean; label: string }) {
  const [listening, setListening] = useState(false);
  useEffect(() => {
    if (!listening) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (['Control', 'Shift', 'Alt', 'Meta'].includes(e.key)) return;
      if (e.key === 'Escape' && !e.ctrlKey && !e.altKey && !e.shiftKey) {
        setListening(false);
        return;
      }
      const k = keyName(e, !!hotkey);
      if (!k) return;
      const mods = [e.ctrlKey && 'Ctrl', e.altKey && 'Alt', e.shiftKey && 'Shift', e.metaKey && (hotkey ? 'Super' : 'Win')].filter(Boolean) as string[];
      if (hotkey && !mods.length) return;
      setListening(false);
      onChange([...mods, k].join('+'));
    };
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  }, [listening, onChange, hotkey]);
  return (
    <div className="flex items-center gap-1.5">
      <button
        type="button"
        onClick={() => setListening(true)}
        aria-label={`${label}: ${value || 'none'}. Click, then press the keys.`}
        className={cx('inline-flex h-9 min-w-[150px] items-center gap-2 rounded-lg border px-3 text-[13px] transition-colors', listening ? 'border-accent bg-accent-soft text-accent' : 'border-line bg-surface hover:border-line-strong')}
      >
        <Keyboard size={14} aria-hidden />
        {listening ? (hotkey ? 'Press keys with Ctrl, Alt or Shift…' : 'Press the keys…') : value || 'No keys'}
      </button>
      {value && !listening && (
        <button type="button" onClick={() => onChange('')} aria-label={`Clear ${label.toLowerCase()}`} className="grid h-8 w-8 place-items-center rounded-lg text-faint hover:bg-surface-2 hover:text-fg">
          <X size={14} />
        </button>
      )}
    </div>
  );
}

export function ago(ms: number): string {
  const s = Math.max(0, Math.round((Date.now() - ms) / 1000));
  if (s < 60) return 'just now';
  if (s < 3600) return `${Math.floor(s / 60)} min ago`;
  if (s < 86400) return `${Math.floor(s / 3600)} h ago`;
  return `${Math.floor(s / 86400)} d ago`;
}
