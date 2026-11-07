// The quick sites shown in Big Picture's Watch row and on the phone: add
// your own, remove any, or go back to the usual list.

import type { Site } from '@shared/types';
import { Plus, RotateCcw, Trash2, Tv } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../../api';
import { Button, IconButton } from '../../../components/ui/Button';
import { TextInput } from '../../../components/ui/Form';
import { Modal } from '../../../components/ui/Overlay';
import { toast } from '../../../state/toasts';

export function SitesDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const [sites, setSites] = useState<Site[]>([]);
  const [name, setName] = useState('');
  const [url, setUrl] = useState('https://');
  useEffect(() => {
    if (open) void api.sites.list().then(setSites, () => undefined);
  }, [open]);
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
    <Modal open={open} onClose={onClose} icon={Tv} title="Quick sites" description="Shown in Big Picture’s Watch row and on your phone. Full screen opens a site in its own Edge window with no browser bars." size="md">
      <div className="space-y-3">
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
