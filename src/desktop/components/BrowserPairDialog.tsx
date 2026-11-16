// A browser extension asks to use the vault: show its code and let the user
// allow or deny it. Lives at the app root so it appears on any page.

import type { BrowserPairRequest } from '@shared/types';
import { Globe, ShieldCheck, X } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../api';
import { useEvent } from '../lib/hooks';
import { toast } from '../state/toasts';
import { Button } from './ui/Button';
import { Modal } from './ui/Overlay';

export function BrowserPairDialog() {
  const [req, setReq] = useState<BrowserPairRequest | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    // A request may have arrived before the window was ready.
    void api.browser.status().then((s) => s.pending && setReq(s.pending), () => undefined);
  }, []);
  useEvent<BrowserPairRequest>('browser:pair-request', (r) => setReq(r));
  useEvent<{ id: string; allowed: boolean }>('browser:pair-done', (d) => setReq((r) => (r && r.id === d.id ? null : r)));

  const answer = async (allow: boolean) => {
    if (!req) return;
    setBusy(true);
    try {
      const ok = await api.browser.respond(req.id, allow);
      if (!ok) toast.info('That request has expired', 'Click Connect in the browser again.');
      else if (allow) toast.success(`${req.name} is connected`, 'It can fill logins while the vault is unlocked.');
    } catch (e) {
      toast.error('Could not answer the request', errorText(e));
    } finally {
      setBusy(false);
      setReq(null);
    }
  };

  return (
    <Modal
      open={!!req}
      onClose={() => void answer(false)}
      title="Allow a browser to use your vault?"
      icon={Globe}
      footer={
        <>
          <Button variant="ghost" icon={X} disabled={busy} onClick={() => void answer(false)}>
            Deny
          </Button>
          <Button variant="primary" icon={ShieldCheck} loading={busy} onClick={() => void answer(true)}>
            Allow
          </Button>
        </>
      }
    >
      {req && (
        <div className="space-y-4 pb-2">
          <p className="text-[13.5px] text-dim">
            <b className="text-fg">{req.name}</b> wants to fill logins from your OmniHub vault. Allow it only if you just clicked <b className="text-fg">Connect</b> in the OmniHub extension.
          </p>
          <div className="rounded-2xl border border-line bg-surface-2 py-4 text-center">
            <div className="text-[11px] font-medium uppercase tracking-wider text-faint">The browser shows</div>
            <div className="mt-1 font-mono text-[34px] font-semibold tracking-[0.3em] text-fg tabular">{req.code}</div>
          </div>
          <p className="text-[12px] text-faint">Allowed browsers are listed in Vault → Browser autofill, where you can remove them at any time.</p>
        </div>
      )}
    </Modal>
  );
}
