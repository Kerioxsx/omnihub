// What the PC is built from (detected) and what OmniHub can't see (the
// case), which shapes the 3D view.

import type { CoolerKind, RigSettings } from '@shared/types';
import { Cpu, HardDrive, MemoryStick, Monitor, RefreshCw } from 'lucide-react';
import type { ReactNode } from 'react';
import { Button } from '../../components/ui/Button';
import { Card, CardHeader } from '../../components/ui/Card';
import { Segmented, Select, Switch } from '../../components/ui/Form';
import type { PartId } from '../../lib/rig/scene3d';
import { Row } from '../games/shared';
import type { Rig } from './useRig';

const SIZE: Record<string, string> = { eAtx: 'E-ATX', atx: 'ATX', microAtx: 'Micro-ATX', miniItx: 'Mini-ITX' };

function Spec({ label, children, part, onHover }: { label: string; children: ReactNode; part?: PartId; onHover: (p: PartId | null) => void }) {
  return (
    <div className="flex items-baseline justify-between gap-4 px-5 py-2.5 hover:bg-surface-2/60" onMouseEnter={() => part && onHover(part)} onMouseLeave={() => onHover(null)}>
      <span className="shrink-0 text-[12.5px] text-faint">{label}</span>
      <span className="min-w-0 text-right text-[13px] text-fg">{children}</span>
    </div>
  );
}

function Count({ label, value, max, onChange }: { label: string; value: number; max: number; onChange: (n: number) => void }) {
  return (
    <Row title={label}>
      <Segmented label={label} size="sm" value={String(value)} onChange={(v) => onChange(Number(v))} options={Array.from({ length: max + 1 }, (_, i) => ({ value: String(i), label: String(i) }))} />
    </Row>
  );
}

export function BuildTab({ rig, onHover }: { rig: Rig; onHover: (p: PartId | null) => void }) {
  const sp = rig.specs;
  const m = rig.settings?.model;
  if (!m) return null;
  const set = (patch: Partial<RigSettings['model']>) => void rig.update({ model: patch });
  const ram = sp?.ram ?? [];
  const sticks = ram.length ? `${ram.length} × ${ram[0].sizeGb} GB ${ram[0].kind} ${ram[0].speed ? `${ram[0].speed} MT/s` : ''}`.trim() : sp ? `${sp.ramGb} GB` : '…';
  return (
    <div className="space-y-4">
      <Card>
        <CardHeader icon={Cpu} title="Detected" subtitle="Read from Windows: the firmware's tables, the registry and the connected devices. Nothing is installed or sent anywhere." className="px-5 pt-4" actions={<Button size="sm" variant="secondary" icon={RefreshCw} onClick={() => void rig.redetect()}>Detect again</Button>} />
        <div className="divide-y divide-line pb-1">
          <Spec label="Processor" part="cpu" onHover={onHover}>
            {sp?.cpu || '…'}
            {sp?.cores ? <span className="text-faint"> · {sp.cores} cores, {sp.threads} threads</span> : null}
          </Spec>
          <Spec label="Graphics" part="gpu" onHover={onHover}>
            {sp?.gpus.join(', ') || '…'}
          </Spec>
          <Spec label="Memory" part="ram" onHover={onHover}>
            <span className="inline-flex items-center gap-1.5">
              <MemoryStick size={13} className="text-faint" /> {sticks}
              {ram[0]?.maker ? <span className="text-faint">· {ram[0].maker}</span> : null}
            </span>
          </Spec>
          <Spec label="Motherboard" part="board" onHover={onHover}>
            {sp?.board ? `${sp.board}${sp.boardMaker ? ` (${sp.boardMaker.replace(/ COMPUTER INC\.?|,? Ltd\.?| Co\.?/gi, '')})` : ''} · ${SIZE[sp.boardSize]}` : '…'}
          </Spec>
          <Spec label="Cooler" part="cooler" onHover={onHover}>
            {sp?.coolerName || (sp ? 'Not recognised (air cooler, or one without USB)' : '…')}
          </Spec>
          {sp?.devices
            .filter((d) => d.role !== 'cooler')
            .map((d) => (
              <Spec key={`${d.vendorId}:${d.productId}`} label={d.role === 'hub' ? 'Hub' : 'Device'} part="fans" onHover={onHover}>
                {d.name}
              </Spec>
            ))}
          {sp?.disks.map((d) => (
            <Spec key={d.name} label="Storage" onHover={onHover}>
              <span className="inline-flex items-center gap-1.5">
                <HardDrive size={13} className="text-faint" /> {d.name}
                {d.nvme && <span className="text-faint">· NVMe</span>}
              </span>
            </Spec>
          ))}
          {sp?.displays.map((d) => (
            <Spec key={d.name} label="Display" onHover={onHover}>
              <span className="inline-flex items-center gap-1.5">
                <Monitor size={13} className="text-faint" /> {d.name} · {d.width}×{d.height} at {d.hz} Hz
              </span>
            </Spec>
          ))}
        </div>
      </Card>

      <Card>
        <CardHeader title="Your case" subtitle="What OmniHub can't see from Windows; the 3D view follows it." className="px-5 pt-4" />
        <div className="divide-y divide-line">
          <Row title="Colour">
            <Segmented label="Case colour" size="sm" value={m.caseColor} onChange={(v) => set({ caseColor: v })} options={[{ value: 'black', label: 'Black' }, { value: 'white', label: 'White' }]} />
          </Row>
          <Row title="Glass side panel">
            <Switch checked={m.glass} onChange={(v) => set({ glass: v })} label="Glass side panel" />
          </Row>
          <Row title="Cooler">
            <Select value={m.cooler ?? 'detected'} onChange={(e) => set({ cooler: e.target.value === 'detected' ? null : (e.target.value as CoolerKind) })} className="w-48">
              <option value="detected">As detected</option>
              <option value="aio">Liquid (AIO)</option>
              <option value="air">Air tower</option>
            </Select>
          </Row>
          {(m.cooler ?? (sp?.cooler === 'aio' ? 'aio' : 'air')) === 'aio' && (
            <Row title="Radiator">
              <Segmented label="Radiator size" size="sm" value={String(m.radiatorMm ?? sp?.radiatorMm ?? 360)} onChange={(v) => set({ radiatorMm: Number(v) })} options={['120', '240', '280', '360'].map((v) => ({ value: v, label: `${v} mm` }))} />
            </Row>
          )}
          <Count label="Front fans" value={m.frontFans} max={4} onChange={(n) => set({ frontFans: n })} />
          <Count label="Top fans" value={m.topFans} max={3} onChange={(n) => set({ topFans: n })} />
          <Count label="Rear fan" value={m.rearFans} max={1} onChange={(n) => set({ rearFans: n })} />
          <Count label="Bottom fans" value={m.bottomFans} max={3} onChange={(n) => set({ bottomFans: n })} />
          <Row title="RGB memory">
            <Switch checked={m.ramRgb} onChange={(v) => set({ ramRgb: v })} label="RGB memory" />
          </Row>
          <Row title="Graphics card mounted upright">
            <Switch checked={m.verticalGpu} onChange={(v) => set({ verticalGpu: v })} label="Graphics card mounted upright" />
          </Row>
          <Row title="Screen inside the case">
            <Switch checked={m.caseScreen} onChange={(v) => set({ caseScreen: v })} label="Screen inside the case" />
          </Row>
        </div>
      </Card>
    </div>
  );
}
