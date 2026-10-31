// Folder rules: "PDFs in Downloads go to Documents\PDFs", "installers older
// than 30 days go to the Recycle Bin". Looked at every half minute.

import { formatRelative } from '@shared/format';
import type { FolderRule, RuleAction, RuleDone } from '@shared/types';
import { Check, FolderInput, FolderOpen, Pencil, Play, Plus, Trash, X } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Card, CardHeader } from '../../components/ui/Card';
import { Field, Segmented, Switch, TextInput } from '../../components/ui/Form';
import { EmptyState } from '../../components/ui/States';
import { useEvent } from '../../lib/hooks';
import { confirm } from '../../state/dialogs';
import { toast } from '../../state/toasts';

const blank = (): FolderRule => ({ id: '', name: '', enabled: true, folder: '', extensions: [], contains: [], action: { kind: 'move', to: '' } });
const split = (s: string) => s.split(/[,\s]+/).map((x) => x.trim()).filter(Boolean);
const what = (a: RuleAction) => (a.kind === 'move' ? `move to ${a.to}` : a.kind === 'copy' ? `copy to ${a.to}` : `Recycle Bin after ${a.days} days`);

function Editor({ rule, onDone }: { rule: FolderRule; onDone: () => void }) {
  const [r, setR] = useState(rule);
  const [exts, setExts] = useState(rule.extensions.join(', '));
  const [words, setWords] = useState(rule.contains.join(', '));
  const [busy, setBusy] = useState(false);
  const pick = (title: string, set: (p: string) => void) => void api.app.pickFolder(title).then((f) => f && set(f));
  const save = async () => {
    setBusy(true);
    try {
      await api.rules.save({ ...r, extensions: split(exts), contains: split(words) });
      toast.success('Rule saved');
      onDone();
    } catch (e) {
      toast.error('Couldn’t save the rule', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  const PathPick = ({ value, title, onPick }: { value: string; title: string; onPick: (p: string) => void }) => (
    <div className="flex gap-2">
      <TextInput value={value} readOnly placeholder="Choose a folder…" className="font-mono" />
      <Button icon={FolderOpen} onClick={() => pick(title, onPick)}>
        Choose
      </Button>
    </div>
  );
  return (
    <Card className="p-5">
      <CardHeader icon={FolderInput} title={rule.id ? 'Edit rule' : 'New rule'} actions={<IconButton icon={X} label="Close" onClick={onDone} />} />
      <div className="mt-4 grid gap-3 md:grid-cols-2">
        <Field label="Name">
          <TextInput value={r.name} onChange={(e) => setR({ ...r, name: e.target.value })} placeholder="PDFs to Documents" />
        </Field>
        <Field label="Watch this folder">
          <PathPick value={r.folder} title="Choose the folder to watch" onPick={(folder) => setR({ ...r, folder })} />
        </Field>
        <Field label="File types" hint="pdf, jpg, zip… (empty = any)">
          <TextInput value={exts} onChange={(e) => setExts(e.target.value)} placeholder="pdf" />
        </Field>
        <Field label="Name contains" hint="Any of these words (empty = any)">
          <TextInput value={words} onChange={(e) => setWords(e.target.value)} placeholder="invoice, receipt" />
        </Field>
      </div>
      <div className="mt-3">
        <Segmented
          label="Then"
          value={r.action.kind}
          onChange={(k) => setR({ ...r, action: k === 'recycle' ? { kind: 'recycle', days: 30 } : { kind: k, to: r.action.kind !== 'recycle' ? r.action.to : '' } })}
          options={[
            { value: 'move', label: 'Move' },
            { value: 'copy', label: 'Copy' },
            { value: 'recycle', label: 'Recycle Bin when old' },
          ]}
        />
      </div>
      <div className="mt-3">
        {r.action.kind === 'recycle' ? (
          <Field label="Older than (days)">
            <TextInput type="number" min={1} value={r.action.days} onChange={(e) => setR({ ...r, action: { kind: 'recycle', days: Math.max(1, Number(e.target.value) || 1) } })} className="w-32" />
          </Field>
        ) : (
          <Field label={r.action.kind === 'move' ? 'Move to' : 'Copy to'}>
            <PathPick value={r.action.to} title="Choose where the files go" onPick={(to) => setR({ ...r, action: { kind: r.action.kind === 'copy' ? 'copy' : 'move', to } })} />
          </Field>
        )}
      </div>
      <div className="mt-4 flex justify-end gap-2">
        <Button variant="ghost" onClick={onDone}>
          Cancel
        </Button>
        <Button variant="primary" icon={Check} loading={busy} onClick={() => void save()}>
          Save rule
        </Button>
      </div>
    </Card>
  );
}

export function RulesTab() {
  const [rules, setRules] = useState<FolderRule[]>([]);
  const [log, setLog] = useState<RuleDone[]>([]);
  const [editing, setEditing] = useState<FolderRule | null>(null);
  const load = useCallback(() => {
    void api.rules.list().then(setRules, () => undefined);
    void api.rules.log().then(setLog, () => undefined);
  }, []);
  useEffect(load, [load]);
  useEvent('rules:done', load);

  return (
    <div className="flex flex-col gap-4">
      {editing ? (
        <Editor rule={editing} onDone={() => { setEditing(null); load(); }} />
      ) : (
        <Card className="p-5">
          <CardHeader
            icon={FolderInput}
            title="Folder rules"
            subtitle="Tidy folders by themselves. A file is only touched once it has finished downloading; subfolders are left alone; deletions go to the Recycle Bin."
            actions={
              <div className="flex gap-2">
                <Button size="sm" icon={Play} onClick={() => void api.rules.runNow().then((d) => { toast.info(d.length ? `${d.length} file${d.length === 1 ? '' : 's'} sorted` : 'Nothing to do right now'); load(); })}>
                  Run now
                </Button>
                <Button size="sm" variant="primary" icon={Plus} onClick={() => setEditing(blank())}>
                  New rule
                </Button>
              </div>
            }
          />
          <div className="mt-4 space-y-2">
            {!rules.length && <EmptyState compact icon={FolderInput} title="No rules yet" description="For example: PDFs in Downloads → Documents\PDFs." />}
            {rules.map((r) => (
              <div key={r.id} className="flex items-center gap-3 rounded-xl border border-line bg-surface px-3 py-2">
                <Switch checked={r.enabled} onChange={(enabled) => void api.rules.save({ ...r, enabled }).then(load, (e: unknown) => toast.error('Couldn’t change it', errorText(e)))} label={`${r.name} on`} />
                <div className="min-w-0 flex-1">
                  <div className="truncate text-[13.5px] font-semibold text-fg">{r.name}</div>
                  <div className="truncate text-[12px] text-dim">
                    {r.extensions.length ? r.extensions.map((e) => `.${e}`).join(' ') : 'Any file'}
                    {r.contains.length ? ` containing ${r.contains.join(' / ')}` : ''} in {r.folder} → {what(r.action)}
                  </div>
                </div>
                <IconButton size="sm" icon={Pencil} label="Edit" onClick={() => setEditing(r)} />
                <IconButton
                  size="sm"
                  icon={Trash}
                  label="Delete"
                  onClick={async () => {
                    if (await confirm({ title: `Delete “${r.name}”?`, confirmLabel: 'Delete', tone: 'danger' })) await api.rules.remove(r.id).then(load);
                  }}
                />
              </div>
            ))}
          </div>
        </Card>
      )}
      {log.length > 0 && (
        <Card className="p-5">
          <CardHeader icon={Check} title="Recently" />
          <div className="mt-3 max-h-72 overflow-auto">
            {log.slice(0, 50).map((d, i) => (
              <div key={i} className="flex items-center gap-2 border-b border-line py-1.5 text-[12.5px] last:border-0">
                <Badge tone={d.ok ? 'good' : 'bad'}>{d.rule}</Badge>
                <span className="min-w-0 flex-1 truncate text-fg">{d.file}</span>
                <span className="truncate text-dim">{d.what}</span>
                <span className="shrink-0 text-faint">{formatRelative(d.at)}</span>
              </div>
            ))}
          </div>
        </Card>
      )}
    </div>
  );
}
