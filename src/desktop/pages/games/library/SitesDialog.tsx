// The quick sites shown in Big Picture's Watch tab and on the phone: add
// your own, remove any, or go back to the usual list. Also the browser they
// open in, Continue watching and the controller remote.

import type { Browser, Site, TvPrefs } from '@shared/types';
import { Plus, RotateCcw, Trash2, Tv } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../../api';
import { Button, IconButton } from '../../../components/ui/Button';
import { Select, Switch, TextInput } from '../../../components/ui/Form';
import { Modal } from '../../../components/ui/Overlay';
import { toast } from '../../../state/toasts';

export function SitesDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const [sites, setSites] = useState<Site[]>([]);
  const [name, setName] = useState('');
  const [url, setUrl] = useState('https://');
  const [tv, setTv] = useState<TvPrefs | null>(null);
  const [browsers, setBrowsers] = useState<Browser[]>([]);
  useEffect(() => {
    if (!open) return;
    void api.sites.list().then(setSites, () => undefined);
    void api.tv.prefs().then(setTv, () => undefined);
    void api.tv.browsers().then(setBrowsers, () => undefined);
  }, [open]);
  const saveTv = (patch: Partial<TvPrefs>) => {
    if (!tv) return;
    const next = { ...tv, ...patch };
    setTv(next);
    api.tv.setPrefs(next).then(setTv, (e) => toast.error('Couldn’t save that', errorText(e)));
  };
  const known = !!tv && browsers.some((b) => b.id === tv.browser);
  const save = async (list: Site[]) => {
    try {
      setSites(await api.sites.save(list));
      return true;
    } catch (e) {
      toast.error('Couldn’t save the sites', errorText(e));
      return false;
    }
  };
  const add = async () => {
    if (await save([...sites, { id: '', name, url, color: '#6366f1' }])) {
      setName('');
      setUrl('https://');
    }
  };
  return (
    <Modal open={open} onClose={onClose} icon={Tv} title="Quick sites" description="Shown in Big Picture’s Watch tab and on your phone. Full screen opens a site in its own window with no browser bars." size="md">
      <div className="space-y-3">
        {tv && (
          <div className="space-y-2.5 rounded-xl border border-line bg-surface-2/50 p-3">
            <div className="flex items-center justify-between gap-3">
              <div className="min-w-0">
                <div className="text-[13.5px] font-medium">Open sites in</div>
                <div className="text-[11.5px] text-faint">{tv.browser ? 'Big Picture asks only the first time.' : 'Not picked yet: Big Picture asks the first time you open one.'}</div>
              </div>
              <Select value={tv.browser} onChange={(e) => saveTv({ browser: e.target.value })} aria-label="Browser for sites" className="w-48">
                <option value="">Ask in Big Picture</option>
                {browsers.map((b) => (
                  <option key={b.id} value={b.id}>
                    {b.name}
                  </option>
                ))}
                <option value="system">My usual browser</option>
              </Select>
            </div>
            <div className="flex items-center justify-between gap-3">
              <div className="min-w-0">
                <div className="text-[13.5px] font-medium">Continue watching</div>
                <div className="text-[11.5px] text-faint">Reads only these streaming sites’ watch pages from that browser’s history, on this PC.</div>
              </div>
              <Switch checked={tv.continueWatching} disabled={!known} onChange={(v) => saveTv({ continueWatching: v })} label="Continue watching" />
            </div>
            <div className="flex items-center justify-between gap-3">
              <div className="min-w-0">
                <div className="text-[13.5px] font-medium">Controller as the remote</div>
                <div className="text-[11.5px] text-faint">In Big Picture: pointer, click, play / pause; View comes back.</div>
              </div>
              <Switch checked={tv.remote} disabled={!known} onChange={(v) => saveTv({ remote: v })} label="Controller as the remote" />
            </div>
          </div>
        )}
        <div className="max-h-[320px] space-y-1 overflow-y-auto pr-1">
          {sites.map((s, i) => (
            <div key={s.id || i} className="flex items-center gap-3 rounded-lg px-2 py-1.5 hover:bg-surface-2">
              <input type="color" value={s.color} aria-label={`${s.name} colour`} onChange={(e) => void save(sites.map((x, j) => (j === i ? { ...x, color: e.target.value } : x)))} className="h-7 w-7 shrink-0 cursor-pointer rounded border-0 bg-transparent p-0" />
              <div className="min-w-0 flex-1">
                <div className="truncate text-[13.5px] font-medium">{s.name}</div>
                <div className="truncate text-[11.5px] text-faint">{s.url}</div>
              </div>
              <IconButton icon={Trash2} label={`Remove ${s.name}`} size="sm" variant="danger" onClick={() => void save(sites.filter((_, j) => j !== i))} />
            </div>
          ))}
        </div>
        <div className="flex gap-2">
          <TextInput value={name} onChange={(e) => setName(e.target.value)} placeholder="Name" className="w-36" aria-label="Site name" />
          <TextInput value={url} onChange={(e) => setUrl(e.target.value)} placeholder="https://…" className="min-w-0 flex-1" aria-label="Site address" />
          <Button variant="primary" icon={Plus} disabled={!name.trim() || url.trim().length < 12} onClick={() => void add()}>
            Add
          </Button>
        </div>
        <Button variant="ghost" icon={RotateCcw} onClick={() => void api.sites.reset().then(setSites)}>
          Back to the usual list
        </Button>
      </div>
    </Modal>
  );
}
