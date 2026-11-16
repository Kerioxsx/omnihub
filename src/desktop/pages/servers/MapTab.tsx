// The live map: the world drawn from above (OmniHub draws it from the
// server's own files as it saves them), with players moving on it in real
// time. Drag to move, scroll to zoom, follow a player.

import type { SrvMap, SrvOnline, SrvView } from '@shared/types';
import { Crosshair, Home, Locate, Minus, Plus, RefreshCw } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { api } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Card } from '../../components/ui/Card';
import { Segmented } from '../../components/ui/Form';
import { Callout, EmptyState } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { useEvent, useInterval, useSize } from '../../lib/hooks';
import { useRoute } from '../../lib/router';
import { DIMENSIONS, Head } from './shared';

type Dim = 'overworld' | 'the_nether' | 'the_end';

function hue(name: string): number {
  let h = 0;
  for (const c of name) h = (h * 31 + c.charCodeAt(0)) % 360;
  return h;
}

export function MapTab({ s }: { s: SrvView }) {
  const route = useRoute();
  const [dim, setDim] = useState<Dim>('overworld');
  const [info, setInfo] = useState<SrvMap | null>(null);
  const [players, setPlayers] = useState<SrvOnline[]>([]);
  const [view, setView] = useState({ x: 0, z: 0, scale: 1 });
  const [follow, setFollow] = useState<string | null>(route.params.get('player'));
  const [hover, setHover] = useState<[number, number] | null>(null);
  const [, redraw] = useState(0);
  const tiles = useRef(new Map<string, HTMLImageElement | 'loading' | 'missing'>());
  const canvas = useRef<HTMLCanvasElement>(null);
  const [wrap, size] = useSize<HTMLDivElement>();
  const drag = useRef<{ x: number; y: number; vx: number; vz: number } | null>(null);
  const centered = useRef(false);

  const load = useCallback(() => {
    void api.servers.map(s.id, dim).then((m) => {
      setInfo(m);
      setPlayers(m.players);
      if (!centered.current) {
        centered.current = true;
        const p = m.players.find((o) => o.name === follow)?.pos ?? m.players[0]?.pos;
        const c = p ? [p[0], p[2]] : m.spawn ? [m.spawn[0], m.spawn[2]] : m.regions[0] ? [m.regions[0][0] * 512 + 256, m.regions[0][1] * 512 + 256] : [0, 0];
        setView((v) => ({ ...v, x: c[0], z: c[1] }));
      }
    }, () => setInfo({ regions: [], spawn: null, players: [], supported: true }));
  }, [s.id, dim, follow]);
  useEffect(() => {
    tiles.current.clear();
    centered.current = false;
    load();
  }, [s.id, dim]); // eslint-disable-line react-hooks/exhaustive-deps
  // Players every 2 seconds; the world's pictures every 2 minutes.
  useInterval(() => void api.servers.watch(s.id).then(setPlayers), s.state === 'running' ? 2000 : null);
  useEvent<{ id: string; online: SrvOnline[] }>('mcserver:players', (e) => e.id === s.id && setPlayers(e.online));
  useInterval(() => {
    tiles.current.clear();
    load();
  }, 120_000);

  useEffect(() => {
    if (!follow) return;
    const p = players.find((o) => o.name === follow)?.pos;
    if (p) setView((v) => ({ ...v, x: p[0], z: p[2] }));
  }, [players, follow]);

  const tile = (x: number, z: number) => {
    const k = `${dim}:${x}:${z}`;
    const t = tiles.current.get(k);
    if (t) return t;
    tiles.current.set(k, 'loading');
    void api.servers.mapTile(s.id, dim, x, z).then(
      (url) => {
        const img = new Image();
        img.onload = () => {
          tiles.current.set(k, img);
          redraw((n) => n + 1);
        };
        img.src = url;
      },
      () => tiles.current.set(k, 'missing'),
    );
    return 'loading';
  };

  const here = players.filter((o) => (o.dimension ?? 'minecraft:overworld').replace('minecraft:', '') === dim);

  // Draw.
  useEffect(() => {
    const c = canvas.current;
    if (!c || !info || !size.width) return;
    const dpr = window.devicePixelRatio || 1;
    c.width = size.width * dpr;
    c.height = size.height * dpr;
    const g = c.getContext('2d')!;
    g.setTransform(dpr, 0, 0, dpr, 0, 0);
    g.fillStyle = '#15171c';
    g.fillRect(0, 0, size.width, size.height);
    g.imageSmoothingEnabled = view.scale < 1;
    const sx = (bx: number) => (bx - view.x) * view.scale + size.width / 2;
    const sz = (bz: number) => (bz - view.z) * view.scale + size.height / 2;
    for (const [rx, rz] of info.regions) {
      const x0 = sx(rx * 512);
      const z0 = sz(rz * 512);
      const w = 512 * view.scale;
      if (x0 > size.width || z0 > size.height || x0 + w < 0 || z0 + w < 0) continue;
      const t = tile(rx, rz);
      if (t instanceof HTMLImageElement) g.drawImage(t, x0, z0, w, w);
      else if (t === 'loading') {
        g.fillStyle = 'rgba(255,255,255,.03)';
        g.fillRect(x0, z0, w, w);
      }
    }
    // Chunk grid when zoomed in.
    if (view.scale >= 4) {
      g.strokeStyle = 'rgba(0,0,0,.18)';
      g.lineWidth = 1;
      const step = 16 * view.scale;
      const ox = sx(Math.floor((view.x - size.width / 2 / view.scale) / 16) * 16);
      const oz = sz(Math.floor((view.z - size.height / 2 / view.scale) / 16) * 16);
      g.beginPath();
      for (let x = ox; x < size.width; x += step) (g.moveTo(x, 0), g.lineTo(x, size.height));
      for (let z = oz; z < size.height; z += step) (g.moveTo(0, z), g.lineTo(size.width, z));
      g.stroke();
    }
    if (info.spawn && dim === 'overworld') {
      const x = sx(info.spawn[0]);
      const z = sz(info.spawn[2]);
      g.fillStyle = '#ffd34d';
      g.strokeStyle = '#1b1b1b';
      g.lineWidth = 2;
      g.beginPath();
      g.moveTo(x, z - 9);
      g.lineTo(x + 8, z - 2);
      g.lineTo(x + 5, z - 2);
      g.lineTo(x + 5, z + 7);
      g.lineTo(x - 5, z + 7);
      g.lineTo(x - 5, z - 2);
      g.lineTo(x - 8, z - 2);
      g.closePath();
      g.stroke();
      g.fill();
    }
    g.font = '600 12px Inter, system-ui, sans-serif';
    g.textAlign = 'center';
    for (const o of here) {
      if (!o.pos) continue;
      const x = sx(o.pos[0]);
      const z = sz(o.pos[2]);
      const hh = hue(o.name);
      g.fillStyle = `hsl(${hh} 70% 55%)`;
      g.strokeStyle = '#fff';
      g.lineWidth = 2.5;
      g.beginPath();
      g.roundRect(x - 9, z - 9, 18, 18, 4);
      g.fill();
      g.stroke();
      g.fillStyle = '#fff';
      g.fillText(o.name[0]?.toUpperCase() ?? '?', x, z + 4);
      const w = g.measureText(o.name).width + 12;
      g.fillStyle = 'rgba(10,12,16,.82)';
      g.beginPath();
      g.roundRect(x - w / 2, z - 32, w, 19, 6);
      g.fill();
      g.fillStyle = '#fff';
      g.fillText(o.name, x, z - 18);
    }
  });

  const zoom = (f: number) => setView((v) => ({ ...v, scale: Math.max(0.125, Math.min(16, v.scale * f)) }));

  if (!info) return <Card className="h-[60vh]" />;
  return (
    <div className="grid gap-4 lg:grid-cols-[1fr_240px]">
      <Card className="relative overflow-hidden">
        <div className="absolute left-3 top-3 z-10 flex gap-2 rounded-[13px] bg-surface/95 shadow-lg backdrop-blur">
          <Segmented size="sm" label="World" value={dim} onChange={(d) => setDim(d)} options={DIMENSIONS.map((d) => ({ value: d.id as Dim, label: d.label }))} />
        </div>
        <div className="absolute right-3 top-3 z-10 flex gap-1 rounded-xl bg-surface/95 p-1 shadow-lg backdrop-blur">
          <IconButton icon={Plus} label="Zoom in" onClick={() => zoom(2)} />
          <IconButton icon={Minus} label="Zoom out" onClick={() => zoom(0.5)} />
          {info.spawn && dim === 'overworld' && <IconButton icon={Home} label="Spawn" onClick={() => (setFollow(null), setView((v) => ({ ...v, x: info.spawn![0], z: info.spawn![2] })))} />}
          <IconButton icon={RefreshCw} label="Draw again" onClick={() => (tiles.current.clear(), load())} />
        </div>
        <div
          ref={wrap}
          className={cx('h-[calc(100vh-250px)] min-h-[420px] touch-none', drag.current ? 'cursor-grabbing' : 'cursor-grab')}
          onPointerDown={(e) => {
            (e.target as HTMLElement).setPointerCapture(e.pointerId);
            drag.current = { x: e.clientX, y: e.clientY, vx: view.x, vz: view.z };
            setFollow(null);
          }}
          onPointerMove={(e) => {
            const r = e.currentTarget.getBoundingClientRect();
            setHover([Math.floor(view.x + (e.clientX - r.left - r.width / 2) / view.scale), Math.floor(view.z + (e.clientY - r.top - r.height / 2) / view.scale)]);
            const d = drag.current;
            if (d) setView((v) => ({ ...v, x: d.vx - (e.clientX - d.x) / v.scale, z: d.vz - (e.clientY - d.y) / v.scale }));
          }}
          onPointerUp={() => (drag.current = null)}
          onPointerLeave={() => setHover(null)}
          onWheel={(e) => zoom(e.deltaY < 0 ? 1.25 : 0.8)}
        >
          <canvas ref={canvas} className="h-full w-full" style={{ width: size.width, height: size.height }} />
        </div>
        {hover && (
          <div className="pointer-events-none absolute bottom-3 left-3 rounded-lg bg-black/70 px-2.5 py-1 font-mono text-[11.5px] text-white">
            {hover[0]}, {hover[1]}
          </div>
        )}
        {info.regions.length === 0 && (
          <div className="absolute inset-0 grid place-items-center">
            <EmptyState icon={Crosshair} title={info.supported ? 'Nothing explored here yet' : 'The map needs Minecraft 1.18 or newer'} description={info.supported ? 'The map fills in as players explore and the server saves (every few minutes).' : 'Players’ positions still show in Players.'} />
          </div>
        )}
      </Card>
      <div className="space-y-3">
        <Card className="divide-y divide-line">
          <div className="px-4 py-3 text-[12.5px] font-medium text-fg">Here now</div>
          {here.length === 0 && <div className="px-4 py-3 text-[12.5px] text-dim">{s.state === 'running' ? 'Nobody in this world.' : 'The server is off.'}</div>}
          {here.map((o) => (
            <button key={o.name} type="button" onClick={() => setFollow(follow === o.name ? null : o.name)} className={cx('flex w-full items-center gap-2.5 px-4 py-2.5 text-left hover:bg-surface-2', follow === o.name && 'bg-accent-soft')}>
              <Head name={o.name} size={24} />
              <div className="min-w-0 flex-1">
                <div className="truncate text-[13px] text-fg">{o.name}</div>
                <div className="font-mono text-[11px] text-faint">{o.pos ? o.pos.map((v) => Math.round(v)).join(', ') : '…'}</div>
              </div>
              <Locate size={14} className={follow === o.name ? 'text-accent' : 'text-faint'} />
            </button>
          ))}
        </Card>
        <Callout tone="info">OmniHub draws the map itself from the world’s files, so it works on every server type. New areas appear after the server saves them.</Callout>
        {players.some((o) => (o.dimension ?? '').replace('minecraft:', '') !== dim && o.pos) && (
          <Button size="sm" variant="ghost" onClick={() => setDim(((players[0].dimension ?? 'minecraft:overworld').replace('minecraft:', '') as Dim) || 'overworld')}>
            Others are in another world
          </Button>
        )}
      </div>
    </div>
  );
}
