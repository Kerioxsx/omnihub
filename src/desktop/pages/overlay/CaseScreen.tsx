// The window on a case screen (a monitor inside the PC): the chosen picture,
// GIF or video, the PC's stats, or the song playing. It never takes focus.

import type { RigMedia, RigSettings, RigStatus, SystemStats } from '@shared/types';
import { useEffect, useState } from 'react';
import { api, on, rigMediaUrl } from '../../api';
import { useNowPlaying } from '../../lib/nowPlaying';

function Stat({ label, value, unit, tone }: { label: string; value: string; unit?: string; tone?: string }) {
  return (
    <div className="min-w-0">
      <div className="text-[1.6vh] font-semibold uppercase tracking-[0.18em] text-white/45">{label}</div>
      <div className="font-display text-[7vh] font-semibold leading-none tabular-nums" style={{ color: tone }}>
        {value}
        {unit && <span className="ml-1 text-[3vh] text-white/50">{unit}</span>}
      </div>
    </div>
  );
}

const heat = (t: number | null) => (t == null ? undefined : t > 85 ? '#ff5a5a' : t > 72 ? '#ffc04d' : '#7cf2c2');

function Stats() {
  const [status, setStatus] = useState<RigStatus | null>(null);
  const [sys, setSys] = useState<SystemStats | null>(null);
  useEffect(() => {
    let alive = true;
    const tick = () => {
      void api.rig.hold().then((s) => alive && setStatus(s), () => undefined);
      void api.app.stats().then((s) => alive && setSys(s), () => undefined);
    };
    tick();
    const t = setInterval(tick, 2000);
    return () => {
      alive = false;
      clearInterval(t);
    };
  }, []);
  const fans = status?.rpm.filter(([n]) => !/pump/i.test(n)).map(([, r]) => r) ?? [];
  const rpm = fans.length ? Math.round(fans.reduce((a, b) => a + b, 0) / fans.length) : null;
  return (
    <div className="grid h-full w-full grid-cols-[repeat(auto-fit,minmax(18vh,1fr))] content-center gap-[4vh] bg-[radial-gradient(ellipse_at_30%_20%,rgba(90,110,230,.25),transparent_60%),#05060a] px-[6vh] text-white">
      <Stat label="CPU" value={status?.cpuTemp != null ? String(Math.round(status.cpuTemp)) : '—'} unit="°C" tone={heat(status?.cpuTemp ?? null)} />
      <Stat label="GPU" value={status?.gpuTemp != null ? String(Math.round(status.gpuTemp)) : '—'} unit="°C" tone={heat(status?.gpuTemp ?? null)} />
      <Stat label="CPU load" value={sys ? String(Math.round(sys.cpu)) : '—'} unit="%" />
      <Stat label="Memory" value={sys ? String(Math.round((sys.memory.used / Math.max(1, sys.memory.total)) * 100)) : '—'} unit="%" />
      <Stat label="Fans" value={rpm != null ? String(rpm) : '—'} unit="RPM" />
      {status?.liquidTemp != null && <Stat label="Liquid" value={status.liquidTemp.toFixed(1)} unit="°C" />}
    </div>
  );
}

function NowPlaying() {
  const { state, art } = useNowPlaying();
  return (
    <div className="relative flex h-full w-full items-center gap-[5vh] overflow-hidden bg-black px-[6vh] text-white">
      {art && <img src={art} alt="" className="absolute inset-0 h-full w-full scale-125 object-cover opacity-40 blur-3xl" />}
      {art ? <img src={art} alt="" className="relative h-[78vh] w-[78vh] shrink-0 rounded-[3vh] object-cover shadow-2xl" /> : <div className="relative h-[78vh] w-[78vh] shrink-0 rounded-[3vh] bg-white/10" />}
      <div className="relative min-w-0">
        <div className="truncate font-display text-[9vh] font-semibold leading-tight">{state?.title ?? 'Nothing playing'}</div>
        <div className="truncate text-[5vh] text-white/70">{state ? `${state.artist}${state.album ? ` · ${state.album}` : ''}` : ''}</div>
      </div>
    </div>
  );
}

export function CaseScreen() {
  const [settings, setSettings] = useState<RigSettings | null>(null);
  const [media, setMedia] = useState<RigMedia[]>([]);
  useEffect(() => {
    document.documentElement.style.background = '#000';
    void api.rig.settings().then(setSettings, () => undefined);
    void api.rig.media().then(setMedia, () => undefined);
    const off = on<RigSettings>('rig:settings', (s) => {
      setSettings(s);
      void api.rig.media().then(setMedia, () => undefined);
    });
    return () => void off.then((u) => u());
  }, []);
  const show = settings?.screens.monitorShow;
  const fit = settings?.screens.fill === false ? 'object-contain' : 'object-cover';
  if (!show || show.kind === 'off') return <div className="h-screen w-screen bg-black" />;
  if (show.kind === 'stats') return <div className="h-screen w-screen"><Stats /></div>;
  if (show.kind === 'nowPlaying') return <div className="h-screen w-screen"><NowPlaying /></div>;
  if (show.kind === 'media') {
    const m = media.find((x) => x.id === show.id);
    if (!m) return <div className="h-screen w-screen bg-black" />;
    return (
      <div className="h-screen w-screen overflow-hidden bg-black">
        {m.kind === 'video' ? <video key={m.id} src={rigMediaUrl(m.id)} autoPlay muted loop playsInline className={`h-full w-full ${fit}`} /> : <img key={m.id} src={rigMediaUrl(m.id)} alt="" className={`h-full w-full ${fit}`} />}
      </div>
    );
  }
  return <div className="h-screen w-screen bg-black" />;
}
