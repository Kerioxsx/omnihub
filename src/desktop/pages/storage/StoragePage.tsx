import { formatBytes } from '@shared/format';
import { ArrowDownWideNarrow, Copy, FolderTree, PieChart, ScanLine, Search, Sparkles, Zap } from 'lucide-react';
import { useEffect, useRef } from 'react';
import { api } from '../../api';
import { Page } from '../../components/Page';
import { Button } from '../../components/ui/Button';
import { Badge, Skeleton } from '../../components/ui/Card';
import { type TabItem, Tabs } from '../../components/ui/Form';
import { ErrorState } from '../../components/ui/States';
import { useAsync } from '../../lib/hooks';
import { useRoute } from '../../lib/router';
import { useSettings } from '../../state/settings';
import { type StorageTab, rootKey, useStorage } from '../../state/storage';
import { CleanupTab } from './CleanupTab';
import { DuplicatesTab } from './DuplicatesTab';
import { ExplorerTab } from './ExplorerTab';
import { LargestTab } from './LargestTab';
import { DrivePicker, ModeHint, ModeToggle, NeverScanned, ScanProgressView, SummaryCard } from './ScanParts';
import { SearchTab } from './SearchTab';
import { TypesTab } from './TypesTab';

function CleanupBadge({ scanId }: { scanId: string }) {
  const version = useStorage((s) => s.version);
  const res = useAsync(() => api.storage.cleanup(scanId), [scanId, version]);
  const safe = (res.data ?? []).filter((s) => s.risk === 'safe').reduce((a, s) => a + s.size, 0);
  if (!safe) return null;
  return <span className="rounded-full bg-good/15 px-1.5 py-px text-[10.5px] font-semibold text-good tabular">{formatBytes(safe, 0)}</span>;
}

export function StoragePage() {
  const route = useRoute();
  const volumes = useStorage((s) => s.volumes);
  const loaded = useStorage((s) => s.volumesLoaded);
  const volumesError = useStorage((s) => s.volumesError);
  const root = useStorage((s) => s.root);
  const mode = useStorage((s) => s.mode);
  const summaries = useStorage((s) => s.summaries);
  const checking = useStorage((s) => s.checkingCache);
  const job = useStorage((s) => s.job);
  const failure = useStorage((s) => s.failure);
  const tab = useStorage((s) => s.tab);
  const { select, scan, cancel, setMode, setTab, loadVolumes } = useStorage.getState();
  const defaultMode = useSettings((s) => s.settings?.storage.defaultMode);
  const modeInit = useRef(false);

  useEffect(() => {
    if (!modeInit.current && defaultMode) {
      modeInit.current = true;
      if (!useStorage.getState().job) setMode(defaultMode === 'standard' ? 'standard' : 'fast');
    }
  }, [defaultMode, setMode]);

  useEffect(() => {
    if (!loaded) void loadVolumes();
  }, [loaded, loadVolumes]);

  // Pick a drive: ?root= from the URL, else keep the current one, else the system drive.
  useEffect(() => {
    if (!loaded) return;
    const wanted = route.params.get('root');
    if (wanted && (!root || rootKey(wanted) !== rootKey(root))) void select(wanted);
    else if (!root && volumes.length) void select((volumes.find((v) => v.root.toUpperCase().startsWith('C:')) ?? volumes[0]).root);
    else if (root && !summaries[rootKey(root)]) void select(root);
  }, [loaded, volumes, route.params, root]); // eslint-disable-line react-hooks/exhaustive-deps

  const key = root ? rootKey(root) : null;
  const volume = key ? (volumes.find((v) => rootKey(v.root) === key) ?? null) : null;
  const summary = key ? summaries[key] : undefined;
  const scanning = !!job && !!key && rootKey(job.root) === key;
  const otherJob = !!job && !scanning;

  const tabs: TabItem<StorageTab>[] = [
    { value: 'explorer', label: 'Explorer', icon: FolderTree },
    { value: 'largest', label: 'Largest files', icon: ArrowDownWideNarrow },
    { value: 'types', label: 'File types', icon: PieChart },
    { value: 'cleanup', label: 'Cleanup', icon: Sparkles, badge: summary ? <CleanupBadge scanId={summary.scanId} /> : undefined },
    { value: 'duplicates', label: 'Duplicates', icon: Copy },
    { value: 'search', label: 'Search', icon: Search },
  ];

  const startScan = () => void scan(root ?? undefined, mode);
  const fast = mode === 'fast' && !!volume?.mftCapable;

  let body;
  if (volumesError) body = <ErrorState title="Could not list drives" error={volumesError} onRetry={() => void loadVolumes()} />;
  else if (!root || (checking && !summary)) body = <Skeleton className="flex-1 rounded-2xl" />;
  else if (scanning && job) body = <ScanProgressView job={job} volumes={volumes} onCancel={() => void cancel()} />;
  else if (!summary)
    body = failure && rootKey(failure.root) === key ? <ErrorState title="The scan failed" error={failure.error} onRetry={startScan} /> : <NeverScanned volume={volume} root={root} mode={mode} onScan={startScan} />;
  else
    body = (
      <>
        <SummaryCard summary={summary} volume={volume} onRefresh={startScan} refreshing={false} />
        <Tabs items={tabs} value={tab} onChange={setTab} className="mt-1" />
        <div key={`${summary.scanId}-${tab}`} className="flex min-h-0 flex-1 flex-col">
          {tab === 'explorer' && <ExplorerTab scanId={summary.scanId} />}
          {tab === 'largest' && <LargestTab scanId={summary.scanId} />}
          {tab === 'types' && <TypesTab scanId={summary.scanId} />}
          {tab === 'cleanup' && <CleanupTab scanId={summary.scanId} />}
          {tab === 'duplicates' && <DuplicatesTab scanId={summary.scanId} />}
          {tab === 'search' && <SearchTab scanId={summary.scanId} />}
        </div>
      </>
    );

  return (
    <Page
      title="Storage"
      subtitle="See what fills your drives, then clean up safely."
      scroll={false}
      actions={
        <>
          {otherJob && job && <Badge tone="accent">Scanning {job.root}…</Badge>}
          <ModeToggle volume={volume} mode={mode} onChange={setMode} />
          <Button variant="primary" icon={fast ? Zap : ScanLine} onClick={startScan} disabled={!root || !!job}>
            {summary ? 'Rescan' : 'Scan'}
          </Button>
        </>
      }
      headerExtra={
        <div className="flex flex-wrap items-center gap-x-5 gap-y-2.5">
          <DrivePicker volumes={volumes} root={root} loaded={loaded} onSelect={(r) => void select(r)} />
          <ModeHint volume={volume} mode={mode} />
        </div>
      }
      className="gap-3"
    >
      {body}
    </Page>
  );
}
