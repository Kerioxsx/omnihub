// Custom Roblox flags: each one switched on or off, its value edited in
// place, and every flag Roblox's client has to pick from (a community
// tracker's list, with the ones Roblox still reads marked).

import type { KnownFlag, RobloxFlags } from '@shared/types';
import { ListFilter, Plus, RefreshCw, Search, Trash2 } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Switch, TextInput } from '../../components/ui/Form';
import { Modal } from '../../components/ui/Overlay';
import { toast } from '../../state/toasts';

type Custom = RobloxFlags['custom'];

export function CustomFlags({ flags, onChange, ignored }: { flags: RobloxFlags; onChange: (f: RobloxFlags) => void; ignored: string[] }) {
  const [name, setName] = useState('');
  const [value, setValue] = useState('');
  const [browsing, setBrowsing] = useState(false);
  const entries = Object.entries(flags.custom);
  const off = new Set(flags.customOff ?? []);

  const put = (custom: Custom, customOff: string[]) => onChange({ ...flags, custom, customOff: customOff.filter((k) => k in custom) });
  const add = (n: string, v: string) => {
    const key = n.trim();
    if (!key) return;
    // A pasted JSON object adds all its flags.
    if (key.startsWith('{')) {
      try {
        const obj = JSON.parse(key) as Record<string, string | number | boolean>;
        put({ ...flags.custom, ...obj }, [...off]);
        setName('');
        return;
      } catch {
        toast.error('Not valid JSON', 'Paste an object like { "FFlagName": "True" }.');
        return;
      }
    }
    if (!/^[A-Za-z0-9_]+$/.test(key)) return toast.error('Not a flag name', 'Flag names are letters, digits and _, like DFIntTaskSchedulerTargetFps.');
    put({ ...flags.custom, [key]: v.trim() || 'True' }, [...off].filter((k) => k !== key));
    setName('');
    setValue('');
  };

  return (
    <div className="space-y-2">
      {entries.length > 0 && (
        <div className="divide-y divide-line overflow-hidden rounded-xl border border-line">
          {entries.map(([k, v]) => (
            <div key={k} className="flex items-center gap-2 px-3 py-2">
              <Switch size="md" checked={!off.has(k)} onChange={(on) => put(flags.custom, on ? [...off].filter((x) => x !== k) : [...off, k])} label={`${k} on`} />
              <div className="min-w-0 flex-1">
                <div className={`truncate font-mono text-[12px] ${off.has(k) ? 'text-faint line-through' : 'text-fg'}`}>{k}</div>
                {ignored.includes(k) && !off.has(k) && <div className="text-[11px] text-warn">Roblox ignores it (not on its allowlist)</div>}
              </div>
              <input value={String(v)} onChange={(e) => put({ ...flags.custom, [k]: e.target.value }, [...off])} className="h-8 w-28 rounded-lg border border-line bg-surface px-2 font-mono text-[12px] text-fg outline-none focus:border-accent/60" aria-label={`${k} value`} />
              <IconButton icon={Trash2} label={`Remove ${k}`} onClick={() => put(Object.fromEntries(entries.filter(([x]) => x !== k)), [...off])} />
            </div>
          ))}
        </div>
      )}
      <div className="flex flex-wrap items-center gap-2">
        <TextInput value={name} onChange={(e) => setName(e.target.value)} onKeyDown={(e) => e.key === 'Enter' && add(name, value)} placeholder="Flag name (or paste JSON)" aria-label="Flag name" className="min-w-0 flex-1 font-mono text-[12px]" />
        <TextInput value={value} onChange={(e) => setValue(e.target.value)} onKeyDown={(e) => e.key === 'Enter' && add(name, value)} placeholder="Value" aria-label="Flag value" className="w-28 font-mono text-[12px]" />
        <Button size="sm" variant="secondary" icon={Plus} disabled={!name.trim()} onClick={() => add(name, value)}>
          Add
        </Button>
        <Button size="sm" variant="ghost" icon={Search} onClick={() => setBrowsing(true)}>
          All flags
        </Button>
      </div>
      <FlagBrowser open={browsing} onClose={() => setBrowsing(false)} have={flags.custom} onAdd={(k, v) => add(k, v)} />
    </div>
  );
}

function FlagBrowser({ open, onClose, have, onAdd }: { open: boolean; onClose: () => void; have: Custom; onAdd: (name: string, value: string) => void }) {
  const [list, setList] = useState<KnownFlag[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [query, setQuery] = useState('');
  const [onlyAllowed, setOnlyAllowed] = useState(false);

  const load = async (refresh: boolean) => {
    setLoading(true);
    try {
      setList(await api.games.robloxFlagList(refresh));
    } catch (e) {
      toast.error('The flag list didn’t load', errorText(e));
    } finally {
      setLoading(false);
    }
  };
  useEffect(() => {
    if (open && !list && !loading) void load(false);
  }, [open, list, loading]);

  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    return (list ?? []).filter((f) => (!onlyAllowed || f.allowed) && (!q || f.name.toLowerCase().includes(q))).slice(0, 150);
  }, [list, query, onlyAllowed]);
  const total = list?.length ?? 0;

  return (
    <Modal open={open} onClose={onClose} title="All Roblox flags" description={`${total ? total.toLocaleString() : 'Every'} flag Roblox’s Windows client has, with the value Roblox sets now. Since late 2025 Roblox only reads the allowlisted ones from your PC; the others are listed so you can try them, but it ignores them.`} icon={ListFilter} size="lg">
      <div className="space-y-3">
        <div className="flex flex-wrap items-center gap-2">
          <TextInput value={query} onChange={(e) => setQuery(e.target.value)} placeholder="Find a flag (fps, shadow, texture…)" aria-label="Find a flag" className="min-w-0 flex-1" autoFocus />
          <label className="flex items-center gap-2 text-[12.5px] text-dim">
            <Switch checked={onlyAllowed} onChange={setOnlyAllowed} label="Only allowlisted" /> Only ones Roblox reads
          </label>
          <IconButton icon={RefreshCw} label="Get the newest list" onClick={() => void load(true)} disabled={loading} />
        </div>
        {loading && !list && <div className="py-6 text-center text-[12.5px] text-faint">Getting the list…</div>}
        <div className="max-h-[55vh] divide-y divide-line overflow-y-auto rounded-xl border border-line">
          {shown.map((f) => (
            <div key={f.name} className="flex items-center gap-2 px-3 py-1.5">
              <div className="min-w-0 flex-1">
                <div className="truncate font-mono text-[12px] text-fg">{f.name}</div>
                <div className="text-[11px] text-faint">
                  {f.value != null ? `Roblox: ${f.value || '(empty)'}` : 'Not set by Roblox'} {f.allowed ? <span className="text-good">· allowlisted</span> : <span>· ignored from your PC</span>}
                </div>
              </div>
              <Button size="sm" variant={f.name in have ? 'ghost' : 'secondary'} icon={Plus} disabled={f.name in have} onClick={() => onAdd(f.name, f.value ?? 'True')}>
                {f.name in have ? 'Added' : 'Add'}
              </Button>
            </div>
          ))}
          {list && shown.length === 0 && <div className="px-3 py-6 text-center text-[12.5px] text-faint">No flag by that name.</div>}
        </div>
        {list && shown.length === 150 && <div className="text-[11.5px] text-faint">Showing the first 150; type more of the name to narrow it.</div>}
      </div>
    </Modal>
  );
}
