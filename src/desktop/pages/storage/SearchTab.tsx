import { formatBytes, formatDate, formatNumber } from '@shared/format';
import type { PathedNode, SearchQuery, SearchResult } from '@shared/types';
import { SearchX, Search as SearchIcon } from 'lucide-react';
import { type CSSProperties, type ReactNode, useEffect, useMemo, useState } from 'react';
import { api, errorText } from '../../api';
import { FileIcon } from '../../components/FileIcon';
import { Card, Skeleton, Spinner } from '../../components/ui/Card';
import { SearchInput, Segmented, Select, TextInput } from '../../components/ui/Form';
import { EmptyState, ErrorState } from '../../components/ui/States';
import { VirtualList } from '../../components/VirtualList';
import { useDebounced } from '../../lib/hooks';
import { useStorage } from '../../state/storage';
import { openItemMenu } from './actions';

const MB = 1024 * 1024;
const MIN_OPTS = [
  ['', 'Any size'],
  [String(MB), '≥ 1 MB'],
  [String(10 * MB), '≥ 10 MB'],
  [String(100 * MB), '≥ 100 MB'],
  [String(1024 * MB), '≥ 1 GB'],
  [String(10 * 1024 * MB), '≥ 10 GB'],
];
const MAX_OPTS = [
  ['', 'No limit'],
  [String(MB), '≤ 1 MB'],
  [String(100 * MB), '≤ 100 MB'],
  [String(1024 * MB), '≤ 1 GB'],
];
const AGE_OPTS = [
  ['', 'Any time'],
  ['a7', 'Last 7 days'],
  ['a30', 'Last 30 days'],
  ['b180', 'Older than 6 months'],
  ['b365', 'Older than a year'],
];

function highlight(name: string, q: string): ReactNode {
  const t = q.trim();
  if (!t || /[*?]/.test(t)) return name;
  const i = name.toLowerCase().indexOf(t.toLowerCase());
  if (i < 0) return name;
  return (
    <>
      {name.slice(0, i)}
      <mark className="rounded bg-accent/25 px-0.5 text-fg">{name.slice(i, i + t.length)}</mark>
      {name.slice(i + t.length)}
    </>
  );
}

export function SearchTab({ scanId }: { scanId: string }) {
  const nodeId = useStorage((s) => s.nodeByScan[scanId] ?? 0);
  const version = useStorage((s) => s.version);
  const [text, setText] = useState('');
  const [minSize, setMinSize] = useState('');
  const [maxSize, setMaxSize] = useState('');
  const [age, setAge] = useState('');
  const [kind, setKind] = useState<'all' | 'files' | 'dirs'>('all');
  const [exts, setExts] = useState('');
  const [scope, setScope] = useState<'all' | 'here'>('all');
  const [result, setResult] = useState<SearchResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const query = useMemo<SearchQuery>(() => {
    const now = Math.floor(Date.now() / 1000);
    const days = age ? Number(age.slice(1)) * 86400 : 0;
    return {
      text,
      under: scope === 'here' ? nodeId : null,
      minSize: minSize ? Number(minSize) : null,
      maxSize: maxSize ? Number(maxSize) : null,
      modifiedAfter: age.startsWith('a') ? now - days : null,
      modifiedBefore: age.startsWith('b') ? now - days : null,
      extensions: exts
        .split(/[\s,;]+/)
        .map((e) => e.replace(/^\./, '').trim())
        .filter(Boolean),
      filesOnly: kind === 'files',
      dirsOnly: kind === 'dirs',
      limit: 1000,
      sort: 'size',
    };
  }, [text, minSize, maxSize, age, kind, exts, scope, nodeId]);
  const debounced = useDebounced(query, 250);
  const active = !!(debounced.text.trim() || debounced.minSize || debounced.maxSize || debounced.modifiedAfter || debounced.modifiedBefore || debounced.extensions?.length);

  useEffect(() => {
    if (!active) {
      setResult(null);
      return;
    }
    let alive = true;
    setBusy(true);
    api.storage
      .search(scanId, debounced)
      .then((r) => {
        if (!alive) return;
        setResult(r);
        setError(null);
      })
      .catch((e: unknown) => alive && setError(errorText(e)))
      .finally(() => alive && setBusy(false));
    return () => {
      alive = false;
    };
  }, [scanId, debounced, active, version]);

  const row = (n: PathedNode, _i: number, style: CSSProperties) => (
    <div
      key={n.id}
      style={style}
      role="listitem"
      tabIndex={0}
      onDoubleClick={() => void api.app.revealPath(n.path)}
      onContextMenu={(e) => openItemMenu(e, { path: n.path, name: n.name, isDir: n.isDir, size: n.size })}
      className="grid grid-cols-[minmax(0,1fr)_96px_96px] items-center gap-3 rounded-lg px-3 outline-none transition-colors hover:bg-surface-2 focus-visible:bg-surface-3"
    >
      <div className="flex min-w-0 items-center gap-2.5">
        <FileIcon name={n.name} isDir={n.isDir} />
        <div className="min-w-0">
          <div className="truncate text-[13px] text-fg">{highlight(n.name, debounced.text)}</div>
          <div className="truncate font-mono text-[10.5px] text-faint">{n.path}</div>
        </div>
      </div>
      <span className="text-right text-[12.5px] font-medium tabular text-fg">{formatBytes(n.size)}</span>
      <span className="text-right text-[12px] tabular text-faint">{n.modified ? formatDate(n.modified) : '—'}</span>
    </div>
  );

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3">
      <Card className="flex flex-wrap items-center gap-2.5 px-4 py-3">
        <SearchInput value={text} onChange={setText} placeholder="File or folder name — * and ? work as wildcards" className="min-w-[280px] flex-1" autoFocus aria-label="Search names" />
        <Select value={minSize} onChange={(e) => setMinSize(e.target.value)} aria-label="Minimum size" className="w-[120px]">
          {MIN_OPTS.map(([v, l]) => (
            <option key={v} value={v}>
              {l}
            </option>
          ))}
        </Select>
        <Select value={maxSize} onChange={(e) => setMaxSize(e.target.value)} aria-label="Maximum size" className="w-[120px]">
          {MAX_OPTS.map(([v, l]) => (
            <option key={v} value={v}>
              {l}
            </option>
          ))}
        </Select>
        <Select value={age} onChange={(e) => setAge(e.target.value)} aria-label="Modified" className="w-[168px]">
          {AGE_OPTS.map(([v, l]) => (
            <option key={v} value={v}>
              {l}
            </option>
          ))}
        </Select>
        <TextInput value={exts} onChange={(e) => setExts(e.target.value)} placeholder="Extensions: mp4, iso" className="w-[170px]" aria-label="Extensions" />
        <Segmented
          size="sm"
          label="Kind"
          value={kind}
          onChange={setKind}
          options={[
            { value: 'all', label: 'All' },
            { value: 'files', label: 'Files' },
            { value: 'dirs', label: 'Folders' },
          ]}
        />
        <Segmented
          size="sm"
          label="Scope"
          value={scope}
          onChange={setScope}
          options={[
            { value: 'all', label: 'Whole scan' },
            { value: 'here', label: 'Current folder', disabled: !nodeId },
          ]}
        />
      </Card>

      <Card className="flex min-h-0 flex-1 flex-col p-1.5">
        <div className="flex items-center justify-between border-b border-line px-3 pb-2 pt-1.5">
          <div className="flex items-center gap-2 text-[12.5px] text-dim">
            {busy && <Spinner size={13} />}
            {result ? (
              <span>
                <span className="font-semibold text-fg tabular">{formatNumber(result.total)}</span> result{result.total === 1 ? '' : 's'} in <span className="tabular">{result.tookMs} ms</span>
                {result.total > result.items.length && <span className="text-faint"> · showing the largest {formatNumber(result.items.length)}</span>}
              </span>
            ) : (
              <span className="text-faint">Type a name or set a filter</span>
            )}
          </div>
          <div className="grid grid-cols-[96px_96px] gap-3 text-[11px] font-semibold uppercase tracking-wider text-faint">
            <span className="text-right">Size</span>
            <span className="text-right">Modified</span>
          </div>
        </div>
        {error ? (
          <ErrorState error={error} />
        ) : !active ? (
          <EmptyState icon={SearchIcon} title="Search this scan" description="Searches names across every file and folder instantly — no disk access needed. Try “*.iso”, “node_modules” or set “≥ 1 GB”." compact />
        ) : !result ? (
          <div className="space-y-1.5 p-2">
            {Array.from({ length: 8 }, (_, i) => (
              <Skeleton key={i} className="h-10" />
            ))}
          </div>
        ) : result.items.length === 0 ? (
          <EmptyState icon={SearchX} title="No matches" description="Try a shorter name or loosen the filters." compact />
        ) : (
          <VirtualList items={result.items} rowHeight={46} renderRow={row} className="flex-1 pt-1" ariaLabel="Search results" />
        )}
      </Card>
    </div>
  );
}
