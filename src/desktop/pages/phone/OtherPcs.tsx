// Your other PCs: pair this OmniHub with OmniHub on a second PC (its PIN
// from "Pair a phone" there), then see how it's doing and lock, sleep,
// restart or shut it down, or control its music. Its certificate is
// remembered when pairing, so later connections can't be intercepted.

import type { PeerPc } from '@shared/types';
import { Lock, Monitor, Moon, Pause, Power, RotateCcw, SkipForward, Trash2 } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Card, CardHeader } from '../../components/ui/Card';
import { TextInput } from '../../components/ui/Form';
import { confirm } from '../../state/dialogs';
import { toast } from '../../state/toasts';

interface PeerStatus {
  hostname?: string;
  cpu?: number;
  memory?: { used: number; total: number };
  uptime?: number;
}

function PeerRow({ pc, onForget }: { pc: PeerPc; onForget: () => void }) {
  const [st, setSt] = useState<PeerStatus | null>(null);
  const [err, setErr] = useState<string | null>(null);
  useEffect(() => {
    void api.peers.status(pc.id).then(
      (s) => setSt(s as PeerStatus),
      (e: unknown) => setErr(errorText(e)),
    );
  }, [pc.id]);
  const act = async (action: string, label: string, ask = false) => {
    if (ask && !(await confirm({ title: `${label} ${pc.name}?`, confirmLabel: label, tone: 'danger' }))) return;
    try {
      await api.peers.action(pc.id, action);
      toast.success(`${pc.name}: ${label.toLowerCase()}`);
    } catch (e) {
      toast.error(`Couldn’t reach ${pc.name}`, errorText(e));
    }
  };
  const mem = st?.memory && st.memory.total ? Math.round((st.memory.used / st.memory.total) * 100) : null;
  return (
    <div className="rounded-xl border border-line p-3">
      <div className="flex items-center gap-2">
        <Monitor size={15} className="text-accent" />
        <span className="text-[13.5px] font-semibold text-fg">{pc.name}</span>
        <span className="truncate text-[11.5px] text-faint">{pc.url}</span>
        <IconButton icon={Trash2} label={`Forget ${pc.name}`} size="sm" className="ml-auto" onClick={onForget} />
      </div>
      <div className="mt-1 text-[12px] text-dim">
        {err ? <span className="text-warn">{err}</span> : st ? `Processor ${Math.round(st.cpu ?? 0)}% · memory ${mem ?? '—'}% · on for ${Math.round((st.uptime ?? 0) / 3600)} h` : 'Reaching it…'}
      </div>
      <div className="mt-2 flex flex-wrap gap-1.5">
        <Button size="sm" variant="ghost" icon={Lock} onClick={() => void act('lock', 'Lock')}>
          Lock
        </Button>
        <Button size="sm" variant="ghost" icon={Moon} onClick={() => void act('sleep', 'Sleep', true)}>
          Sleep
        </Button>
        <Button size="sm" variant="ghost" icon={RotateCcw} onClick={() => void act('restart', 'Restart', true)}>
          Restart
        </Button>
        <Button size="sm" variant="ghost" icon={Power} onClick={() => void act('shutdown', 'Shut down', true)}>
          Shut down
        </Button>
        <Button size="sm" variant="ghost" icon={Pause} onClick={() => void act('toggle', 'Play/pause')}>
          Music
        </Button>
        <IconButton icon={SkipForward} label="Next song" size="sm" onClick={() => void act('next', 'Next song')} />
      </div>
    </div>
  );
}

export function OtherPcs() {
  const [pcs, setPcs] = useState<PeerPc[]>([]);
  const [url, setUrl] = useState('');
  const [pin, setPin] = useState('');
  const [busy, setBusy] = useState(false);
  useEffect(() => void api.peers.list().then(setPcs, () => undefined), []);
  const pair = async () => {
    setBusy(true);
    try {
      const pc = await api.peers.pair(url, pin);
      setPcs((l) => [...l.filter((x) => x.id !== pc.id), pc]);
      setUrl('');
      setPin('');
      toast.success(`Paired with ${pc.name}`, pc.fingerprint ? `Its certificate: ${pc.fingerprint.slice(0, 23)}… (compare with the other PC’s pairing window)` : undefined);
    } catch (e) {
      toast.error('Couldn’t pair', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Card className="p-5">
      <CardHeader icon={Monitor} title="Your other PCs" subtitle="Control a second PC that runs OmniHub: on it, open Phone → Pair a phone, then type its address and PIN here. It shows up there as a paired device, with the same permissions." />
      <div className="mt-3 flex flex-wrap gap-2">
        <TextInput value={url} onChange={(e) => setUrl(e.target.value)} placeholder="Address, like 192.168.1.20:7878" className="min-w-[200px] flex-1" />
        <TextInput value={pin} onChange={(e) => setPin(e.target.value)} placeholder="PIN" className="w-28" />
        <Button variant="primary" loading={busy} disabled={!url.trim() || !pin.trim()} onClick={() => void pair()}>
          Pair
        </Button>
      </div>
      {pcs.length > 0 && (
        <div className="mt-3 space-y-2">
          {pcs.map((pc) => (
            <PeerRow key={pc.id} pc={pc} onForget={() => void api.peers.forget(pc.id).then(setPcs)} />
          ))}
        </div>
      )}
    </Card>
  );
}
