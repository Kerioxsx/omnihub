// Vesper Client: OmniHub's own Minecraft PvP client, with what it needs
// (version, loader, dependencies), one-click install through the OmniHub
// Client profile, the jar for doing it by hand, and what's in it.

import { Check, CloudRain, Download, Flame, Gauge, Music2, Package, Sparkles, Star, Swords, X } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import { api } from '../../api';
import { Page } from '../../components/Page';
import { Button } from '../../components/ui/Button';
import { Badge, Card, CardHeader } from '../../components/ui/Card';
import { Switch } from '../../components/ui/Form';
import { useSettings } from '../../state/settings';
import { McClientCard } from '../mods/McClient';

export const VESPER_VERSION = '0.4.1';
const JAR = 'https://github.com/Kerioxsx/omnihub/releases/latest/download/Vesper-Client-1.8.9.jar';
const FORGE = 'https://files.minecraftforge.net/net/minecraftforge/forge/index_1.8.9.html';

const REQUIRES: { label: string; value: string; ok?: boolean }[] = [
  { label: 'Minecraft', value: 'Java Edition 1.8.9' },
  { label: 'Loader', value: 'Forge 1.8.9 — 11.15.1.2318 (recommended build)' },
  { label: 'Java', value: 'Java 8 (the one the Minecraft Launcher uses for 1.8.9)' },
  { label: 'Required mods', value: 'None — Vesper is one jar' },
  { label: 'Optional', value: 'PolyPatcher (more FPS; OmniHub adds it with the client). OptiFine 1.8.9 HD U M5 isn’t tested yet.' },
];

const LOADERS: { name: string; ok: boolean; note: string }[] = [
  { name: 'Forge', ok: true, note: '1.8.9 only' },
  { name: 'Fabric', ok: false, note: 'not supported' },
  { name: 'Legacy Fabric', ok: false, note: 'not supported' },
  { name: 'NeoForge', ok: false, note: 'doesn’t exist for 1.8.9' },
];

const FEATURES: { icon: LucideIcon; title: string; text: string }[] = [
  { icon: Sparkles, title: 'Cosmetics', text: 'Wings, comet plume, halo, orbiting glyphs and the Wraith flame head with horns; three palettes.' },
  { icon: Swords, title: 'Kill & hit effects', text: 'Nine kill effects (four funny ones), hit lines, hit sounds and sword animations with 1.7 block-hit.' },
  { icon: Star, title: 'World', text: 'Skies, rain and puddles, neon jump marks, world glyphs, a block outline, time changer and your own world colour.' },
  { icon: Flame, title: 'Emotes', text: 'Flex, wave, dance, salute, cheer and a car flex — press B in game.' },
  { icon: Music2, title: 'Live lyrics', text: 'The song playing in OmniHub shows its synced lyrics in your world.' },
  { icon: Package, title: 'Resource packs', text: 'Browse and download 1.8.9 packs from Modrinth inside the game (Client → Resource packs).' },
  { icon: Gauge, title: 'FPS', text: 'Skips far mobs, caps FPS in menus and in the background, and an FPS boost that puts your settings back when off.' },
  { icon: CloudRain, title: 'Blur & camera', text: 'Menu, motion and speed blur, FOV up to 150 and a smooth zoom.' },
];

export function ClientPage() {
  const media = useSettings((s) => s.settings?.media);
  const update = useSettings((s) => s.update);
  return (
    <Page
      title="Vesper Client"
      subtitle={`OmniHub's own PvP client for Minecraft 1.8.9 · version ${VESPER_VERSION} · only you see its effects`}
      actions={
        <div className="flex gap-2">
          <Button variant="secondary" icon={Download} onClick={() => void api.app.openUrl(JAR)}>
            Download the jar
          </Button>
        </div>
      }
    >
      <div className="grid gap-5 px-8 pb-10">
        <div className="grid gap-5 xl:grid-cols-[1.25fr_1fr]">
          <McClientCard />
          <Card className="p-5">
            <CardHeader icon={Package} title="What you need" subtitle="Vesper is a Forge mod for 1.8.9. OmniHub sets all of this up for you; this is for installing it by hand." />
            <dl className="mt-4 grid gap-2.5 text-[13px]">
              {REQUIRES.map((r) => (
                <div key={r.label} className="grid grid-cols-[110px_1fr] gap-3">
                  <dt className="text-dim">{r.label}</dt>
                  <dd className="text-fg">{r.value}</dd>
                </div>
              ))}
            </dl>
            <div className="mt-4 flex flex-wrap gap-2">
              {LOADERS.map((l) => (
                <Badge key={l.name} tone={l.ok ? 'good' : 'neutral'}>
                  {l.ok ? <Check size={12} /> : <X size={12} />} {l.name} · {l.note}
                </Badge>
              ))}
            </div>
            <ol className="mt-4 list-decimal space-y-1 pl-5 text-[12.5px] text-dim">
              <li>Run Minecraft 1.8.9 once in the official launcher.</li>
              <li>
                Install Forge 1.8.9-11.15.1.2318 ({' '}
                <button type="button" className="underline decoration-line hover:text-fg" onClick={() => void api.app.openUrl(FORGE)}>
                  files.minecraftforge.net
                </button>{' '}
                → Installer → Install client).
              </li>
              <li>Put Vesper-Client-1.8.9.jar in .minecraft/mods and start the forge profile.</li>
            </ol>
          </Card>
        </div>
        <Card className="p-5">
          <CardHeader icon={Sparkles} title="What's inside" subtitle="All original, all adjustable in the game (Vesper → Client)." />
          <div className="mt-4 grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
            {FEATURES.map((f) => (
              <div key={f.title} className="rounded-xl border border-line p-3">
                <div className="flex items-center gap-2 text-[13.5px] font-semibold text-fg">
                  <f.icon size={15} className="text-accent" /> {f.title}
                </div>
                <p className="mt-1 text-[12.5px] leading-relaxed text-dim">{f.text}</p>
              </div>
            ))}
          </div>
        </Card>
        {media && (
          <Card className="p-5">
            <div className="flex items-center justify-between gap-4">
              <div>
                <div className="text-[14px] font-semibold text-fg">Lyrics in Minecraft</div>
                <div className="text-[12.5px] text-dim">Share the playing song's synced lyrics with Vesper on this PC (127.0.0.1 only). Turn on World lyrics in Vesper too.</div>
              </div>
              <Switch label="Lyrics in Minecraft" checked={media.gameLyrics} onChange={(gameLyrics) => void update({ media: { gameLyrics } })} />
            </div>
          </Card>
        )}
      </div>
    </Page>
  );
}
