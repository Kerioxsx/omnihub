// Volume & calls: the PC's volume, each app's volume and mute, the
// microphone, and the apps in a call right now (Discord, WhatsApp, Nyxen,
// Teams…) with "mute my mic" and "deafen".

import { Headphones, HeadphoneOff, Mic, MicOff, PhoneCall, Volume1, Volume2, VolumeX } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { client, type AppVolume, type Call, type Mixer } from '../client';
import { toast, useApp } from '../state';
import { PullToRefresh } from '../ui/PullToRefresh';
import { Empty, ErrorState, ListSkeleton, SectionTitle } from '../ui/common';
import { SubHeader } from './More';
import { cx, errorMessage, useInterval, usePageVisible, vibrate } from '../lib/util';

const HUES = [262, 199, 330, 152, 28, 220, 285, 175];
export function AppAvatar({ name, icon, size = 40 }: { name: string; icon?: string | null; size?: number }) {
  if (icon) return <img src={icon} alt="" width={size} height={size} className="shrink-0 rounded-xl" style={{ width: size, height: size }} draggable={false} />;
  let h = 0;
  for (const c of name) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  const hue = HUES[h % HUES.length];
  return (
    <div className="grid shrink-0 place-items-center rounded-xl font-display font-bold text-white" style={{ width: size, height: size, fontSize: size * 0.42, background: `linear-gradient(135deg, hsl(${hue} 70% 55%), hsl(${(hue + 40) % 360} 70% 45%))` }} aria-hidden>
      {name.charAt(0).toUpperCase()}
    </div>
  );
}

const minutes = (since: number | null) => {
  if (!since) return null;
  const m = Math.max(0, Math.round((Date.now() - since) / 60000));
  return m < 1 ? 'just now' : m < 60 ? `${m} min` : `${Math.floor(m / 60)} h ${m % 60} min`;
};

/** The mixer, refreshed while shown; local changes win over a refresh for a moment. */
export function useMixer(active: boolean, every = 3000) {
  const enabled = !!useApp((s) => s.info?.features.media);
  const visible = usePageVisible();
  const [mixer, setMixerState] = useState<Mixer | null>(null);
  const [error, setError] = useState<string | null>(null);
  const touched = useRef(0);
  const load = useCallback(async () => {
    try {
      const m = await client.sound();
      if (Date.now() - touched.current > 1500) setMixerState(m);
      setError(null);
    } catch (e) {
      setError(errorMessage(e));
    }
  }, []);
  const on = enabled && active && visible;
  useEffect(() => {
    if (on) void load();
  }, [on, load]);
  useInterval(() => void load(), on ? every : null);

  const local = (m: Mixer) => {
    touched.current = Date.now();
    setMixerState(m);
  };
  const send = async (p: Promise<Mixer>, what: string) => {
    try {
      const m = await p;
      touched.current = 0;
      setMixerState(m);
    } catch (e) {
      toast.error(what, errorMessage(e));
      touched.current = 0;
      void load();
    }
  };
  // Slider drags send a few requests a second at most.
  const timers = useRef(new Map<string, ReturnType<typeof setTimeout>>());
  const later = (key: string, fn: () => void) => {
    const t = timers.current.get(key);
    if (t) clearTimeout(t);
    timers.current.set(key, setTimeout(fn, 160));
  };

  const setApp = (key: string, patch: { level?: number; muted?: boolean }) => {
    if (!mixer) return;
    local({ ...mixer, apps: mixer.apps.map((a) => (a.key === key ? { ...a, level: patch.level ?? a.level, muted: patch.muted ?? a.muted } : a)) });
    const go = () => void send(client.soundApp(key, patch), 'Could not change the volume');
    if (patch.level != null) later(`app:${key}`, go);
    else go();
  };
  const setMaster = (patch: { level?: number; muted?: boolean }) => {
    if (!mixer?.master) return;
    local({ ...mixer, master: { level: patch.level ?? mixer.master.level, muted: patch.muted ?? mixer.master.muted } });
    const go = () => void send(client.soundMaster(patch), 'Could not change the volume');
    if (patch.level != null) later('master', go);
    else go();
  };
  const setMic = (muted: boolean) => {
    if (!mixer?.mic) return;
    vibrate(muted ? [10, 40, 10] : 10);
    local({ ...mixer, mic: { ...mixer.mic, muted } });
    void send(client.soundMic(muted), muted ? 'Could not mute the microphone' : 'Could not turn the microphone on');
  };
  return { enabled, mixer, error, reload: load, setApp, setMaster, setMic };
}

type MixerApi = ReturnType<typeof useMixer>;

function VolumeIcon({ level, muted, size = 20 }: { level: number; muted: boolean; size?: number }) {
  return muted || level === 0 ? <VolumeX size={size} /> : level < 0.5 ? <Volume1 size={size} /> : <Volume2 size={size} />;
}

function Level({ value, muted, onChange, label }: { value: number; muted: boolean; onChange: (v: number) => void; label: string }) {
  return (
    <input
      type="range"
      aria-label={label}
      min={0}
      max={1}
      step={0.01}
      value={muted ? 0 : value}
      onChange={(e) => onChange(Number(e.target.value))}
      className="h-1.5 w-full cursor-pointer appearance-none rounded-full bg-surface-3 accent-[var(--accent)]"
    />
  );
}

/** One call: who, how long, mute my mic, deafen, and that app's volume. */
function CallCard({ call, api, compact }: { call: Call; api: MixerApi; compact?: boolean }) {
  const m = api.mixer!;
  const app = m.apps.find((a) => a.key === call.key);
  const micMuted = !!m.mic?.muted;
  const deaf = !!app?.muted;
  const since = minutes(call.sinceMs);
  return (
    <div className={cx('rounded-2xl border border-line bg-surface p-3.5', compact && 'p-3')}>
      <div className="flex items-center gap-3">
        <div className="relative">
          <AppAvatar name={call.name} size={compact ? 36 : 42} />
          <span className="absolute -bottom-1 -right-1 grid h-5 w-5 place-items-center rounded-full bg-good text-white ring-2 ring-[var(--surface)]">
            <PhoneCall size={11} />
          </span>
        </div>
        <div className="min-w-0 flex-1">
          <div className="truncate text-[15px] font-semibold">In a call · {call.name}</div>
          <div className="truncate text-[12.5px] text-dim">{[since && `On for ${since}`, micMuted ? 'Your mic is muted' : 'Your mic is on'].filter(Boolean).join(' · ')}</div>
        </div>
      </div>
      <div className="mt-3 grid grid-cols-2 gap-2">
        <button type="button" onClick={() => api.setMic(!micMuted)} disabled={!m.mic} className={cx('press flex h-11 items-center justify-center gap-2 rounded-xl text-[14px] font-semibold disabled:opacity-40', micMuted ? 'bg-bad text-white' : 'bg-surface-2 text-fg')} aria-pressed={micMuted}>
          {micMuted ? <MicOff size={18} /> : <Mic size={18} />}
          {micMuted ? 'Unmute mic' : 'Mute mic'}
        </button>
        <button type="button" onClick={() => app && api.setApp(app.key, { muted: !deaf })} disabled={!app} className={cx('press flex h-11 items-center justify-center gap-2 rounded-xl text-[14px] font-semibold disabled:opacity-40', deaf ? 'bg-bad text-white' : 'bg-surface-2 text-fg')} aria-pressed={deaf}>
          {deaf ? <HeadphoneOff size={18} /> : <Headphones size={18} />}
          {deaf ? 'Undeafen' : 'Deafen'}
        </button>
      </div>
      {app && !compact && (
        <div className="mt-3 flex items-center gap-3 text-dim">
          <VolumeIcon level={app.level} muted={app.muted} size={18} />
          <Level label={`${call.name} volume`} value={app.level} muted={app.muted} onChange={(level) => api.setApp(app.key, { level, muted: false })} />
          <span className="w-9 text-right font-mono text-[12px] tabular-nums">{Math.round((app.muted ? 0 : app.level) * 100)}</span>
        </div>
      )}
    </div>
  );
}

/** Calls in progress, for the top of Home and Music. Renders nothing without a call. */
export function CallBar({ active, className }: { active: boolean; className?: string }) {
  const api = useMixer(active, 4000);
  const calls = api.mixer?.calls ?? [];
  if (!api.enabled || !calls.length) return null;
  return (
    <div className={cx('space-y-2', className)}>
      {calls.map((c) => (
        <CallCard key={c.key} call={c} api={api} compact />
      ))}
    </div>
  );
}

function AppRow({ app, api }: { app: AppVolume; api: MixerApi }) {
  return (
    <div className="flex items-center gap-3 px-4 py-3">
      <AppAvatar name={app.name} size={38} />
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-1.5">
          <span className="truncate text-[15px] font-semibold">{app.name}</span>
          {app.active && <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-good" title="Playing sound" />}
        </div>
        <div className="mt-1.5 flex items-center gap-2.5">
          <Level label={`${app.name} volume`} value={app.level} muted={app.muted} onChange={(level) => api.setApp(app.key, { level, muted: false })} />
          <span className="w-8 shrink-0 text-right font-mono text-[12px] tabular-nums text-dim">{Math.round((app.muted ? 0 : app.level) * 100)}</span>
        </div>
      </div>
      <button type="button" onClick={() => api.setApp(app.key, { muted: !app.muted })} className={cx('press grid h-10 w-10 shrink-0 place-items-center rounded-xl', app.muted ? 'bg-bad/15 text-bad' : 'bg-surface-2 text-dim')} aria-label={app.muted ? `Unmute ${app.name}` : `Mute ${app.name}`} aria-pressed={app.muted}>
        <VolumeIcon level={app.level} muted={app.muted} size={18} />
      </button>
    </div>
  );
}

/** Calls, microphone, PC volume and every app's volume. */
export function SoundPanel({ api }: { api: MixerApi }) {
  const m = api.mixer;
  if (!m) return api.error ? <ErrorState message={api.error} onRetry={() => void api.reload()} /> : <ListSkeleton rows={5} />;
  return (
    <div>
      {m.calls.length > 0 && (
        <div className="space-y-2.5">
          {m.calls.map((c) => (
            <CallCard key={c.key} call={c} api={api} />
          ))}
        </div>
      )}

      <div className="card mt-4 overflow-hidden">
        {m.master && (
          <div className="flex items-center gap-3 px-4 py-3.5">
            <button type="button" onClick={() => api.setMaster({ muted: !m.master!.muted })} className={cx('press grid h-11 w-11 shrink-0 place-items-center rounded-2xl', m.master.muted ? 'bg-bad/15 text-bad' : 'bg-accent-soft text-accent')} aria-label={m.master.muted ? 'Unmute the PC' : 'Mute the PC'}>
              <VolumeIcon level={m.master.level} muted={m.master.muted} />
            </button>
            <div className="min-w-0 flex-1">
              <div className="flex justify-between text-[15px] font-semibold">
                <span>PC volume</span>
                <span className="font-mono text-[13px] tabular-nums text-dim">{Math.round((m.master.muted ? 0 : m.master.level) * 100)}</span>
              </div>
              <div className="mt-2">
                <Level label="PC volume" value={m.master.level} muted={m.master.muted} onChange={(level) => api.setMaster({ level, muted: false })} />
              </div>
            </div>
          </div>
        )}
        {m.mic && (
          <button type="button" onClick={() => api.setMic(!m.mic!.muted)} className={cx('press flex w-full items-center gap-3 px-4 py-3.5 text-left', m.master && 'border-t border-line')}>
            <div className={cx('grid h-11 w-11 shrink-0 place-items-center rounded-2xl', m.mic.muted ? 'bg-bad text-white' : 'bg-good/15 text-good')}>{m.mic.muted ? <MicOff size={20} /> : <Mic size={20} />}</div>
            <div className="min-w-0 flex-1">
              <div className="text-[15px] font-semibold">{m.mic.muted ? 'Microphone muted' : 'Microphone on'}</div>
              <div className="text-[12.5px] text-dim">{m.mic.muted ? 'No app hears you — tap to turn it back on' : 'Tap to mute it in every app'}</div>
            </div>
          </button>
        )}
      </div>

      <SectionTitle>Apps</SectionTitle>
      {m.apps.length ? (
        <div className="card divide-y divide-[var(--line)] overflow-hidden">
          {m.apps.map((a) => (
            <AppRow key={a.key} app={a} api={api} />
          ))}
        </div>
      ) : (
        <Empty icon={<Volume2 size={28} />} title="No app is making sound" body="Apps show up here once they play something on the PC." />
      )}
      <p className="mt-4 px-2 text-center text-xs leading-relaxed text-faint">Muting the microphone works in Discord, WhatsApp, Nyxen and every other app. Hanging up happens in the app itself — Windows doesn't let other programs end a call.</p>
    </div>
  );
}

export function SoundPage({ active, onBack }: { active: boolean; onBack: () => void }) {
  const api = useMixer(active);
  const calls = api.mixer?.calls.length ?? 0;
  return (
    <div className="flex h-full flex-col">
      <SubHeader title="Volume & calls" subtitle={calls ? `${calls === 1 ? 'A call' : `${calls} calls`} in progress` : 'Every app on the PC'} onBack={onBack} />
      <PullToRefresh onRefresh={api.reload} className="min-h-0 flex-1">
        <div className="px-safe pb-tabbar">{api.enabled ? <SoundPanel api={api} /> : <Empty icon={<VolumeX size={28} />} title="Turned off on the PC" body="Music and sound from the phone are off in OmniHub → Settings → Music." />}</div>
      </PullToRefresh>
    </div>
  );
}
