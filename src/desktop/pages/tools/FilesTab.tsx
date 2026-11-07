// File tools: checksums, the shredder, bulk rename with a live preview,
// and the files Windows lists as recently used.

import { formatBytes, formatRelative } from '@shared/format';
import type { HashKind, RecentFile, RenameCase, RenameItem, RenameRule } from '@shared/types';
import { Check, Copy, ExternalLink, FileSearch, FolderOpen, Flame, History, PenLine, ShieldCheck, Square, X } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Card, CardHeader, Skeleton } from '../../components/ui/Card';
import { Field, Segmented, Select, TextInput } from '../../components/ui/Form';
import { ProgressBar } from '../../components/ui/Progress';
import { Callout, EmptyState } from '../../components/ui/States';
import { useEvent } from '../../lib/hooks';
import { copyText } from '../../lib/util';
import { confirm } from '../../state/dialogs';
import { toast } from '../../state/toasts';

const base = (p: string) => p.split(/[\\/]/).pop() ?? p;
const newJob = () => Math.random().toString(36).slice(2);

function useProgress(job: string | null) {
  const [p, setP] = useState<{ done: number; total: number } | null>(null);
  useEvent<{ job: string; done: number; total: number }>('tools:progress', (e) => {
    if (e.job === job) setP({ done: e.done, total: e.total });
  });
  useEffect(() => setP(null), [job]);
  return p;
}

function Checksum() {
  const [path, setPath] = useState<string | null>(null);
  const [kind, setKind] = useState<HashKind>('sha256');
  const [expected, setExpected] = useState('');
  const [result, setResult] = useState<{ path: string; kind: HashKind; hash: string } | null>(null);
  const [job, setJob] = useState<string | null>(null);
  const progress = useProgress(job);

  // A pasted checksum tells which kind it is by its length.
  const clean = expected.replace(/[^0-9a-f]/gi, '').toLowerCase();
  useEffect(() => {
    if (clean.length === 40) setKind('sha1');
    else if (clean.length === 128) setKind('sha512');
    else if (clean.length === 64) setKind('sha256');
  }, [clean.length]);

  const run = async (p: string, k: HashKind) => {
    const id = newJob();
    setJob(id);
    setResult(null);
    try {
      setResult({ path: p, kind: k, hash: await api.files.hash(p, k, id) });
    } catch (e) {
      if (errorText(e) !== 'Stopped.') toast.error('Couldn’t check the file', errorText(e));
    } finally {
      setJob(null);
    }
  };
  const pick = async () => {
    const [p] = await api.app.pickFiles('Choose a file to check');
    if (p) {
      setPath(p);
      void run(p, kind);
    }
  };
  const fresh = result && result.path === path && result.kind === kind ? result.hash : null;
  const verdict = fresh && clean ? (fresh === clean ? 'match' : 'differ') : null;

  return (
    <Card className="p-5">
      <CardHeader icon={ShieldCheck} title="Checksum" subtitle="Make sure a download is exactly the file it should be: paste the SHA-256 from the website, pick the file." />
      <div className="mt-4 grid gap-3 md:grid-cols-[1fr_auto]">
        <Field label="Checksum from the website (optional)">
          <TextInput value={expected} onChange={(e) => setExpected(e.target.value)} placeholder="e.g. 9f86d081884c7d659a2feaa0c55ad015…" spellCheck={false} className="font-mono" />
        </Field>
        <Field label="Kind">
          <Segmented label="Checksum kind" value={kind} onChange={(k) => { setKind(k); if (path) void run(path, k); }} options={[{ value: 'sha256', label: 'SHA-256' }, { value: 'sha1', label: 'SHA-1' }, { value: 'sha512', label: 'SHA-512' }]} />
        </Field>
      </div>
      <div className="mt-3 flex flex-wrap items-center gap-2">
        <Button variant="primary" icon={FileSearch} onClick={() => void pick()} disabled={!!job}>
          {path ? 'Check another file' : 'Choose a file'}
        </Button>
        {path && <span className="truncate text-[13px] text-dim">{base(path)}</span>}
        {job && <Button size="sm" variant="ghost" icon={Square} onClick={() => void api.hidden.cancel(job)}>Stop</Button>}
      </div>
      {job && <ProgressBar className="mt-3" value={progress && progress.total ? progress.done / progress.total : null} label="Reading the file" />}
      {fresh && (
        <div className="mt-4 rounded-xl bg-surface-2 p-3">
          <div className="flex items-center justify-between gap-2">
            <code className="break-all font-mono text-[12.5px] text-fg">{fresh}</code>
            <IconButton icon={Copy} label="Copy" onClick={() => void copyText(fresh).then(() => toast.success('Copied'))} />
          </div>
          {verdict === 'match' && <Badge tone="good" icon={Check} className="mt-2">Matches: the file is exactly right</Badge>}
          {verdict === 'differ' && <Badge tone="bad" icon={X} className="mt-2">Doesn’t match: the file is different or damaged, don’t run it</Badge>}
        </div>
      )}
    </Card>
  );
}

function Shredder() {
  const [job, setJob] = useState<string | null>(null);
  const shred = async (path: string) => {
    const name = base(path);
    const ok = await confirm({
      title: `Shred ${name}?`,
      description: 'It is overwritten with random data, then deleted. It won’t go to the Recycle Bin and can’t be brought back.',
      details: <span className="text-[12.5px] text-dim">On SSDs and USB sticks the drive itself may keep old copies of the data where no program can reach them; for those, BitLocker (or another full-disk encryption) is what keeps deleted files unreadable.</span>,
      typed: name,
      confirmLabel: 'Shred for good',
      tone: 'danger',
    });
    if (!ok) return;
    const id = newJob();
    setJob(id);
    try {
      const n = await api.files.shred(path, id);
      toast.success(`Shredded ${name}`, n === 1 ? undefined : `${n} files`);
    } catch (e) {
      toast.error('Couldn’t shred it', errorText(e));
    } finally {
      setJob(null);
    }
  };
  return (
    <Card className="p-5">
      <CardHeader icon={Flame} title="Secure shredder" subtitle="Delete a file or folder so it can’t be recovered with undelete tools." />
      <div className="mt-4 flex flex-wrap gap-2">
        <Button variant="danger" icon={Flame} loading={!!job} onClick={() => void api.app.pickFiles('Choose what to shred').then((f) => { if (f[0]) void shred(f[0]); })}>
          Shred a file…
        </Button>
        <Button variant="outline" icon={FolderOpen} disabled={!!job} onClick={() => void api.app.pickFolder('Choose a folder to shred').then((f) => { if (f) void shred(f); })}>
          Shred a folder…
        </Button>
      </div>
    </Card>
  );
}

const CASES: { value: RenameCase; label: string }[] = [
  { value: 'keep', label: 'Keep' },
  { value: 'lower', label: 'lower' },
  { value: 'upper', label: 'UPPER' },
  { value: 'title', label: 'Title' },
];

function BulkRename() {
  const [files, setFiles] = useState<string[]>([]);
  const [rule, setRule] = useState<RenameRule>({ pattern: '', find: '', replace: '', case: 'keep', start: 1 });
  const [plan, setPlan] = useState<RenameItem[]>([]);
  const [busy, setBusy] = useState(false);
  const set = (p: Partial<RenameRule>) => setRule((r) => ({ ...r, ...p }));

  useEffect(() => {
    if (!files.length) return setPlan([]);
    const t = setTimeout(() => void api.files.renamePlan(files, rule).then(setPlan, () => undefined), 120);
    return () => clearTimeout(t);
  }, [files, rule]);
  const problems = plan.filter((p) => p.problem).length;
  const changes = useMemo(() => plan.filter((p) => base(p.from) !== p.to).length, [plan]);

  const apply = async () => {
    setBusy(true);
    try {
      const n = await api.files.rename(files, rule);
      toast.success(`Renamed ${n} file${n === 1 ? '' : 's'}`);
      setFiles(plan.map((p) => p.from.slice(0, p.from.length - base(p.from).length) + p.to));
      setRule((r) => ({ ...r, pattern: '', find: '', replace: '', case: 'keep' }));
    } catch (e) {
      toast.error('Nothing was renamed', errorText(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Card className="p-5">
      <CardHeader
        icon={PenLine}
        title="Bulk rename"
        subtitle="Rename many files at once and see every new name before anything changes."
        actions={<Button size="sm" icon={FolderOpen} onClick={() => void api.app.pickFiles('Choose files to rename').then((f) => f.length && setFiles(f))}>{files.length ? `${files.length} files` : 'Choose files…'}</Button>}
      />
      {files.length > 0 && (
        <>
          <div className="mt-4 grid gap-3 md:grid-cols-2">
            <Field label="New name" hint="{name} old name · {n} or {n:3} number · {date} {time} · {ext}">
              <TextInput value={rule.pattern} onChange={(e) => set({ pattern: e.target.value })} placeholder="Holiday {n:3}" />
            </Field>
            <Field label="Start numbers at">
              <TextInput type="number" min={0} value={rule.start} onChange={(e) => set({ start: Math.max(0, Number(e.target.value) || 0) })} />
            </Field>
            <Field label="Find">
              <TextInput value={rule.find} onChange={(e) => set({ find: e.target.value })} placeholder="IMG_" />
            </Field>
            <Field label="Replace with">
              <TextInput value={rule.replace} onChange={(e) => set({ replace: e.target.value })} />
            </Field>
          </div>
          <div className="mt-3 flex flex-wrap items-center justify-between gap-2">
            <Segmented label="Letter case" size="sm" value={rule.case} onChange={(c) => set({ case: c })} options={CASES} />
            <Button variant="primary" loading={busy} disabled={!changes || problems > 0} onClick={() => void apply()}>
              Rename {changes} file{changes === 1 ? '' : 's'}
            </Button>
          </div>
          {problems > 0 && <Callout tone="warn" className="mt-3">{problems} name{problems === 1 ? '' : 's'} can’t be used yet; fix them and the button comes back.</Callout>}
          <div className="mt-3 max-h-72 overflow-auto rounded-xl border border-line">
            {plan.map((p) => (
              <div key={p.from} className="grid grid-cols-2 gap-3 border-b border-line px-3 py-1.5 text-[12.5px] last:border-0">
                <span className="truncate text-dim">{base(p.from)}</span>
                <span className={p.problem ? 'truncate text-bad' : base(p.from) === p.to ? 'truncate text-faint' : 'truncate text-fg'} title={p.problem || p.to}>
                  {p.to}
                  {p.problem && <span className="ml-2 text-[11.5px]">({p.problem})</span>}
                </span>
              </div>
            ))}
          </div>
        </>
      )}
    </Card>
  );
}

function Recent() {
  const [items, setItems] = useState<RecentFile[] | null>(null);
  const [kind, setKind] = useState('all');
  useEffect(() => void api.files.recent(120).then(setItems, () => setItems([])), []);
  const groups: Record<string, string[]> = { documents: ['pdf', 'doc', 'docx', 'xls', 'xlsx', 'ppt', 'pptx', 'txt', 'md', 'csv'], pictures: ['png', 'jpg', 'jpeg', 'gif', 'webp', 'heic', 'bmp'], videos: ['mp4', 'mov', 'mkv', 'avi', 'webm'], other: [] };
  const extOf = (n: string) => (n.includes('.') ? n.split('.').pop()!.toLowerCase() : '');
  const shown = (items ?? []).filter((f) => kind === 'all' || (kind === 'other' ? !Object.values(groups).flat().includes(extOf(f.name)) : groups[kind].includes(extOf(f.name))));
  return (
    <Card className="p-5">
      <CardHeader
        icon={History}
        title="Recent files"
        subtitle="What you opened lately, from Windows’ own list."
        actions={
          <Select value={kind} onChange={(e) => setKind(e.target.value)} aria-label="Show">
            <option value="all">Everything</option>
            <option value="documents">Documents</option>
            <option value="pictures">Pictures</option>
            <option value="videos">Videos</option>
            <option value="other">Other</option>
          </Select>
        }
      />
      <div className="mt-3">
        {!items ? (
          <Skeleton className="h-40 rounded-xl" />
        ) : !shown.length ? (
          <EmptyState compact icon={History} title="Nothing here yet" description="Files you open in other apps show up here." />
        ) : (
          <div className="max-h-96 overflow-auto">
            {shown.map((f) => (
              <div key={f.path} className="group flex items-center gap-3 rounded-lg px-2 py-1.5 hover:bg-surface-2">
                <div className="min-w-0 flex-1">
                  <div className="truncate text-[13px] text-fg">{f.name}</div>
                  <div className="truncate text-[11.5px] text-faint">{f.path}</div>
                </div>
                <span className="shrink-0 text-[12px] text-faint">{formatBytes(f.size)} · {formatRelative(f.used)}</span>
                <IconButton size="sm" icon={ExternalLink} label="Open" onClick={() => void api.app.openPath(f.path).catch((e: unknown) => toast.error('Couldn’t open it', errorText(e)))} />
                <IconButton size="sm" icon={FolderOpen} label="Show in folder" onClick={() => void api.app.revealPath(f.path)} />
              </div>
            ))}
          </div>
        )}
      </div>
    </Card>
  );
}

export function FilesTab() {
  return (
    <div className="grid gap-4 xl:grid-cols-2">
      <Checksum />
      <Shredder />
      <BulkRename />
      <Recent />
    </div>
  );
}
