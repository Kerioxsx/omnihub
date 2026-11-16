// Approve on your phone: the PC asks before something sensitive (showing a
// vault password, restarting as admin, shredding…) and waits for a tap here.

import type { ApprovalRequest } from '@shared/types';
import { ShieldCheck } from 'lucide-react';
import { useEffect, useState } from 'react';
import { client } from '../client';
import { useEvent } from '../lib/events';
import { errorMessage, usePageVisible, vibrate } from '../lib/util';
import { toast } from '../state';
import { Button } from '../ui/common';
import { Sheet } from '../ui/Sheet';

export function ApprovalSheet() {
  const [list, setList] = useState<ApprovalRequest[]>([]);
  const [busy, setBusy] = useState(false);
  const [now, setNow] = useState(() => Date.now() / 1000);
  const visible = usePageVisible();

  // Ones that came in while the app wasn't open.
  useEffect(() => {
    if (visible) void client.approvals().then(setList, () => undefined);
  }, [visible]);
  useEvent<ApprovalRequest>('approval:request', (r) => {
    if (!r?.id) return;
    vibrate([60, 40, 60]);
    setList((l) => [...l.filter((x) => x.id !== r.id), r]);
  });
  useEvent<{ id: string }>('approval:done', (d) => setList((l) => l.filter((x) => x.id !== d?.id)));
  useEffect(() => {
    if (!list.length) return;
    const t = setInterval(() => setNow(Date.now() / 1000), 1000);
    return () => clearInterval(t);
  }, [list.length]);

  const open = list.filter((r) => r.expiresAt > now);
  const req = open[0];
  const answer = async (approve: boolean) => {
    if (!req) return;
    setBusy(true);
    try {
      await client.answerApproval(req.id, approve);
      toast.success(approve ? 'Allowed' : 'Declined', req.title);
    } catch (e) {
      toast.error('Too late', errorMessage(e));
    } finally {
      setList((l) => l.filter((x) => x.id !== req.id));
      setBusy(false);
    }
  };

  return (
    <Sheet open={!!req} onClose={() => void answer(false)} title="Your PC is asking" subtitle={req ? `${Math.max(0, Math.round(req.expiresAt - now))} s left` : undefined}>
      {req && (
        <div className="space-y-4 pb-2">
          <div className="flex items-start gap-3 rounded-2xl bg-surface-2 p-4">
            <ShieldCheck size={26} className="mt-0.5 shrink-0 text-accent" />
            <div className="min-w-0">
              <div className="text-[16px] font-semibold">{req.title}</div>
              {req.detail && <div className="mt-0.5 break-words text-[14px] text-dim">{req.detail}</div>}
              <div className="mt-2 text-[12.5px] text-faint">Only allow it if you’re at your PC doing this right now.</div>
            </div>
          </div>
          <div className="flex gap-2.5">
            <Button variant="ghost" className="flex-1" disabled={busy} onClick={() => void answer(false)}>
              Decline
            </Button>
            <Button className="flex-1" loading={busy} onClick={() => void answer(true)}>
              Allow
            </Button>
          </div>
        </div>
      )}
    </Sheet>
  );
}
