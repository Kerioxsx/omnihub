// PC checks: each drive's own health report, a temperature stress test,
// a Wi-Fi QR code for guests, and pinning any window on top.

import { formatBytes } from '@shared/format';
import type { DriveHealth, PinWindow, StressResult, WifiQr } from '@shared/types';
import { Flame, HardDrive, Pin, PinOff, QrCode, RefreshCw, Square } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Badge, Card, CardHeader } from '../../components/ui/Card';
import { Segmented } from '../../components/ui/Form';
import { ProgressBar } from '../../components/ui/Progress';
import { Callout } from '../../components/ui/States';
import { useEvent } from '../../lib/hooks';
import { toast } from '../../state/toasts';

function Drives() {
  const [list, setList] = useState<DriveHealth[] | null>(null);
  const [busy, setBusy] = useState(false);
  const load = async () => {
    setBusy(true);
    try {
      setList(await api.pcCheck.drives());
    } catch (e) {
      toast.error('Couldn’t read the drives', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  useEffect(() => void load(), []);
  return (
    <Card className="p-5">
      <CardHeader icon={HardDrive} title="Drive health" subtitle="Each drive’s own report (SMART): health, temperature, wear and errors." actions={<Button size="sm" variant="ghost" icon={RefreshCw} loading={busy} onClick={() => void load()}>Check</Button>} />
      <div className="mt-3 space-y-2">
        {list?.map((d) => (
          <div key={d.name} className="rounded-xl border border-line px-3.5 py-3">
            <div className="flex flex-wrap items-center gap-2">
              <span className="text-[14px] font-semibold text-fg">{d.name}</span>
              <Badge tone={d.health === 'Healthy' ? 'good' : d.health === 'Unknown' ? 'neutral' : 'bad'}>{d.health}</Badge>
              <span className="text-[12px] text-faint">
                {[d.media, d.bus, d.size ? formatBytes(d.size) : ''].filter(Boolean).join(' · ')}
              </span>
            </div>
            <div className="mt-1 flex flex-wrap gap-x-4 text-[12.5px] text-dim">
              {d.temperature != null && <span>{d.temperature} °C</span>}
              {d.wear != null && <span>{d.wear}% worn</span>}
              {d.powerOnHours != null && <span>{d.powerOnHours.toLocaleString()} hours on</span>}
              {(d.readErrors ?? 0) + (d.writeErrors ?? 0) > 0 && <span className="text-warn">{(d.readErrors ?? 0) + (d.writeErrors ?? 0)} errors</span>}
            </div>
            <div className="mt-1 text-[12.5px] text-fg">{d.verdict}</div>
          </div>
        ))}
      </div>
      <p className="mt-3 text-[11.5px] text-faint">Some figures need OmniHub to run as administrator, and USB drives often don’t report them.</p>
    </Card>
  );
}

function Stress() {
  const [minutes, setMinutes] = useState('3');
  const [run, setRun] = useState<{ seconds: number; temp: number | null } | null>(null);
  const [result, setResult] = useState<StressResult | null>(null);
  useEvent<{ seconds: number; temp: number | null }>('stress:progress', setRun);
  useEvent<StressResult>('stress:done', (r) => {
    setRun(null);
    setResult(r);
  });
  const total = Number(minutes) * 60;
  return (
    <Card className="p-5">
      <CardHeader icon={Flame} title="Temperature stress test" subtitle="Every processor core at full load for a few minutes, to see how hot it gets and whether it slows itself down." />
      {run ? (
        <div className="mt-4 space-y-2">
          <ProgressBar value={run.seconds / total} />
          <div className="flex items-center justify-between text-[13px]">
            <span className="text-dim">
              {run.seconds} s of {total} s · {run.temp != null ? `${Math.round(run.temp)} °C` : 'temperature not readable'}
            </span>
            <Button size="sm" variant="danger" icon={Square} onClick={() => void api.pcCheck.stopStress()}>
              Stop
            </Button>
          </div>
        </div>
      ) : (
        <div className="mt-4 flex flex-wrap items-center gap-3">
          <Segmented label="Length" value={minutes} onChange={setMinutes} options={[{ value: '1', label: '1 min' }, { value: '3', label: '3 min' }, { value: '10', label: '10 min' }]} />
          <Button
            variant="primary"
            icon={Flame}
            onClick={() => {
              setResult(null);
              setRun({ seconds: 0, temp: null });
              void api.pcCheck.stress(total).catch((e: unknown) => {
                setRun(null);
                toast.error('The test didn’t start', errorText(e));
              });
            }}
          >
            Start the test
          </Button>
          <span className="text-[12px] text-faint">It stops by itself at 100 °C. Close games first.</span>
        </div>
      )}
      {result && (
        <Callout tone={result.throttled || (result.maxTemp ?? 0) >= 95 ? 'warn' : 'good'} className="mt-4">
          {result.verdict}
          {result.maxTemp != null && ` (started at ${Math.round(result.startTemp ?? 0)} °C, peak ${Math.round(result.maxTemp)} °C, after ${result.seconds} s)`}
        </Callout>
      )}
    </Card>
  );
}

function Wifi() {
  const [qr, setQr] = useState<WifiQr | null>(null);
  const [show, setShow] = useState(false);
  const load = async () => {
    try {
      setQr(await api.wifiQr());
    } catch (e) {
      toast.error('No Wi-Fi to share', errorText(e));
    }
  };
  return (
    <Card className="p-5">
      <CardHeader icon={QrCode} title="Wi-Fi for guests" subtitle="A QR code that joins your Wi-Fi when a phone camera scans it. Shown here only." actions={!qr && <Button size="sm" onClick={() => void load()}>Show QR code</Button>} />
      {qr && (
        <div className="mt-4 flex flex-wrap items-center gap-5">
          <div className="h-[200px] w-[200px] rounded-xl bg-white p-2" dangerouslySetInnerHTML={{ __html: qr.svg }} />
          <div className="space-y-1 text-[13.5px]">
            <div>
              <span className="text-faint">Network:</span> <span className="font-semibold text-fg">{qr.ssid}</span>
            </div>
            <div>
              <span className="text-faint">Password:</span>{' '}
              {qr.password ? (
                <button type="button" className="font-mono text-fg underline decoration-dotted" onClick={() => setShow(!show)}>
                  {show ? qr.password : '•••••••• (show)'}
                </button>
              ) : (
                <span className="text-dim">none</span>
              )}
            </div>
            <Button size="sm" variant="ghost" onClick={() => setQr(null)}>
              Hide
            </Button>
          </div>
        </div>
      )}
    </Card>
  );
}

function Pins() {
  const [list, setList] = useState<PinWindow[]>([]);
  const load = () => void api.pin.windows().then(setList, () => undefined);
  useEffect(load, []);
  return (
    <Card className="p-5">
      <CardHeader icon={Pin} title="Pin a window on top" subtitle="Keep a video, a guide or a chat above everything. A deck or wheel button can pin the window in front, too." actions={<Button size="sm" variant="ghost" icon={RefreshCw} onClick={load}>Refresh</Button>} />
      <div className="mt-3 max-h-[300px] divide-y divide-line overflow-y-auto rounded-xl border border-line">
        {list.map((w) => (
          <div key={w.handle} className="flex items-center gap-3 px-3.5 py-2">
            <div className="min-w-0 flex-1">
              <div className="truncate text-[13px] text-fg">{w.title}</div>
              <div className="text-[11.5px] text-faint">{w.app}</div>
            </div>
            <Button
              size="xs"
              variant={w.pinned ? 'primary' : 'ghost'}
              icon={w.pinned ? PinOff : Pin}
              onClick={() =>
                void api.pin.set(w.handle, !w.pinned).then(
                  (on) => setList(list.map((x) => (x.handle === w.handle ? { ...x, pinned: on } : x))),
                  (e: unknown) => toast.error('Couldn’t pin it', errorText(e)),
                )
              }
            >
              {w.pinned ? 'Unpin' : 'Pin'}
            </Button>
          </div>
        ))}
      </div>
    </Card>
  );
}

export function ChecksTab() {
  return (
    <div className="space-y-4">
      <Drives />
      <Stress />
      <Pins />
      <Wifi />
    </div>
  );
}
