// Minecraft server pieces both the desktop app and the phone use: server
// types and states, player heads and item tiles (drawn here, so nothing is
// fetched), dimensions, durations.

import type { SrvFlavor, SrvItem, SrvState, SrvView } from './types';

const join = (...parts: (string | false | null | undefined)[]) => parts.filter(Boolean).join(' ');

export const FLAVORS: { id: SrvFlavor; label: string; hint: string }[] = [
  { id: 'paper', label: 'Paper', hint: 'Fast vanilla gameplay with plugins (Bukkit, Spigot and Paper plugins). The usual choice.' },
  { id: 'vanilla', label: 'Vanilla', hint: 'Mojang’s own server, exactly like the game. Data packs only.' },
  { id: 'fabric', label: 'Fabric', hint: 'For Fabric mods and modpacks.' },
  { id: 'neoForge', label: 'NeoForge', hint: 'For NeoForge mods and modpacks (most big modpacks).' },
  { id: 'forge', label: 'Forge', hint: 'For Forge mods and older modpacks.' },
  { id: 'quilt', label: 'Quilt', hint: 'For Quilt (and most Fabric) mods.' },
];

export const flavorLabel = (f: SrvFlavor) => FLAVORS.find((x) => x.id === f)?.label ?? f;

export const STATE: Record<SrvState, { label: string; tone: 'good' | 'warn' | 'bad' | 'neutral' | 'info' }> = {
  stopped: { label: 'Stopped', tone: 'neutral' },
  installing: { label: 'Setting up', tone: 'info' },
  starting: { label: 'Starting', tone: 'warn' },
  running: { label: 'Online', tone: 'good' },
  stopping: { label: 'Stopping', tone: 'warn' },
  crashed: { label: 'Crashed', tone: 'bad' },
};

export const busy = (s: SrvView) => s.state === 'starting' || s.state === 'stopping' || s.state === 'installing';
export const up = (s: SrvView) => s.state === 'running' || s.state === 'starting';

function hash(s: string): number {
  let h = 2166136261;
  for (const c of s) h = Math.imul(h ^ c.charCodeAt(0), 16777619) >>> 0;
  return h;
}

/** A blocky face for a player, from their name (no skins are fetched). */
export function Head({ name, size = 28, className }: { name: string; size?: number; className?: string }) {
  const h = hash(name);
  const hue = h % 360;
  const cells = Array.from({ length: 64 }, (_, i) => {
    const x = i % 8;
    const y = Math.floor(i / 8);
    const mirror = x > 3 ? 7 - x : x;
    const v = (hash(`${name}${mirror}${y}`) % 100) / 100;
    if (y < 2 || (y === 2 && v > 0.6)) return `hsl(${(hue + 30) % 360} 45% ${18 + v * 14}%)`;
    if (y === 4 && (x === 1 || x === 2 || x === 5 || x === 6)) return x === 2 || x === 5 ? '#1d2a4a' : '#f4f4f4';
    if (y === 6 && x >= 2 && x <= 5) return `hsl(${hue} 30% 28%)`;
    return `hsl(${(hue + 200) % 360} 30% ${58 + v * 12}%)`;
  });
  return (
    <div className={join('grid shrink-0 grid-cols-8 overflow-hidden rounded-md', className)} style={{ width: size, height: size }} aria-hidden>
      {cells.map((c, i) => (
        <span key={i} style={{ background: c }} />
      ))}
    </div>
  );
}

/** A picture for an item, from what it is (Minecraft's own textures aren't shipped). */
const GLYPHS: [RegExp, string][] = [
  [/elytra/, '🪽'], [/totem/, '🗿'], [/sword/, '🗡️'], [/pickaxe/, '⛏️'], [/_axe$/, '🪓'], [/shovel/, '🥄'], [/_hoe$/, '🌾'], [/^bow$|crossbow/, '🏹'], [/arrow/, '➶'], [/trident/, '🔱'], [/shield/, '🛡️'], [/fishing_rod/, '🎣'],
  [/helmet/, '⛑️'], [/chestplate/, '🦺'], [/leggings/, '👖'], [/boots/, '👢'], [/apple/, '🍎'], [/carrot/, '🥕'], [/potato/, '🥔'], [/bread/, '🍞'], [/beef|porkchop|mutton|chicken|rabbit|steak/, '🍖'], [/cod|salmon|fish/, '🐟'], [/cake|cookie|pie/, '🍰'], [/melon/, '🍉'],
  [/torch|fire/, '🔥'], [/lantern/, '🏮'], [/log|planks|wood|stick/, '🪵'], [/diamond/, '💎'], [/emerald/, '💚'], [/netherite/, '⬛'], [/gold/, '🟡'], [/iron|raw_/, '⚙️'], [/copper/, '🟠'], [/redstone/, '🔴'], [/lapis/, '🔵'], [/coal/, '⚫'], [/ender_pearl|ender_eye/, '🟣'],
  [/firework/, '🎆'], [/potion|bottle/, '🧪'], [/book/, '📖'], [/map/, '🗺️'], [/bucket/, '🪣'], [/shulker|chest|barrel/, '📦'], [/bed$/, '🛏️'], [/boat/, '🛶'], [/minecart/, '🛒'], [/glass/, '🔲'], [/wheat|seeds|sapling/, '🌱'], [/flower|tulip|poppy|dandelion|rose/, '🌸'], [/egg/, '🥚'], [/string|wool/, '🧶'], [/bone/, '🦴'], [/tnt/, '🧨'], [/stone|cobble|brick|deepslate|tile/, '🪨'], [/dirt|sand|gravel|clay/, '🟫'],
];
const glyph = (base: string) => GLYPHS.find(([re]) => re.test(base))?.[1];

/** An item as a tile: a picture from what it is (or its initials), its count, enchantment glint. */
export function ItemTile({ it, size = 44, onClick, selected }: { it: SrvItem | null; size?: number; onClick?: () => void; selected?: boolean }) {
  const base = it?.id.split(':').pop() ?? '';
  const hue = hash(base) % 360;
  const pic = glyph(base);
  const short = pic ?? base.split('_').filter(Boolean).map((w) => w[0].toUpperCase()).slice(0, 2).join('');
  return (
    <button
      type="button"
      disabled={!it || !onClick}
      onClick={onClick}
      title={it ? `${it.custom ? `${it.custom} (${it.label})` : it.label}${it.count > 1 ? ` × ${it.count}` : ''}` : undefined}
      className={join('relative grid place-items-center rounded-md border text-[11px] font-bold transition', it ? 'border-black/20 text-white shadow-inner' : 'border-line bg-surface-2/60', selected && 'ring-2 ring-accent', it && onClick && 'hover:brightness-110')}
      style={{ width: size, height: size, background: it ? `linear-gradient(140deg, hsl(${hue} 45% 52%), hsl(${hue} 50% 36%))` : undefined }}
    >
      {it && <span className={join('drop-shadow', pic && 'text-[20px] leading-none')}>{short}</span>}
      {it?.enchanted && <span className="pointer-events-none absolute inset-0 rounded-md bg-[linear-gradient(120deg,transparent_30%,rgba(200,140,255,.45)_50%,transparent_70%)]" />}
      {it && it.count > 1 && <span className="absolute bottom-0 right-0.5 text-[10.5px] font-bold text-white [text-shadow:1px_1px_0_#000]">{it.count}</span>}
    </button>
  );
}

export const DIMENSIONS = [
  { id: 'overworld', label: 'Overworld' },
  { id: 'the_nether', label: 'Nether' },
  { id: 'the_end', label: 'The End' },
];

export const dimLabel = (d: string | null) => DIMENSIONS.find((x) => x.id === d?.replace('minecraft:', ''))?.label ?? d?.replace('minecraft:', '') ?? 'Overworld';

/** “just now”, or how long ago (“5 min ago”). */
export const agoText = (secs: number) => {
  const a = ago(secs);
  return a === 'just now' ? a : `${a} ago`;
};

export function ago(secs: number): string {
  const s = Math.max(0, Math.floor(Date.now() / 1000 - secs));
  if (s < 60) return 'just now';
  if (s < 3600) return `${Math.floor(s / 60)} min`;
  if (s < 86400) return `${Math.floor(s / 3600)} h ${Math.floor((s % 3600) / 60)} min`;
  return `${Math.floor(s / 86400)} d ${Math.floor((s % 86400) / 3600)} h`;
}
