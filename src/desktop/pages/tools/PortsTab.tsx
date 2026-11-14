// "What is using port 3000?": every port in use with its program, and End.

import type { LanLink, PortUse } from '@shared/types';
import { Network, RefreshCw, Smartphone, X } from 'lucide-react';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Card, CardHeader, Skeleton } from '../../components/ui/Card';
import { SearchInput, Segmented } from '../../components/ui/Form';
import { EmptyState } from '../../components/ui/States';
import { confirm } from '../../state/dialogs';
import { toast } from '../../state/toasts';

type Show = 'listening' | 'all';

function matches(p: PortUse, q: string) {
  const s = q.trim().replace(/^:/, '').toLowerCase();
  if (!s) return true;
  if (/^\d+$/.test(s)) return p.port === Number(s) || p.remotePort === Number(s) || p.pid === Number(s);
  return p.process.toLowerCase().includes(s) || p.local.includes(s) || (p.remote ?? '').includes(s);
}

/** A dev server on this PC, opened on a phone on the same Wi-Fi. */
function PhoneQr({ p, onClose }: { p: PortUse; onClose: () => void }) {
  const [link, setLink] = useState<LanLink | null>(null);
  useEffect(() => void api.dev.lanLink(p.port, p.local).then(setLink, (e: unknown) => toast.error('Couldn’t make the link', errorText(e))), [p.port, p.local]);
  return (
    <div className="mt-3 flex items-start gap-4 rounded-xl border border-line bg-surface p-3">
      {link?.qrSvgs[0] && <div className="h-36 w-36 shrink-0 rounded-lg bg-white p-1.5 [&_svg]:h-full [&_svg]:w-full" dangerouslySetInnerHTML={{ __html: link.qrSvgs[0] }} />}
      <div className="min-w-0 flex-1 text-[13px]">
        <div className="font-semibold text-fg">
          {p.process} on port {p.port}, on your phone
        </div>
        {link?.note ? <p className="mt-1 text-warn">{link.note}</p> : <p className="mt-1 text-dim">Scan with a phone on the same Wi-Fi. If it doesn’t load, allow the program through Windows Firewall.</p>}
        {link?.urls.map((u) => (
          <div key={u} className="mt-1 font-mono text-[12px] text-dim">
            {u}
          </div>
        ))}
      </div>
      <IconButton size="sm" icon={X} label="Close" onClick={onClose} />
    </div>
  );
}

export function PortsTab() {
  const [qr, setQr] = useState<PortUse | null>(null);
  const [list, setList] = useState<PortUse[] | null>(null);
  const [q, setQ] = useState('');
  const [show, setShow] = useState<Show>('listening');
  const load = useCallback(() => void api.ports.list().then(setList, (e: unknown) => toast.error('Couldn’t read the ports', errorText(e))), []);
  useEffect(load, [load]);

  const shown = useMemo(() => (list ?? []).filter((p) => (show === 'all' || p.proto === 'UDP' || p.state === 'Listening') && matches(p, q)), [list, q, show]);

  const end = async (p: PortUse) => {
    const ok = await confirm({ title: `End ${p.process}?`, description: `It is using port ${p.port}. Anything unsaved in it is lost.`, confirmLabel: 'End it', tone: 'danger' });
    if (!ok) return;
    try {
      await api.ports.end(p.pid);
      toast.success(`Ended ${p.process}`);
      load();
    } catch (e) {
      toast.error('Couldn’t end it', errorText(e));
    }
  };

  return (
    <Card className="flex min-h-0 flex-1 flex-col p-5">
      <CardHeader
        icon={Network}
        title="Ports"
        subtitle="Which program has which port: type a port number like 3000, or a program’s name."
        actions={<IconButton icon={RefreshCw} label="Refresh" onClick={load} />}
      />
      <div className="mt-4 flex flex-wrap items-center gap-2">
        <SearchInput value={q} onChange={setQ} placeholder="Port, program or address" className="w-72" />
        <Segmented label="Show" size="sm" value={show} onChange={setShow} options={[{ value: 'listening', label: 'Open ports' }, { value: 'all', label: 'Every connection' }]} />
        {list && <span className="text-[12px] text-faint">{shown.length} of {list.length}</span>}
      </div>
      {qr && <PhoneQr key={`${qr.local}:${qr.port}`} p={qr} onClose={() => setQr(null)} />}
      <div className="mt-3 min-h-0 flex-1 overflow-auto rounded-xl border border-line">
        {!list ? (
          <Skeleton className="h-60" />
        ) : !shown.length ? (
          <EmptyState compact icon={Network} title={q ? `Nothing uses ${q}` : 'No ports in use'} description={q && /^\d+$/.test(q.trim()) ? 'That port is free.' : undefined} />
        ) : (
          <table className="w-full text-[12.5px]">
            <thead className="sticky top-0 bg-surface text-left text-faint">
              <tr>
                <th className="px-3 py-2 font-medium">Port</th>
                <th className="px-3 py-2 font-medium">Program</th>
                <th className="px-3 py-2 font-medium">Address</th>
                <th className="px-3 py-2 font-medium">State</th>
                <th className="w-16" />
              </tr>
            </thead>
            <tbody>
              {shown.map((p, i) => (
                <tr key={`${p.proto}-${p.local}-${p.port}-${p.remote}-${p.remotePort}-${i}`} className="border-t border-line hover:bg-surface-2">
                  <td className="px-3 py-1.5 font-mono text-fg">
                    {p.port} <span className="text-faint">{p.proto}</span>
                  </td>
                  <td className="px-3 py-1.5">
                    <span className="text-fg">{p.process}</span> <span className="text-faint">· {p.pid}</span>
                  </td>
                  <td className="px-3 py-1.5 font-mono text-dim">
                    {p.local}
                    {p.remote && <span className="text-faint"> → {p.remote}:{p.remotePort}</span>}
                  </td>
                  <td className="px-3 py-1.5">{p.state ? <Badge tone={p.state === 'Listening' ? 'accent' : 'neutral'}>{p.state}</Badge> : <span className="text-faint">—</span>}</td>
                  <td className="whitespace-nowrap px-2 py-1">
                    {p.proto === 'TCP' && p.state === 'Listening' && p.pid > 4 && <IconButton size="sm" icon={Smartphone} label={`Open port ${p.port} on your phone`} onClick={() => setQr(p)} />}
                    {p.pid > 4 && <IconButton size="sm" icon={X} label={`End ${p.process}`} onClick={() => void end(p)} />}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
      {q && /^\d+$/.test(q.trim()) && list && !shown.length && (
        <Button size="sm" variant="ghost" className="mt-2 self-start" onClick={() => setShow('all')}>
          Look in every connection too
        </Button>
      )}
    </Card>
  );
}
