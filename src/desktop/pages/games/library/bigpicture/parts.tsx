// Big Picture's cards and rows. Every focusable thing is a button marked
// `data-bp-on` while the controller is on it; the shell scrolls it into view.

import type { Browser, LibraryGame, Site } from '@shared/types';
import { Check, ChevronLeft, ChevronRight, Gamepad2, Globe, Library, LogOut, MonitorPlay, MousePointer2, Search, Settings2, Sparkles, Tv } from 'lucide-react';
import { type ReactNode, useEffect, useState } from 'react';
import { cx } from '../../../../lib/cx';
import { GLYPHS, type PadFamily } from '../../../../lib/pads';
import { Cover, STATUS_LABEL, ago, hours } from '../LibraryTab';
import { Glyph } from '../PadPanel';
import { type Item, type NavRow, REMOTE_LEGEND, type Target, since, targetTitle } from './model';

export interface Ctx {
  family: PadFamily;
  /** A quick site's colour by id. */
  color: (site: string) => string;
}

const ICONS: Record<string, typeof Tv> = { library: Library, watch: Tv, pads: Gamepad2, settings: Settings2, leave: LogOut, browser: Globe, remote: MousePointer2, search: Search, sparkles: Sparkles, play: MonitorPlay };

export function Clock() {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const t = setInterval(() => setNow(new Date()), 15_000);
    return () => clearInterval(t);
  }, []);
  return <span className="tabular-nums">{now.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}</span>;
}

/** The OmniHub orbit: a ring with a gap and a moon. */
export function OrbitMark({ size = 32 }: { size?: number }) {
  return (
    <span className="relative inline-block shrink-0 -rotate-[30deg] rounded-full" style={{ width: size, height: size, border: `${Math.round(size / 8)}px solid var(--bp-accent)`, borderRightColor: 'transparent' }}>
      <span className="absolute rounded-full" style={{ width: size / 5, height: size / 5, top: size * 0.28, left: size * 0.28, background: 'var(--bp-accent)' }} />
    </span>
  );
}

/** A small key-cap ("LB", "View", "✥"). */
export function Cap({ children, className }: { children: ReactNode; className?: string }) {
  return <span className={cx('inline-grid h-7 min-w-7 place-items-center rounded-lg border border-white/20 bg-white/10 px-2 text-[12px] font-extrabold tracking-wide text-white', className)}>{children}</span>;
}

export function Hint({ k, label }: { k: ReactNode; label: string }) {
  return (
    <span className="flex items-center gap-2 whitespace-nowrap">
      {typeof k === 'string' ? <Cap>{k}</Cap> : k}
      {label}
    </span>
  );
}

const TRIGGER: Record<PadFamily, string> = { xbox: 'LT', playstation: 'L2', nintendo: 'ZL', generic: 'L2' };

/** One button of the remote legend, drawn for the controller in hand. */
export function LegendKey({ face, family }: { face: (typeof REMOTE_LEGEND)[number]['face']; family: PadFamily }) {
  const g = GLYPHS[family];
  if (face === 'bottom' || face === 'right' || face === 'left' || face === 'top') return <Glyph family={family} which={face} size={30} />;
  const text = { view: g.view, start: g.start, lbrb: `${g.lb} ${g.rb}`, lt: TRIGGER[family], ls: 'L stick', rs: 'R stick', dpad: '✥ D-pad' }[face];
  return <Cap className="h-[30px]">{text}</Cap>;
}

export function RemoteLegend({ family, className }: { family: PadFamily; className?: string }) {
  return (
    <div className={cx('grid grid-cols-2 gap-x-8 gap-y-3 lg:grid-cols-3', className)}>
      {REMOTE_LEGEND.map((l) => (
        <div key={l.face} className="flex items-center gap-3 text-[15px] text-white/85">
          <span className="flex w-[92px] shrink-0 justify-end">
            <LegendKey face={l.face} family={family} />
          </span>
          {l.label}
        </div>
      ))}
    </div>
  );
}

// ---------- art ----------

/** A gradient with an orbit ring, for things without art. */
function Plate({ color, className, children }: { color: string; className?: string; children?: ReactNode }) {
  return (
    <div className={cx('relative h-full w-full overflow-hidden', className)} style={{ background: `radial-gradient(ellipse at 74% 22%, ${color}aa, transparent 55%), linear-gradient(125deg, ${color} -10%, #0b0f1e 85%)` }}>
      <span className="absolute right-[12%] top-[16%] aspect-square w-[34%] rounded-full border-[10px] border-white/10 shadow-[0_0_50px_rgba(255,255,255,0.08)]" />
      <span className="pointer-events-none absolute inset-0" style={{ background: 'linear-gradient(25deg, transparent 43%, rgba(255,255,255,0.07) 43.4%, transparent 70%)' }} />
      {children}
    </div>
  );
}

function Img({ src, className, fallback }: { src: string | null | undefined; className?: string; fallback: ReactNode }) {
  const [bad, setBad] = useState(false);
  useEffect(() => setBad(false), [src]);
  if (!src || bad) return <>{fallback}</>;
  return <img src={src} alt="" draggable={false} loading="lazy" onError={() => setBad(true)} className={cx('h-full w-full object-cover', className)} />;
}

function targetColor(t: Target, ctx: Ctx): string {
  if (t.kind === 'site') return t.site.color;
  if (t.kind === 'watching') return ctx.color(t.w.site);
  const hue = [...t.game.name].reduce((h, c) => (h * 31 + c.charCodeAt(0)) % 360, 7);
  return `hsl(${hue} 55% 38%)`;
}

/** Landscape art for a game, a show or a channel. */
function WideArt({ t, ctx }: { t: Target; ctx: Ctx }) {
  const plate = <Plate color={targetColor(t, ctx)} />;
  if (t.kind === 'game') return <Img src={t.game.art.header ?? t.game.art.hero} fallback={<Cover game={t.game} bare />} />;
  if (t.kind === 'watching') return <Img src={t.w.thumb} fallback={plate} />;
  return plate;
}

function subFor(t: Target): string {
  if (t.kind === 'game') return t.game.lastPlayed ? `Played ${ago(t.game.lastPlayed).toLowerCase()}` : t.game.minutes ? `${hours(t.game.minutes)} played` : 'Not played yet';
  if (t.kind === 'watching') return [t.w.episode, since(t.w.at)].filter(Boolean).join(' · ');
  return 'Opens full screen';
}

function badgeFor(t: Target): string {
  if (t.kind === 'game') return t.game.source;
  if (t.kind === 'watching') return t.w.service;
  return 'Channel';
}

// ---------- rows ----------

type Pick = (ri: number, ci: number) => void;

/** The big card at the top of Home. */
function Hero({ row, ri, on, at, ctx, pick }: { row: NavRow; ri: number; on: boolean; at: number; ctx: Ctx; pick: Pick }) {
  const it = row.items[at];
  if (!it || it.type !== 'feature') return null;
  const t = it.target;
  const art = t.kind === 'game' ? t.game.art.hero : t.kind === 'watching' ? t.w.thumb : null;
  const watching = t.kind === 'watching';
  const pills =
    t.kind === 'game'
      ? [t.game.source, t.game.minutes ? `${hours(t.game.minutes)} played` : 'New', t.game.status ? STATUS_LABEL[t.game.status] : ''].filter(Boolean)
      : t.kind === 'watching'
        ? [t.w.service, t.w.episode, since(t.w.at)].filter(Boolean)
        : ['Channel', 'Full screen', 'Controller remote'];
  const color = targetColor(t, ctx);
  const n = row.items.length;
  return (
    <section key={it.key} className="bp2-hero relative isolate h-[clamp(340px,44vh,520px)] overflow-clip rounded-[26px] border border-white/15 shadow-[0_18px_55px_rgba(3,6,20,0.55)]">
      <div className="absolute inset-0 -z-30" style={{ background: `radial-gradient(ellipse 55% 90% at 79% 67%, ${color}cc, transparent 75%), linear-gradient(125deg, #080d23 20%, ${color}55 100%)` }} />
      <div className="bp2-fade absolute inset-y-0 right-0 -z-20 w-[68%]" style={{ maskImage: 'linear-gradient(90deg, transparent, #000 38%)', WebkitMaskImage: 'linear-gradient(90deg, transparent, #000 38%)' }}>
        {art ? (
          <Img src={art} className="bp2-drift" fallback={null} />
        ) : t.kind === 'game' ? (
          <Cover game={t.game} bare className="scale-125 opacity-70 blur-2xl" />
        ) : (
          <span className="absolute right-[10%] top-[-20%] aspect-square w-[52%] rounded-full border border-white/10 shadow-[0_0_90px_rgba(164,137,255,0.13)]" style={{ background: 'radial-gradient(circle at 55% 60%, rgba(218,208,255,0.05) 0 60%, rgba(167,160,249,0.3) 60.3%, transparent 61%)' }} />
        )}
      </div>
      <div className="absolute inset-0 -z-10" style={{ background: 'linear-gradient(90deg, rgba(5,11,27,0.97) 0%, rgba(6,12,31,0.88) 36%, rgba(7,11,33,0.35) 70%, rgba(8,11,24,0.1)), linear-gradient(180deg, transparent 70%, rgba(0,0,0,0.45))' }} />
      <div className="bp2-rise flex h-full w-[min(64%,780px)] flex-col items-start p-[clamp(24px,2.8vw,48px)]">
        <span className="flex items-center gap-2 text-[12px] font-extrabold uppercase tracking-[0.22em] text-[#c6d0e9]">
          <span className="h-2 w-2 rounded-full bg-[var(--bp-mint)] shadow-[0_0_12px_var(--bp-mint)]" />
          {watching ? 'Continue watching' : t.kind === 'site' ? 'Channels' : 'Pick up where you left off'}
        </span>
        {t.kind === 'game' && t.game.art.logo ? (
          <Img src={t.game.art.logo} className="!h-auto !w-auto !object-contain mt-4 mb-4 max-h-[14vh] max-w-[34vw] object-contain object-left drop-shadow-[0_6px_28px_rgba(0,0,0,0.8)]" fallback={<HeroTitle text={targetTitle(t)} />} />
        ) : (
          <HeroTitle text={targetTitle(t)} />
        )}
        <div className="mb-5 flex flex-wrap gap-2">
          {pills.map((p) => (
            <span key={p} className="rounded-lg border border-white/15 bg-white/[0.08] px-3 py-1.5 text-[12px] font-bold uppercase tracking-wide text-[#d6dced]">
              {p}
            </span>
          ))}
        </div>
        <div className="mt-auto flex items-center gap-3">
          <button
            type="button"
            tabIndex={-1}
            data-bp-on={on || undefined}
            onClick={() => pick(ri, at)}
            className={cx('bp2-card flex items-center gap-3 rounded-2xl px-7 py-4 text-[18px] font-extrabold text-[#10172c]', on && 'bp2-on')}
            style={{ background: 'linear-gradient(120deg, #eef1ff, var(--bp-accent))' }}
          >
            <Glyph family={ctx.family} which="bottom" size={26} />
            {t.kind === 'game' ? 'Play now' : watching ? 'Resume' : 'Open'}
          </button>
          {n > 1 && (
            <span className="flex items-center gap-1 rounded-2xl border border-white/20 bg-white/10 px-4 py-4 text-[15px] font-bold text-white/85">
              <ChevronLeft size={18} /> More <ChevronRight size={18} />
            </span>
          )}
        </div>
      </div>
      {n > 1 && (
        <div className="absolute bottom-5 right-6 flex items-center gap-4">
          <span className="flex gap-1.5">
            {row.items.map((x, i) => (
              <button key={x.key} type="button" tabIndex={-1} aria-label={`Show ${i + 1}`} onClick={() => pick(ri, i)} className={cx('h-2 rounded-full transition-all', i === at ? 'w-7 bg-[var(--bp-accent)]' : 'w-2 bg-white/30 hover:bg-white/60')} />
            ))}
          </span>
          <span className="text-[11px] font-bold uppercase tracking-[0.19em] text-[#9ca5c4]">
            {String(at + 1).padStart(2, '0')} / {String(n).padStart(2, '0')}
          </span>
        </div>
      )}
    </section>
  );
}

function HeroTitle({ text }: { text: string }) {
  // Long show names get smaller so two lines always fit.
  const size = text.length > 24 ? 'text-[clamp(32px,3.4vw,58px)]' : text.length > 12 ? 'text-[clamp(38px,4.4vw,74px)]' : 'text-[clamp(44px,5.4vw,90px)]';
  return <h1 className={cx('mb-4 mt-4 line-clamp-2 font-black uppercase leading-[0.96] tracking-[-0.05em] [text-wrap:balance]', size)}>{text}</h1>;
}

function Head({ row }: { row: NavRow }) {
  if (!row.title) return null;
  return (
    <div className="mb-3 mt-8 flex items-end justify-between gap-4">
      <div>
        <h2 className="text-[21px] font-extrabold tracking-[-0.02em]">{row.title}</h2>
        {row.sub && <p className="mt-0.5 text-[13px] text-[var(--bp-muted)]">{row.sub}</p>}
      </div>
    </div>
  );
}

function WideCard({ t, on, ctx, onClick }: { t: Target; on: boolean; ctx: Ctx; onClick: () => void }) {
  return (
    <button type="button" tabIndex={-1} data-bp-on={on || undefined} onClick={onClick} className={cx('bp2-card group relative aspect-video w-[clamp(250px,20vw,340px)] shrink-0 overflow-hidden rounded-2xl border border-[var(--bp-stroke)] bg-[var(--bp-panel)] text-left', on ? 'bp2-on scale-[1.04]' : 'hover:border-white/30')}>
      <WideArt t={t} ctx={ctx} />
      <span className="absolute inset-0 bg-gradient-to-t from-black/90 via-black/30 to-transparent" />
      <span className="absolute right-2 top-2 rounded-md bg-[#131a33cc] px-2 py-1 text-[10px] font-extrabold uppercase tracking-wide">{badgeFor(t)}</span>
      <span className="absolute inset-x-0 bottom-0 p-3.5">
        <span className="line-clamp-1 block text-[16px] font-extrabold leading-tight">{targetTitle(t)}</span>
        <span className="mt-0.5 line-clamp-1 block text-[12px] text-white/65">{subFor(t)}</span>
      </span>
    </button>
  );
}

function CoverCard({ g, on, onClick }: { g: LibraryGame; on: boolean; onClick: () => void }) {
  return (
    <button type="button" tabIndex={-1} data-bp-on={on || undefined} onClick={onClick} className={cx('bp2-card group min-w-0 overflow-hidden rounded-2xl border border-[var(--bp-stroke)] bg-[var(--bp-panel)] text-left', on ? 'bp2-on -translate-y-1' : 'hover:border-white/30')}>
      <span className="block aspect-[2/3] overflow-hidden">
        <Cover game={g} />
      </span>
      <span className="flex items-center justify-between gap-2 px-3 py-2.5">
        <span className="min-w-0">
          <span className="block truncate text-[13px] font-extrabold">{g.name}</span>
          <span className="block truncate text-[11px] text-[var(--bp-muted)]">{g.minutes ? `${hours(g.minutes)} played` : 'Not played yet'}</span>
        </span>
        {g.favorite && <span className="text-[13px] text-[var(--bp-mint)]">★</span>}
      </span>
    </button>
  );
}

function ChannelTile({ s, on, onClick }: { s: Site; on: boolean; onClick: () => void }) {
  return (
    <button type="button" tabIndex={-1} data-bp-on={on || undefined} onClick={onClick} className={cx('bp2-card relative aspect-video min-w-0 overflow-hidden rounded-2xl border border-[var(--bp-stroke)] text-left', on ? 'bp2-on scale-[1.03]' : 'hover:border-white/30')}>
      <Plate color={s.color}>
        <span className="absolute inset-x-0 bottom-0 p-4">
          <span className="block text-[clamp(20px,1.7vw,30px)] font-black uppercase leading-none tracking-[-0.04em] drop-shadow-[0_4px_12px_rgba(0,0,0,0.6)]">{s.name}</span>
          <span className="mt-1 block text-[12px] font-semibold text-white/70">Opens full screen</span>
        </span>
      </Plate>
    </button>
  );
}

function Toggle({ on }: { on: boolean }) {
  return (
    <span className={cx('relative inline-block h-7 w-12 shrink-0 rounded-full transition-colors', on ? 'bg-[var(--bp-accent)]' : 'bg-white/15')}>
      <span className={cx('absolute top-1 h-5 w-5 rounded-full bg-white shadow transition-all', on ? 'left-6' : 'left-1')} />
    </span>
  );
}

const BROWSER_COLOR: Record<string, string> = { edge: '#2fb5d2', chrome: '#4285f4', brave: '#fb542b', vivaldi: '#ef3939', opera: '#ff1b2d', operagx: '#fa1e4e', firefox: '#ff7139' };

function BrowserCard({ b, selected, on, onClick }: { b: Browser | null; selected: boolean; on: boolean; onClick: () => void }) {
  const color = b ? (BROWSER_COLOR[b.id] ?? '#8da6ff') : '#9ba8c6';
  return (
    <button type="button" tabIndex={-1} data-bp-on={on || undefined} onClick={onClick} className={cx('bp2-card relative flex min-h-[132px] flex-col items-start justify-between rounded-2xl border p-5 text-left', selected ? 'border-[var(--bp-accent)] bg-[#1d2642]' : 'border-[var(--bp-stroke)] bg-[var(--bp-panel)]', on && 'bp2-on')}>
      <span className="grid h-12 w-12 place-items-center rounded-xl" style={{ background: `${color}26`, color }}>
        <Globe size={26} />
      </span>
      <span>
        <span className="block text-[17px] font-extrabold">{b ? b.name : 'My usual browser'}</span>
        <span className="block text-[12px] text-[var(--bp-muted)]">{b ? (b.kind === 'firefox' ? 'Full screen (kiosk)' : 'Full screen, no bars') : 'Windows decides; no controller remote'}</span>
      </span>
      {selected && (
        <span className="absolute right-4 top-4 grid h-7 w-7 place-items-center rounded-full bg-[var(--bp-accent)] text-[#10172c]">
          <Check size={17} strokeWidth={3} />
        </span>
      )}
    </button>
  );
}

function ItemView({ it, on, ctx, onClick }: { it: Item; on: boolean; ctx: Ctx; onClick: () => void }) {
  switch (it.type) {
    case 'wide':
      return <WideCard t={it.target} on={on} ctx={ctx} onClick={onClick} />;
    case 'cover':
      return <CoverCard g={it.game} on={on} onClick={onClick} />;
    case 'channel':
      return <ChannelTile s={it.site} on={on} onClick={onClick} />;
    case 'chip':
      return (
        <button type="button" tabIndex={-1} data-bp-on={on || undefined} onClick={onClick} className={cx('bp2-card rounded-xl border px-4 py-2.5 text-[14px] font-bold', it.selected ? 'border-transparent bg-[var(--bp-accent)] text-[#0e1425]' : 'border-[var(--bp-stroke)] bg-[#1b2439] text-[#c8d2e9]', on && 'bp2-on')}>
          {it.label}
        </button>
      );
    case 'quick': {
      const Icon = ICONS[it.icon] ?? Sparkles;
      return (
        <button type="button" tabIndex={-1} data-bp-on={on || undefined} onClick={onClick} className={cx('bp2-card flex items-center gap-3.5 rounded-2xl border border-[var(--bp-stroke)] p-4 text-left', on ? 'bp2-on bg-[#1a2540]' : 'hover:bg-[#1a2540]')} style={on ? undefined : { background: 'linear-gradient(135deg, #131a2d, #101625)' }}>
          <span className="grid h-11 w-11 shrink-0 place-items-center rounded-xl bg-[#222b4b] text-[var(--bp-accent)]">
            <Icon size={21} />
          </span>
          <span className="min-w-0">
            <span className="block truncate text-[15px] font-extrabold">{it.label}</span>
            <span className="block truncate text-[12px] text-[var(--bp-muted)]">{it.sub}</span>
          </span>
        </button>
      );
    }
    case 'setting':
      return (
        <button type="button" tabIndex={-1} data-bp-on={on || undefined} onClick={onClick} className={cx('bp2-card flex w-full items-center justify-between gap-4 rounded-2xl border px-5 py-4 text-left', on ? 'bp2-on border-transparent bg-[#1a2238]' : 'border-transparent hover:bg-white/[0.04]')}>
          <span className="min-w-0">
            <span className="block text-[16px] font-bold">{it.label}</span>
            {it.sub && <span className="mt-0.5 block max-w-[640px] text-[13px] text-[var(--bp-muted)]">{it.sub}</span>}
          </span>
          {typeof it.value === 'boolean' ? (
            <Toggle on={it.value} />
          ) : (
            <span className="flex shrink-0 items-center gap-1 text-[14px] font-bold text-[var(--bp-accent)]">
              {it.value} <ChevronRight size={18} />
            </span>
          )}
        </button>
      );
    case 'theme': {
      const swatch = { nebula: 'linear-gradient(125deg,#192849,#5949a0)', oled: 'linear-gradient(125deg,#111,#1b1b1b)', aurora: 'linear-gradient(125deg,#082e33,#157871)', ember: 'linear-gradient(125deg,#27202e,#ae5869)' }[it.id];
      return (
        <button type="button" tabIndex={-1} data-bp-on={on || undefined} onClick={onClick} className={cx('bp2-card flex h-[84px] items-end rounded-xl border-2 p-3 text-left text-[13px] font-extrabold', it.selected ? 'border-white' : 'border-transparent', on && 'bp2-on')} style={{ background: swatch }}>
          {it.label}
        </button>
      );
    }
    case 'browser':
      return <BrowserCard b={it.browser} selected={it.selected} on={on} onClick={onClick} />;
    case 'button':
      return (
        <button
          type="button"
          tabIndex={-1}
          data-bp-on={on || undefined}
          onClick={onClick}
          className={cx('bp2-card rounded-2xl border px-7 py-3.5 text-[16px] font-extrabold', it.primary ? 'border-transparent text-[#10172c]' : 'border-white/25 bg-white/10 text-white', on && 'bp2-on')}
          style={it.primary ? { background: 'linear-gradient(120deg, #eef1ff, var(--bp-accent))' } : undefined}
        >
          {it.label}
        </button>
      );
    case 'notice':
      return (
        <button type="button" tabIndex={-1} data-bp-on={on || undefined} onClick={onClick} className={cx('bp2-card flex w-full items-center justify-between gap-5 rounded-2xl border border-[#647cb650] bg-[#26334a66] px-6 py-5 text-left', on && 'bp2-on')}>
          <span>
            <span className="block text-[17px] font-extrabold">{it.title}</span>
            <span className="mt-1 block max-w-[760px] text-[13px] leading-relaxed text-[#c9d7ef]">{it.sub}</span>
          </span>
          <span className="shrink-0 rounded-xl bg-[var(--bp-accent)] px-4 py-2.5 text-[14px] font-extrabold text-[#10172c]">{it.action}</span>
        </button>
      );
    default:
      return null;
  }
}

/** One row as it looks on screen. */
export function RowView({ row, ri, on, at, ctx, pick }: { row: NavRow; ri: number; on: boolean; at: number; ctx: Ctx; pick: Pick }) {
  if (row.kind === 'hero') return <Hero row={row} ri={ri} on={on} at={at} ctx={ctx} pick={pick} />;
  const cell = (it: Item, ci: number) => <ItemView key={it.key} it={it} on={on && ci === at} ctx={ctx} onClick={() => pick(ri, ci)} />;
  const body = (() => {
    switch (row.kind) {
      case 'strip':
        return <div className="-mx-5 -my-5 flex gap-4 overflow-x-hidden px-5 py-5">{row.items.map(cell)}</div>;
      case 'grid':
        return (
          <div className="grid gap-4" style={{ gridTemplateColumns: `repeat(${row.cols ?? 5}, minmax(0, 1fr))` }}>
            {row.items.map(cell)}
          </div>
        );
      case 'chips':
        return <div className="flex flex-wrap gap-2">{row.items.map(cell)}</div>;
      case 'quick':
        return (
          <div className="grid gap-3.5" style={{ gridTemplateColumns: `repeat(${Math.max(1, row.items.length)}, minmax(0, 1fr))` }}>
            {row.items.map(cell)}
          </div>
        );
      case 'themes':
        return <div className="grid max-w-[760px] grid-cols-4 gap-3 px-5">{row.items.map(cell)}</div>;
      case 'browsers':
        return (
          <div className="grid gap-3.5" style={{ gridTemplateColumns: `repeat(${Math.min(4, Math.max(2, row.items.length))}, minmax(0, 1fr))` }}>
            {row.items.map(cell)}
          </div>
        );
      case 'buttons':
        return <div className="flex flex-wrap gap-3">{row.items.map(cell)}</div>;
      default:
        return <div>{row.items.map(cell)}</div>;
    }
  })();
  return (
    <div className={cx(row.kind === 'grid' && !row.title && 'mt-4', row.kind === 'chips' && 'mt-6')}>
      <Head row={row} />
      {body}
    </div>
  );
}
