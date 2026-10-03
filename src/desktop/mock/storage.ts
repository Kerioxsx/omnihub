// Storage commands of the mock backend.

import type {
  ChildrenPage,
  CleanupCategory,
  DeleteResult,
  DupeGroup,
  DupeJobProgress,
  DupeOptions,
  JobProgress,
  PathedNode,
  Risk,
  ScanMethod,
  ScanRequest,
  ScanSummary,
  SearchQuery,
  SearchResult,
  SortKey,
  Suggestion,
} from '@shared/types';
import { emit } from './bus';
import { appInfo, audit, settings } from './core';
import { TOKENS, driveTree, volumeFor } from './drives';
import { DAY, NOW, hashString } from './rng';
import { FLAG_CLOUD, FLAG_SYSTEM, type FakeTree, extensionOf } from './tree';

interface Scan {
  scanId: string;
  root: string;
  tree: FakeTree;
  rootId: number;
  method: ScanMethod;
  scannedAt: number;
  durationMs: number;
  fromCache: boolean;
  errors: number;
}

const scans = new Map<string, Scan>();

function normRoot(root: string): string {
  const r = root.replace(/\//g, '\\');
  return /^[a-z]:\\?$/i.test(r) ? `${r.slice(0, 2).toUpperCase()}\\` : r.replace(/\\+$/, '');
}

function scanIdFor(root: string): string {
  return normRoot(root).toLowerCase();
}

function getScan(scanId: string): Scan {
  const s = scans.get(scanId);
  if (!s) throw new Error(`scan ${scanId} is not loaded`);
  return s;
}

function summaryOf(s: Scan): ScanSummary {
  const n = s.tree.node(s.rootId);
  const vol = s.rootId === 0 ? volumeFor(s.root) : null;
  const nodes = n.files + n.dirs + 1;
  return {
    scanId: s.scanId,
    root: s.root,
    method: s.method,
    scannedAt: s.scannedAt,
    durationMs: s.durationMs,
    files: n.files,
    dirs: n.dirs,
    size: n.size,
    alloc: n.alloc,
    errors: s.errors,
    fromCache: s.fromCache,
    volumeTotal: vol?.total ?? null,
    volumeFree: vol?.free ?? null,
    nodes,
    memoryBytes: nodes * 44,
  };
}

// ---------- scan jobs ----------

interface Job {
  progress: JobProgress;
  timer: ReturnType<typeof setInterval> | null;
}
const jobs = new Map<string, Job>();
let jobSeq = 0;

export function scanStart(req: ScanRequest): string {
  const jobId = `job-${++jobSeq}-${Date.now().toString(36)}`;
  const root = normRoot(req.root);
  const tree = driveTree(root);
  const rootId = tree?.findPath(root) ?? null;
  const progress: JobProgress = { jobId, root, method: '', phase: 'starting', state: 'running', recordsDone: 0, recordsTotal: 0, files: 0, dirs: 0, bytes: 0, errors: 0, current: '', elapsedMs: 0, scanId: null, error: null };
  const job: Job = { progress, timer: null };
  jobs.set(jobId, job);

  if (!tree || rootId === null || !tree.node(rootId).isDir) {
    setTimeout(() => finishFailed(job, `Cannot open ${root}: The system cannot find the path specified. (os error 3)`, false), 450);
    return jobId;
  }

  const vol = volumeFor(root);
  const isVolume = rootId === 0;
  const wantsMft = req.mode === 'fast' || (req.mode === 'auto' && appInfo.elevated);
  const mft = wantsMft && isVolume && !!vol?.mftCapable;
  const elevate = mft && !appInfo.elevated;
  const order = tree.subtree(rootId);
  const target = tree.node(rootId);
  const recordsTotal = Math.round(order.length * 1.07 + 1200);
  const started = performance.now();
  const tElev = elevate ? 1500 : 250;
  const tOpen = tElev + 220;
  const tMain = mft ? tOpen + 1650 : 250 + 3400;
  const tSave = tMain + 260;

  const tick = () => {
    const el = performance.now() - started;
    const p = job.progress;
    p.elapsedMs = Math.round(el);
    if (mft) {
      p.method = el >= tElev ? 'mft' : '';
      if (el < 250) p.phase = 'starting';
      else if (el < tElev) p.phase = 'elevating';
      else if (el < tOpen) p.phase = 'opening';
      else if (el < tMain) {
        p.phase = 'mft';
        const f = (el - tOpen) / (tMain - tOpen);
        p.recordsTotal = recordsTotal;
        p.recordsDone = Math.min(recordsTotal, Math.round(recordsTotal * f));
        p.files = Math.round(target.files * f);
        p.dirs = Math.round(target.dirs * f);
        p.bytes = Math.round(target.size * f);
      } else {
        p.phase = 'saving';
        p.recordsDone = recordsTotal;
        p.files = target.files;
        p.dirs = target.dirs;
        p.bytes = target.size;
      }
    } else {
      p.method = 'walk';
      if (el < 250) p.phase = 'starting';
      else {
        p.phase = 'walk';
        const f = Math.min(1, (el - 250) / (tMain - 250));
        const eased = 1 - Math.pow(1 - f, 1.4);
        p.files = Math.round(target.files * eased);
        p.dirs = Math.round(target.dirs * eased);
        p.bytes = Math.round(target.size * eased);
        const nodeId = order[Math.min(order.length - 1, Math.floor(order.length * eased))];
        p.current = tree.path(nodeId);
        p.errors = Math.floor(eased * 3);
      }
    }
    const doneAt = mft ? tSave : tMain;
    if (el >= doneAt) {
      finishDone(job, {
        scanId: scanIdFor(root),
        root,
        tree,
        rootId,
        method: mft ? 'mft' : 'walk',
        scannedAt: Math.floor(Date.now() / 1000),
        durationMs: Math.round(mft ? el - tElev : el),
        fromCache: false,
        errors: mft ? 0 : 3,
      });
      return;
    }
    emit('storage:progress', { ...p });
  };
  job.timer = setInterval(tick, 200);
  setTimeout(tick, 30);
  return jobId;
}

function stop(job: Job) {
  if (job.timer) clearInterval(job.timer);
  job.timer = null;
}

function finishDone(job: Job, scan: Scan) {
  stop(job);
  scans.set(scan.scanId, scan);
  job.progress.state = 'done';
  job.progress.phase = 'done';
  job.progress.scanId = scan.scanId;
  emit('storage:progress', { ...job.progress });
  emit('storage:done', { jobId: job.progress.jobId, summary: summaryOf(scan) });
}

function finishFailed(job: Job, error: string, cancelled: boolean) {
  stop(job);
  job.progress.state = cancelled ? 'cancelled' : 'failed';
  job.progress.error = error;
  emit('storage:progress', { ...job.progress });
  emit('storage:failed', { jobId: job.progress.jobId, error, cancelled });
}

export function scanProgress(jobId: string): JobProgress | null {
  const j = jobs.get(jobId);
  return j ? { ...j.progress } : null;
}

export function scanCancel(jobId: string): void {
  const j = jobs.get(jobId);
  if (j && j.progress.state === 'running') finishFailed(j, 'Scan cancelled', true);
}

export function scanList(): ScanSummary[] {
  return [...scans.values()].map(summaryOf);
}

export function scanSummary(scanId: string): ScanSummary {
  return summaryOf(getScan(scanId));
}

/** C:\ has an MFT snapshot from two days ago; other drives were never scanned. */
export function openCached(root: string): ScanSummary | null {
  const r = normRoot(root);
  if (r !== 'C:\\') return null;
  const tree = driveTree(r)!;
  const scan: Scan = { scanId: scanIdFor(r), root: r, tree, rootId: 0, method: 'mft', scannedAt: NOW - Math.round(2.15 * DAY), durationMs: 2870, fromCache: true, errors: 0 };
  scans.set(scan.scanId, scan);
  return summaryOf(scan);
}

// ---------- queries ----------

export function children(scanId: string, node: number, sort: SortKey, descending: boolean, offset: number, limit: number): ChildrenPage {
  const s = getScan(scanId);
  const id = node < 0 || node >= s.tree.nodes.length ? s.rootId : node;
  const { items, total } = s.tree.children(id, sort, descending, offset, limit);
  return { node: s.tree.view(id), path: s.tree.path(id), breadcrumbs: s.tree.breadcrumbs(id, s.rootId), items, total };
}

export function treemap(scanId: string, node: number, depth: number, maxItems: number) {
  const s = getScan(scanId);
  return s.tree.treemap(node, depth, maxItems);
}

export function topFiles(scanId: string, node: number, n: number): PathedNode[] {
  const s = getScan(scanId);
  return s.tree.topFiles(node, n).map((v) => ({ ...v, path: s.tree.path(v.id) }));
}

export function extensions(scanId: string, node: number) {
  return getScan(scanId).tree.extensions(node);
}

export function search(scanId: string, q: SearchQuery): SearchResult {
  const s = getScan(scanId);
  const t0 = performance.now();
  const { items, total } = s.tree.search({ ...q, under: q.under ?? s.rootId });
  const out = items.map((v) => ({ ...v, path: s.tree.path(v.id) }));
  return { items: out, total, tookMs: Math.max(1, Math.round(performance.now() - t0)) };
}

export function nodePath(scanId: string, node: number): string {
  return getScan(scanId).tree.path(node);
}

// ---------- cleanup ----------

interface Rule {
  id: string;
  category: CleanupCategory;
  risk: Risk;
  title: string;
  description: string;
  patterns: string[];
  olderThanDays: number | null;
  needsAdmin: boolean;
  actionHint: string | null;
  whole: boolean;
}

const RULES: Rule[] = [
  { id: 'user-temp', category: 'temp', risk: 'safe', title: 'Temporary files', description: 'Files programs left in your Temp folder. Anything untouched for a day is safe to remove.', patterns: ['{TEMP}'], olderThanDays: 1, needsAdmin: false, actionHint: null, whole: false },
  { id: 'windows-temp', category: 'temp', risk: 'safe', title: 'Windows temporary files', description: 'The system-wide Temp folder.', patterns: ['{WINDIR}\\Temp'], olderThanDays: 1, needsAdmin: true, actionHint: null, whole: false },
  {
    id: 'browser-cache',
    category: 'cache',
    risk: 'safe',
    title: 'Browser caches',
    description: 'Cached web pages, scripts and images. Browsers download them again when needed. Close the browser first.',
    patterns: ['{LOCALAPPDATA}\\Google\\Chrome\\User Data\\*\\Cache', '{LOCALAPPDATA}\\Google\\Chrome\\User Data\\*\\Code Cache', '{LOCALAPPDATA}\\Google\\Chrome\\User Data\\*\\GPUCache', '{LOCALAPPDATA}\\Microsoft\\Edge\\User Data\\*\\Cache', '{LOCALAPPDATA}\\Microsoft\\Edge\\User Data\\*\\Code Cache', '{LOCALAPPDATA}\\Mozilla\\Firefox\\Profiles\\*\\cache2'],
    olderThanDays: null,
    needsAdmin: false,
    actionHint: null,
    whole: false,
  },
  {
    id: 'shader-cache',
    category: 'cache',
    risk: 'safe',
    title: 'Graphics shader caches',
    description: 'Compiled shaders from your GPU driver and games. They are rebuilt the next time a game runs (the first launch may stutter briefly).',
    patterns: ['{LOCALAPPDATA}\\D3DSCache', '{LOCALAPPDATA}\\NVIDIA\\DXCache', '{LOCALAPPDATA}\\NVIDIA\\GLCache', '{LOCALAPPDATA}\\AMD\\DxCache'],
    olderThanDays: null,
    needsAdmin: false,
    actionHint: null,
    whole: false,
  },
  { id: 'thumbnails', category: 'cache', risk: 'review', title: 'Thumbnail cache', description: "Explorer's picture previews. Some files stay locked while Explorer is running.", patterns: ['{LOCALAPPDATA}\\Microsoft\\Windows\\Explorer\\thumbcache_*.db', '{LOCALAPPDATA}\\Microsoft\\Windows\\Explorer\\iconcache_*.db'], olderThanDays: null, needsAdmin: false, actionHint: null, whole: true },
  {
    id: 'crash-dumps',
    category: 'crashDumps',
    risk: 'safe',
    title: 'Crash dumps and error reports',
    description: 'Memory dumps and reports written when programs or Windows crashed. Only useful to send to a developer.',
    patterns: ['{LOCALAPPDATA}\\CrashDumps', '{LOCALAPPDATA}\\Microsoft\\Windows\\WER\\ReportArchive', '{PROGRAMDATA}\\Microsoft\\Windows\\WER\\ReportArchive', '{PROGRAMDATA}\\Microsoft\\Windows\\WER\\ReportQueue', '{WINDIR}\\Minidump', '{WINDIR}\\LiveKernelReports'],
    olderThanDays: null,
    needsAdmin: false,
    actionHint: null,
    whole: false,
  },
  { id: 'memory-dump', category: 'crashDumps', risk: 'safe', title: 'Full memory dump', description: 'The last complete memory dump Windows wrote after a blue screen.', patterns: ['{WINDIR}\\MEMORY.DMP'], olderThanDays: null, needsAdmin: true, actionHint: null, whole: true },
  { id: 'windows-update', category: 'system', risk: 'review', title: 'Windows Update downloads', description: 'Update packages that were already installed. Windows downloads them again if it still needs them.', patterns: ['{WINDIR}\\SoftwareDistribution\\Download'], olderThanDays: 7, needsAdmin: true, actionHint: 'cleanmgr /sageset', whole: false },
  { id: 'delivery-optimization', category: 'system', risk: 'safe', title: 'Delivery Optimization cache', description: 'Update pieces Windows keeps to share with other PCs.', patterns: ['{WINDIR}\\ServiceProfiles\\NetworkService\\AppData\\Local\\Microsoft\\Windows\\DeliveryOptimization\\Cache'], olderThanDays: null, needsAdmin: true, actionHint: null, whole: false },
  {
    id: 'dev-caches',
    category: 'developer',
    risk: 'review',
    title: 'Developer package caches',
    description: 'Downloaded packages kept by npm, pip, Yarn, Cargo, Gradle and Go. Builds download them again when needed.',
    patterns: ['{LOCALAPPDATA}\\npm-cache', '{LOCALAPPDATA}\\pip\\cache', '{LOCALAPPDATA}\\Yarn\\Cache', '{USERPROFILE}\\.cargo\\registry\\cache', '{USERPROFILE}\\.gradle\\caches', '{LOCALAPPDATA}\\go-build'],
    olderThanDays: null,
    needsAdmin: false,
    actionHint: null,
    whole: false,
  },
  { id: 'windows-old', category: 'system', risk: 'info', title: 'Previous Windows installation', description: 'Windows.old lets you roll back a feature update. Remove it with Disk Cleanup → Clean up system files → Previous Windows installation(s).', patterns: ['{SYSTEMDRIVE}\\Windows.old'], olderThanDays: null, needsAdmin: true, actionHint: 'cleanmgr', whole: true },
  { id: 'hibernation', category: 'system', risk: 'info', title: 'Hibernation file', description: 'Reserved for hibernate and Fast Startup. Running `powercfg /h off` as administrator removes it (and disables both).', patterns: ['{SYSTEMDRIVE}\\hiberfil.sys'], olderThanDays: null, needsAdmin: true, actionHint: 'powercfg /h off', whole: true },
  { id: 'pagefile', category: 'system', risk: 'info', title: 'Page file', description: 'Virtual memory managed by Windows. Adjust it in System → Advanced system settings → Performance → Virtual memory.', patterns: ['{SYSTEMDRIVE}\\pagefile.sys', '{SYSTEMDRIVE}\\swapfile.sys'], olderThanDays: null, needsAdmin: true, actionHint: 'SystemPropertiesPerformance', whole: true },
];

const expand = (p: string) => p.replace(/\{(\w+)\}/g, (_, k: string) => TOKENS[k] ?? `{${k}}`);
const RISK_ORDER: Record<Risk, number> = { safe: 0, review: 1, info: 2 };

function build(s: Scan, rule: Omit<Rule, 'patterns' | 'olderThanDays' | 'whole'>, targets: number[], keepPaths = true): Suggestion | null {
  const t = s.tree;
  const inScope = [...new Set(targets)].filter((id) => t.contains(s.rootId, id)).sort((a, b) => a - b);
  const kept: number[] = [];
  for (const id of inScope) if (!kept.some((k) => t.contains(k, id))) kept.push(id);
  let size = 0;
  let files = 0;
  for (const id of kept) {
    size += t.node(id).size;
    files += t.node(id).files;
  }
  if (size === 0) return null;
  kept.sort((a, b) => t.node(b).size - t.node(a).size);
  return {
    id: rule.id,
    category: rule.category,
    title: rule.title,
    description: rule.description,
    risk: rule.risk,
    size,
    files,
    items: kept.slice(0, 50).map((id) => {
      const n = t.node(id);
      return { id, path: t.path(id), size: n.size, modified: n.modified, isDir: n.isDir };
    }),
    paths: keepPaths ? kept.map((id) => t.path(id)) : [],
    needsAdmin: rule.needsAdmin,
    actionHint: rule.actionHint,
  };
}

export function cleanup(scanId: string): Suggestion[] {
  const s = getScan(scanId);
  const t = s.tree;
  if (!s.root.toUpperCase().startsWith('C:')) {
    const large = largeOld(s);
    return large ? [large] : [];
  }
  const now = Math.floor(Date.now() / 1000);
  const out: Suggestion[] = [];
  for (const rule of RULES) {
    let targets: number[] = [];
    for (const pat of rule.patterns) {
      for (const id of t.resolvePattern(expand(pat))) {
        if (rule.whole) targets.push(id);
        else targets.push(...t.liveChildren(id).map((c) => c.id));
      }
    }
    if (rule.olderThanDays != null) {
      const cutoff = now - rule.olderThanDays * DAY;
      targets = targets.filter((id) => t.node(id).modified <= cutoff);
    }
    const sug = build(s, rule, targets);
    if (sug) out.push(sug);
  }
  const bin = build(s, { id: 'recycle-bin', category: 'recycleBin', risk: 'safe', title: 'Recycle Bin', description: 'Files you already deleted. Emptying the Recycle Bin removes them for good.', needsAdmin: false, actionHint: 'empty-recycle-bin' }, t.resolvePattern('C:\\$Recycle.Bin'), false);
  if (bin) out.push(bin);
  const dl = t.resolvePattern(`${TOKENS.USERPROFILE}\\Downloads`)[0];
  if (dl !== undefined) {
    const cutoff = now - settings.storage.cleanup.installerAgeDays * DAY;
    const exts = ['exe', 'msi', 'msix', 'appx', 'iso', 'img', 'zip', '7z', 'rar', 'dmg', 'apk'];
    const ids = t
      .filesUnder(dl)
      .filter((n) => n.size >= 10 * 1024 * 1024 && n.modified <= cutoff && !(n.flags & FLAG_CLOUD) && exts.includes(extensionOf(n.name) ?? ''))
      .map((n) => n.id);
    const sug = build(s, { id: 'old-downloads', category: 'downloads', risk: 'review', title: 'Old installers and archives in Downloads', description: 'Setup programs, disk images and archives you downloaded more than a month ago. Usually already installed or extracted.', needsAdmin: false, actionHint: null }, ids);
    if (sug) out.push(sug);
  }
  const large = largeOld(s);
  if (large) out.push(large);
  out.sort((a, b) => RISK_ORDER[a.risk] - RISK_ORDER[b.risk] || b.size - a.size);
  return out;
}

function largeOld(s: Scan): Suggestion | null {
  const t = s.tree;
  const { largeFileMin, largeFileAgeDays } = settings.storage.cleanup;
  const cutoff = Math.floor(Date.now() / 1000) - largeFileAgeDays * DAY;
  const protectedRoots = ['WINDIR', 'PROGRAMFILES', 'PROGRAMFILESX86'].flatMap((k) => t.resolvePattern(TOKENS[k])).concat(t.resolvePattern('C:\\ProgramData\\Microsoft'), t.resolvePattern('C:\\System Volume Information'), t.resolvePattern('C:\\Recovery'));
  const ids = t
    .filesUnder(s.rootId)
    .filter((n) => n.size >= largeFileMin && n.modified <= cutoff && !(n.flags & (FLAG_SYSTEM | FLAG_CLOUD)) && !n.name.startsWith('$') && !protectedRoots.some((p) => t.contains(p, n.id)))
    .sort((a, b) => b.size - a.size)
    .slice(0, 200)
    .map((n) => n.id);
  const sizeLabel = largeFileMin >= 1 << 30 ? `${(largeFileMin / (1 << 30)).toFixed(0)} GB` : `${(largeFileMin / (1 << 20)).toFixed(0)} MB`;
  return build(s, { id: 'large-old', category: 'largeOld', risk: 'review', title: 'Large files you have not touched in a while', description: `Files over ${sizeLabel} not modified for ${largeFileAgeDays} days. Move them to an external drive or delete what you no longer need.`, needsAdmin: false, actionHint: null }, ids);
}

// ---------- delete ----------

export function deletePaths(paths: string[], permanent: boolean): DeleteResult[] {
  const results: DeleteResult[] = paths.map((path) => {
    const tree = driveTree(path);
    const id = tree?.findPath(path) ?? null;
    if (!tree || id === null || id === 0) return { path, ok: false, error: 'The system cannot find the file specified. (os error 2)' };
    const lower = path.toLowerCase();
    if (lower.includes('thumbcache_') && lower.endsWith('.db') && hashString(path) % 3 === 0) return { path, ok: false, error: 'The process cannot access the file because it is being used by another process. (os error 32)' };
    if (!appInfo.elevated && lower.startsWith('c:\\windows\\') && !lower.startsWith('c:\\windows\\temp\\') && hashString(path) % 4 === 0) return { path, ok: false, error: 'Access is denied. (os error 5)' };
    tree.markDeleted(id);
    return { path, ok: true, error: null };
  });
  const count = results.filter((r) => r.ok).length;
  emit('storage:deleted', { count, results });
  audit('desktop', permanent ? 'storage.delete-permanent' : 'storage.delete', `${count} of ${paths.length} item(s) ${permanent ? 'deleted permanently' : 'moved to the Recycle Bin'}`, count === paths.length);
  return results;
}

export function emptyRecycleBin(): void {
  for (const root of ['C:\\$Recycle.Bin', 'D:\\$RECYCLE.BIN']) {
    const tree = driveTree(root);
    const id = tree?.findPath(root);
    if (tree && id != null) for (const c of tree.liveChildren(id)) tree.markDeleted(c.id);
  }
  audit('desktop', 'storage.empty-recycle-bin', 'Recycle Bin emptied');
}

// ---------- duplicates ----------

interface DupeJob {
  progress: DupeJobProgress;
  groups: DupeGroup[];
  timer: ReturnType<typeof setInterval> | null;
}
const dupeJobs = new Map<string, DupeJob>();

export function dupesStart(scanId: string, opts: DupeOptions): string {
  const s = getScan(scanId);
  const t = s.tree;
  const under = opts.under != null && opts.under < t.nodes.length ? opts.under : s.rootId;
  const min = Math.max(1, opts.minSize ?? 1024 * 1024);
  const bySize = new Map<number, number[]>();
  for (const f of t.filesUnder(under)) {
    if (f.size < min || f.flags & FLAG_CLOUD) continue;
    const list = bySize.get(f.size);
    if (list) list.push(f.id);
    else bySize.set(f.size, [f.id]);
  }
  const groups: DupeGroup[] = [];
  let candidates = 0;
  let candidateBytes = 0;
  for (const [size, ids] of bySize) {
    if (ids.length < 2) continue;
    candidates += ids.length;
    candidateBytes += size * ids.length;
    const files = ids.map((id) => ({ id, path: t.path(id), modified: t.node(id).modified })).sort((a, b) => a.modified - b.modified);
    groups.push({ hash: (hashString(String(size)).toString(16) + hashString(`${size}x`).toString(16)).padEnd(16, '0'), size, wasted: size * (ids.length - 1), files });
  }
  groups.sort((a, b) => b.wasted - a.wasted);
  const jobId = `dupes-${Date.now().toString(36)}`;
  const job: DupeJob = { progress: { jobId, phase: 1, filesDone: 0, filesTotal: candidates, bytesDone: 0, bytesTotal: candidateBytes, done: false }, groups: groups.slice(0, opts.maxGroups ?? 500), timer: null };
  dupeJobs.set(jobId, job);
  const started = performance.now();
  job.timer = setInterval(() => {
    const el = performance.now() - started;
    const p = job.progress;
    if (el < 1100) {
      p.phase = 1;
      p.filesDone = Math.round(candidates * (el / 1100));
    } else if (el < 2600) {
      p.phase = 2;
      p.filesDone = candidates;
      p.bytesDone = Math.round(candidateBytes * ((el - 1100) / 1500));
    } else {
      p.phase = 3;
      p.bytesDone = candidateBytes;
      p.done = true;
      if (job.timer) clearInterval(job.timer);
      emit('storage:dupes-done', { jobId, groups: job.groups.length, wasted: job.groups.reduce((a, g) => a + g.wasted, 0) });
    }
  }, 150);
  return jobId;
}

export function dupesProgress(jobId: string): DupeJobProgress | null {
  const j = dupeJobs.get(jobId);
  return j ? { ...j.progress } : null;
}

export function dupesResult(jobId: string): DupeGroup[] | null {
  const j = dupeJobs.get(jobId);
  if (!j || !j.progress.done) return null;
  // Drop files deleted since the job finished.
  return j.groups
    .map((g) => ({ ...g, files: g.files.filter((f) => driveTree(f.path)?.findPath(f.path) != null) }))
    .filter((g) => g.files.length > 1)
    .map((g) => ({ ...g, wasted: g.size * (g.files.length - 1) }));
}

export function dupesCancel(jobId: string): void {
  const j = dupeJobs.get(jobId);
  if (j?.timer) clearInterval(j.timer);
  dupeJobs.delete(jobId);
}
