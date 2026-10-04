import { formatRelative } from '@shared/format';
import type { BrowserStatus } from '@shared/types';
import { Check, Copy, FolderOpen, Globe, RefreshCw, Trash, Wrench } from 'lucide-react';
import { type ReactNode, useCallback, useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Spinner } from '../../components/ui/Card';
import { Switch } from '../../components/ui/Form';
import { Modal } from '../../components/ui/Overlay';
import { Callout } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { useEvent } from '../../lib/hooks';
import { copyText } from '../../lib/util';
import { confirm } from '../../state/dialogs';
import { useSettings } from '../../state/settings';
import { toast } from '../../state/toasts';

function Step({ n, done, title, children }: { n: number; done?: boolean; title: ReactNode; children?: ReactNode }) {
  return (
    <li className="flex gap-3">
      <span className={cx('mt-0.5 flex h-6 w-6 shrink-0 items-center justify-center rounded-full text-[12px] font-semibold', done ? 'bg-good/15 text-good' : 'bg-accent-soft text-accent')}>{done ? <Check size={13} /> : n}</span>
      <div className="min-w-0 flex-1 pb-4">
        <div className="text-[13.5px] font-medium text-fg">{title}</div>
        {children && <div className="mt-1 text-[12.5px] leading-relaxed text-dim">{children}</div>}
      </div>
    </li>
  );
}

function CopyChip({ text, label }: { text: string; label?: string }) {
  const [done, setDone] = useState(false);
  return (
    <button
      type="button"
      onClick={() =>
        void copyText(text).then((ok) => {
          setDone(ok);
          setTimeout(() => setDone(false), 1500);
        })
      }
      className="mt-1.5 inline-flex max-w-full items-center gap-2 rounded-lg border border-line bg-surface-2 px-2.5 py-1.5 font-mono text-[12px] text-fg hover:border-accent/40"
      title="Copy"
    >
      <span className="truncate">{label ?? text}</span>
      {done ? <Check size={13} className="shrink-0 text-good" /> : <Copy size={13} className="shrink-0 text-faint" />}
    </button>
  );
}

/** Brave / Chrome / Edge extension setup, paired browsers and options. */
export function BrowserAutofill({ open, onClose }: { open: boolean; onClose: () => void }) {
  const settings = useSettings((s) => s.settings);
  const update = useSettings((s) => s.update);
  const [st, setSt] = useState<BrowserStatus | null>(null);
  const [busy, setBusy] = useState(false);

  const load = useCallback(() => {
    api.browser.status().then(setSt, (e: unknown) => toast.error('Could not read the browser status', errorText(e)));
  }, []);
  useEffect(() => {
    if (open) load();
  }, [open, load]);
  useEvent('browser:clients', load);
  useEvent('browser:pair-done', load);
  useEvent('settings:changed', () => open && load());

  if (!settings) return null;
  const enabled = settings.vault.browserAutofill;
  const registered = st?.browsers.filter((b) => b.registered).map((b) => b.browser) ?? [];

  const toggle = async (on: boolean) => {
    setBusy(true);
    try {
      await update({ vault: { browserAutofill: on } });
      load();
      if (on) toast.success('Browser autofill is on', 'Now add the OmniHub extension to your browser.');
    } finally {
      setBusy(false);
    }
  };
  const repair = async () => {
    try {
      const r = await api.browser.repair();
      toast.success('Browsers updated', `${r.filter((b) => b.registered).map((b) => b.browser).join(', ') || 'No browser'} can reach OmniHub.`);
      load();
    } catch (e) {
      toast.error('Could not register OmniHub with the browsers', errorText(e));
    }
  };
  const revoke = async (id: string, name: string) => {
    const ok = await confirm({ title: `Remove ${name}?`, description: 'It can no longer fill logins from your vault until you connect it again.', tone: 'danger', confirmLabel: 'Remove' });
    if (!ok) return;
    await api.browser.revoke(id).catch((e: unknown) => toast.error('Could not remove it', errorText(e)));
    load();
  };

  return (
    <Modal open={open} onClose={onClose} title="Browser autofill" description="Fill usernames, emails, passwords and 2FA codes in Brave, Chrome and Edge." icon={Globe} size="lg">
      <div className="space-y-5 pb-2">
        <div className="flex items-center justify-between gap-4 rounded-xl border border-line bg-surface px-4 py-3">
          <div>
            <div className="text-[13.5px] font-medium text-fg">Let the OmniHub extension use this vault</div>
            <div className="text-[12px] text-faint">Logins fill only on the website they were saved for, and only while the vault is unlocked.</div>
          </div>
          <Switch checked={enabled} disabled={busy} onChange={(v) => void toggle(v)} label="Browser autofill" size="lg" />
        </div>

        {enabled && (
          <>
            <ol className="pl-1">
              <Step n={1} done={registered.length > 0} title="OmniHub is registered with your browsers">
                {st ? (
                  <span className="flex flex-wrap items-center gap-1.5">
                    {st.browsers.map((b) => (
                      <Badge key={b.browser} tone={b.registered ? 'good' : 'neutral'}>
                        {b.browser}
                        {b.registered ? '' : ' — not found'}
                      </Badge>
                    ))}
                    <Button size="sm" variant="ghost" icon={Wrench} onClick={() => void repair()}>
                      Repair
                    </Button>
                  </span>
                ) : (
                  <Spinner size={14} />
                )}
              </Step>
              <Step n={2} title="Open the extensions page">
                Type this in the address bar (Chrome: <span className="font-mono">chrome://extensions</span>, Edge: <span className="font-mono">edge://extensions</span>):
                <div>
                  <CopyChip text="brave://extensions" />
                </div>
              </Step>
              <Step n={3} title={<>Turn on “Developer mode” (top right)</>} />
              <Step n={4} title={<>Click “Load unpacked” and choose this folder</>}>
                {st?.extensionDir ? (
                  <div className="flex flex-wrap items-center gap-2">
                    <CopyChip text={st.extensionDir} />
                    <Button size="sm" variant="ghost" icon={FolderOpen} onClick={() => st.extensionDir && void api.app.revealPath(st.extensionDir)}>
                      Show in Explorer
                    </Button>
                  </div>
                ) : (
                  <span className="text-warn">The extension folder is missing — reinstall OmniHub.</span>
                )}
              </Step>
              <Step n={5} done={(st?.clients.length ?? 0) > 0} title="Click the OmniHub icon in the browser and choose “Connect”">
                OmniHub then asks you here to allow it, with a code to compare. Pin the icon from the puzzle-piece menu so it stays in view.
              </Step>
            </ol>

            <div>
              <div className="mb-2 flex items-center justify-between">
                <div className="text-[12px] font-medium uppercase tracking-wider text-faint">Connected browsers</div>
                <IconButton icon={RefreshCw} label="Refresh" size="sm" onClick={load} />
              </div>
              {st && st.clients.length === 0 && <div className="rounded-xl border border-dashed border-line px-4 py-3 text-[12.5px] text-faint">No browser connected yet.</div>}
              <div className="space-y-1.5">
                {st?.clients.map((c) => (
                  <div key={c.id} className="flex items-center gap-3 rounded-xl border border-line bg-surface px-4 py-2.5">
                    <Globe size={16} className="text-accent" aria-hidden />
                    <div className="min-w-0 flex-1">
                      <div className="truncate text-[13.5px] font-medium text-fg">{c.name}</div>
                      <div className="text-[12px] text-faint">
                        Connected {formatRelative(c.created)} · last used {formatRelative(c.lastSeen)}
                      </div>
                    </div>
                    <IconButton icon={Trash} label={`Remove ${c.name}`} variant="danger" size="sm" onClick={() => void revoke(c.id, c.name)} />
                  </div>
                ))}
              </div>
            </div>

            <div className="flex items-center justify-between gap-4 rounded-xl border border-line bg-surface px-4 py-3">
              <div>
                <div className="text-[13.5px] font-medium text-fg">Offer to save logins I type</div>
                <div className="text-[12px] text-faint">After you sign in somewhere new, or change a password, the extension asks whether to save it here.</div>
              </div>
              <Switch checked={settings.vault.browserOfferSave} onChange={(v) => void update({ vault: { browserOfferSave: v } })} label="Offer to save logins" />
            </div>

            <Callout tone="info" title="Shortcuts in the browser">
              <span className="font-mono">Ctrl+Shift+L</span> fills the best login for the page. Clicking a sign-in box shows your logins for that site, and 2FA boxes get the current code from logins that have a 2FA secret.
            </Callout>
          </>
        )}
      </div>
    </Modal>
  );
}
