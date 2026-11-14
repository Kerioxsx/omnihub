// "Let someone send you files": a QR code a friend scans to send photos or
// files to this PC without pairing. It ends by itself.

import { formatBytes } from '@shared/format';
import type { GuestCreated, GuestLink } from '@shared/types';
import { Copy, QrCode, Square, UserPlus } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Badge, Card, CardHeader } from '../../components/ui/Card';
import { Field, Select, TextInput } from '../../components/ui/Form';
import { Callout } from '../../components/ui/States';
import { useEvent, useInterval } from '../../lib/hooks';
import { copyText } from '../../lib/util';
import { toast } from '../../state/toasts';

const GB = 1 << 30;

export function GuestCard({ running }: { running: boolean }) {
  const [links, setLinks] = useState<GuestLink[]>([]);
  const [shown, setShown] = useState<GuestCreated | null>(null);
  const [label, setLabel] = useState('');
  const [minutes, setMinutes] = useState(15);
  const [size, setSize] = useState(4);
  const [busy, setBusy] = useState(false);
  const load = useCallback(() => void api.guests.list().then(setLinks, () => undefined), []);
  useEffect(load, [load]);
  useEvent('guest:received', load);
  useInterval(load, links.length ? 15_000 : null);

  const create = async () => {
    setBusy(true);
    try {
      const c = await api.guests.create(label, minutes, size * GB);
      setShown(c);
      setLabel('');
      load();
    } catch (e) {
      toast.error('Couldn’t make the link', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  const minsLeft = (l: GuestLink) => Math.max(0, Math.round((l.expiresAt * 1000 - Date.now()) / 60_000));

  return (
    <Card className="p-5">
      <CardHeader icon={UserPlus} title="Let someone send you files" subtitle="A QR code a friend scans to send photos, videos or files to this PC, without pairing. It only receives (they see nothing on your PC) and ends by itself." />
      {!running ? (
        <Callout tone="info" className="mt-4">
          Turn on the phone companion first.
        </Callout>
      ) : (
        <>
          <div className="mt-4 grid gap-3 sm:grid-cols-[1fr_auto_auto_auto] sm:items-end">
            <Field label="Who (optional)">
              <TextInput value={label} onChange={(e) => setLabel(e.target.value)} placeholder="Sam’s phone" maxLength={40} />
            </Field>
            <Field label="Works for">
              <Select value={minutes} onChange={(e) => setMinutes(Number(e.target.value))}>
                <option value={15}>15 minutes</option>
                <option value={60}>1 hour</option>
                <option value={240}>4 hours</option>
              </Select>
            </Field>
            <Field label="Up to">
              <Select value={size} onChange={(e) => setSize(Number(e.target.value))}>
                <option value={1}>1 GB</option>
                <option value={4}>4 GB</option>
                <option value={20}>20 GB</option>
              </Select>
            </Field>
            <Button variant="primary" icon={QrCode} loading={busy} onClick={() => void create()}>
              Show QR code
            </Button>
          </div>
          {shown && (
            <div className="mt-4 flex flex-col items-center gap-3 rounded-2xl border border-line bg-surface p-4 sm:flex-row sm:items-start">
              {/* The QR code is made on this PC (an SVG of the link). */}
              <div className="h-44 w-44 shrink-0 rounded-xl bg-white p-2 [&_svg]:h-full [&_svg]:w-full" dangerouslySetInnerHTML={{ __html: shown.qrSvgs[0] ?? '' }} />
              <div className="min-w-0 flex-1 text-[13px]">
                <div className="font-semibold text-fg">Scan with any phone on this Wi-Fi</div>
                <p className="mt-1 text-dim">The phone may warn about the connection the first time (this PC’s own certificate): continue anyway. Files go to Incoming\Guests.</p>
                {shown.urls.map((u) => (
                  <button key={u} type="button" onClick={() => void copyText(u).then(() => toast.success('Link copied'))} className="mt-2 flex w-full items-center gap-2 truncate rounded-lg bg-surface-2 px-2.5 py-1.5 text-left font-mono text-[11.5px] text-dim hover:text-fg">
                    <Copy size={12} className="shrink-0" /> <span className="truncate">{u}</span>
                  </button>
                ))}
              </div>
            </div>
          )}
          {links.length > 0 && (
            <div className="mt-4 space-y-2">
              {links.map((l) => (
                <div key={l.code} className="flex items-center gap-3 rounded-xl border border-line px-3 py-2 text-[13px]">
                  <Badge tone="accent">{l.label}</Badge>
                  <span className="min-w-0 flex-1 truncate text-dim">
                    {l.files} file{l.files === 1 ? '' : 's'} · {formatBytes(l.usedBytes)} of {formatBytes(l.maxBytes)} · {minsLeft(l)} min left
                  </span>
                  <Button
                    size="sm"
                    variant="ghost"
                    icon={Square}
                    onClick={() =>
                      void api.guests.stop(l.code).then(() => {
                        if (shown?.link.code === l.code) setShown(null);
                        load();
                      })
                    }
                  >
                    End
                  </Button>
                </div>
              ))}
            </div>
          )}
        </>
      )}
    </Card>
  );
}
