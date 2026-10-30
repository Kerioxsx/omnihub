// "Share → OmniHub" from any app on the phone (Android, with OmniHub added
// to the home screen): a link or text arrives here, and goes to the PC.

import { ClipboardCopy, ExternalLink, Lightbulb } from 'lucide-react';
import { useState } from 'react';
import { client } from '../client';
import { errorMessage } from '../lib/util';
import { toast, useApp } from '../state';
import { Sheet } from '../ui/Sheet';

export interface Shared {
  title: string;
  text: string;
  url: string;
}

/** What was shared into the app's address (the manifest's share target), once. */
export function takeShared(): Shared | null {
  const p = new URLSearchParams(location.search);
  if (!p.has('share_title') && !p.has('share_text') && !p.has('share_url')) return null;
  const shared = { title: p.get('share_title') ?? '', text: p.get('share_text') ?? '', url: p.get('share_url') ?? '' };
  for (const k of ['share_title', 'share_text', 'share_url']) p.delete(k);
  const rest = p.toString();
  history.replaceState(history.state, '', location.pathname + (rest ? `?${rest}` : '') + location.hash);
  return shared.title || shared.text || shared.url ? shared : null;
}

export function ShareSheet({ shared, onClose }: { shared: Shared | null; onClose: () => void }) {
  const f = useApp((s) => s.info?.features);
  const [busy, setBusy] = useState<string | null>(null);
  if (!shared) return null;
  // Apps put the link in `url` or inside `text`.
  const link = shared.url || shared.text.match(/https?:\/\/\S+/i)?.[0] || '';
  const body = [shared.text, shared.url].filter((x, i, a) => x && a.indexOf(x) === i).join('\n');
  const go = async (what: string, run: () => Promise<unknown>, done: string) => {
    setBusy(what);
    try {
      await run();
      toast.success(done);
      onClose();
    } catch (e) {
      toast.error("Couldn't send it", errorMessage(e));
    } finally {
      setBusy(null);
    }
  };
  const row = 'press flex w-full items-center gap-3 rounded-2xl bg-surface-2 px-4 py-3.5 text-left disabled:opacity-40';
  return (
    <Sheet open onClose={onClose} title="Send to your PC" subtitle={shared.title || link || shared.text.slice(0, 80)}>
      <div className="space-y-2 pb-2">
        {link && f?.apps && (
          <button type="button" className={row} disabled={!!busy} onClick={() => go('open', () => client.openLink(link), 'Opened on the PC')}>
            <ExternalLink size={20} className="text-accent" />
            <span className="flex-1">
              <span className="block text-[15px] font-semibold">Open on the PC</span>
              <span className="block truncate text-[12.5px] text-dim">{link}</span>
            </span>
          </button>
        )}
        {f?.clipboard && (
          <button type="button" className={row} disabled={!!busy} onClick={() => go('copy', () => client.sendClipboard(link || body), 'On the PC clipboard')}>
            <ClipboardCopy size={20} className="text-accent" />
            <span className="flex-1">
              <span className="block text-[15px] font-semibold">Copy on the PC</span>
              <span className="block text-[12.5px] text-dim">Paste it there with Ctrl+V</span>
            </span>
          </button>
        )}
        {f?.notes && (
          <button type="button" className={row} disabled={!!busy} onClick={() => go('idea', () => client.createNote({ kind: 'idea', title: (shared.title || body.split('\n')[0]).slice(0, 80) || 'Shared from my phone', body, sendToClaude: false }), 'Saved as an idea')}>
            <Lightbulb size={20} className="text-accent" />
            <span className="flex-1">
              <span className="block text-[15px] font-semibold">Save as an idea</span>
              <span className="block text-[12.5px] text-dim">In Notes on the PC</span>
            </span>
          </button>
        )}
      </div>
    </Sheet>
  );
}
