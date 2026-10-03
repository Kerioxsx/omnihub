// Types shared by the desktop UI and the phone app. They mirror the Rust
// structs in crates/omnihub-core (serde camelCase); keep the two in sync.

// ---------- storage ----------

export type DriveKind = 'fixed' | 'removable' | 'network' | 'optical' | 'ram' | 'unknown';

export interface VolumeInfo {
  root: string;
  label: string;
  fileSystem: string;
  kind: DriveKind;
  total: number;
  free: number;
  clusterSize: number;
  serial: number | null;
  mftCapable: boolean;
}

export type ScanMode = 'auto' | 'fast' | 'standard';
export type ScanMethod = 'mft' | 'mftIncremental' | 'walk';

export interface ScanRequest {
  root: string;
  mode: ScanMode;
  image?: string | null;
  exclude?: string[];
}

export type JobState = 'running' | 'done' | 'failed' | 'cancelled';

export interface JobProgress {
  jobId: string;
  root: string;
  /** "mft" or "walk" once decided */
  method: string;
  /** starting | elevating | opening | journal | refresh | mft | saving | loading | walk */
  phase: string;
  state: JobState;
  recordsDone: number;
  recordsTotal: number;
  files: number;
  dirs: number;
  bytes: number;
  errors: number;
  current: string;
  elapsedMs: number;
  scanId: string | null;
  error: string | null;
}

export interface ScanSummary {
  scanId: string;
  root: string;
  method: ScanMethod;
  scannedAt: number;
  durationMs: number;
  files: number;
  dirs: number;
  size: number;
  alloc: number;
  errors: number;
  fromCache: boolean;
  volumeTotal: number | null;
  volumeFree: number | null;
  nodes: number;
  memoryBytes: number;
}

export interface NodeView {
  id: number;
  name: string;
  isDir: boolean;
  size: number;
  alloc: number;
  files: number;
  dirs: number;
  /** Unix seconds */
  modified: number;
  hidden: boolean;
  system: boolean;
  reparse: boolean;
  cloud: boolean;
  isVirtual: boolean;
  children: number;
  /** share of the parent's size, 0..1 */
  fraction: number;
}

export interface PathedNode extends NodeView {
  path: string;
}

export interface Crumb {
  id: number;
  name: string;
}

export interface ChildrenPage {
  node: NodeView;
  path: string;
  breadcrumbs: Crumb[];
  items: NodeView[];
  total: number;
}

export type SortKey = 'size' | 'alloc' | 'name' | 'modified' | 'files';

export interface TreemapItem {
  /** 4294967295 for the folded "N smaller items" entry */
  id: number;
  name: string;
  size: number;
  isDir: boolean;
  ext: string | null;
  folded: number;
  children: TreemapItem[];
}

export interface ExtensionStat {
  ext: string;
  count: number;
  size: number;
  alloc: number;
}

export interface SearchQuery {
  text: string;
  under?: number | null;
  minSize?: number | null;
  maxSize?: number | null;
  modifiedBefore?: number | null;
  modifiedAfter?: number | null;
  extensions?: string[];
  filesOnly?: boolean;
  dirsOnly?: boolean;
  limit?: number | null;
  sort?: SortKey;
}

export interface SearchResult {
  items: PathedNode[];
  total: number;
  tookMs: number;
}

export type CleanupCategory = 'temp' | 'cache' | 'crashDumps' | 'downloads' | 'largeOld' | 'recycleBin' | 'developer' | 'system';
export type Risk = 'safe' | 'review' | 'info';

export interface SuggestionItem {
  id: number;
  path: string;
  size: number;
  modified: number;
  isDir: boolean;
}

export interface Suggestion {
  id: string;
  category: CleanupCategory;
  title: string;
  description: string;
  risk: Risk;
  size: number;
  files: number;
  items: SuggestionItem[];
  paths: string[];
  needsAdmin: boolean;
  /** e.g. "empty-recycle-bin", "cleanmgr", "powercfg /h off" */
  actionHint: string | null;
}

export interface DeleteResult {
  path: string;
  ok: boolean;
  error: string | null;
}

export interface DupeOptions {
  under?: number;
  minSize?: number;
  maxGroups?: number;
}

export interface DupeFile {
  id: number;
  path: string;
  modified: number;
}

export interface DupeGroup {
  hash: string;
  size: number;
  wasted: number;
  files: DupeFile[];
}

export interface DupeJobProgress {
  jobId: string;
  /** 1 = quick hash, 2 = full hash, 3 = done */
  phase: number;
  filesDone: number;
  filesTotal: number;
  bytesDone: number;
  bytesTotal: number;
  done: boolean;
}

// ---------- apps & screenshots ----------

export type AppSource = 'desktop' | 'store' | 'startMenu';

export interface AppInfo {
  id: string;
  name: string;
  publisher: string;
  version: string;
  source: AppSource;
  installLocation: string | null;
  installDate: number | null;
  size: number | null;
  sizeFromScan: boolean;
  aumid: string | null;
  launchable: boolean;
  uninstallable: boolean;
}

export interface Screenshot {
  id: string;
  path: string;
  created: number;
  width: number;
  height: number;
  bytes: number;
  appExe: string | null;
  appTitle: string | null;
  tags: string[];
  note: string;
  favorite: boolean;
  exists: boolean;
}

export type CaptureKind = 'screen' | 'allScreens' | 'window';

export interface ShotFilter {
  query?: string;
  tag?: string | null;
  app?: string | null;
  favorites?: boolean;
  limit?: number | null;
}

export interface PendingRegion {
  id: string;
  x: number;
  y: number;
  width: number;
  height: number;
  scale: number;
  /** data: URL of the frozen screen */
  image: string;
}

export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

// ---------- notes ----------

export type NoteKind = 'note' | 'idea';

export interface Note {
  id: string;
  kind: NoteKind;
  title: string;
  body: string;
  tags: string[];
  pinned: boolean;
  color: string | null;
  created: number;
  updated: number;
  exportedPath: string | null;
  exportedAt: number | null;
}

export interface NoteInput {
  id?: string | null;
  kind: NoteKind;
  title: string;
  body: string;
  tags: string[];
  pinned: boolean;
  color?: string | null;
}

export interface NoteFilter {
  kind?: NoteKind | null;
  query?: string;
  tag?: string | null;
  limit?: number | null;
}

export interface FolderFile {
  name: string;
  path: string;
  size: number;
  modified: number;
  isDir: boolean;
  fromOmnihub: boolean;
  preview: string;
}

// ---------- vault ----------

export type EntryKind = 'login' | 'email' | 'note' | 'card' | 'wifi' | 'other';

export interface VaultStatus {
  exists: boolean;
  unlocked: boolean;
  entries: number;
  dpapi: boolean;
  helloAvailable: boolean;
  helloEnabled: boolean;
  autoLockMinutes: number;
  locksIn: number | null;
  retryAfter: number;
}

export interface EntrySummary {
  id: string;
  kind: EntryKind;
  title: string;
  username: string;
  email: string;
  url: string;
  tags: string[];
  favorite: boolean;
  updated: number;
  hasPassword: boolean;
  hasNotes: boolean;
  primaryAccount: boolean;
  passwordScore: number;
}

export interface Entry {
  id: string;
  kind: EntryKind;
  title: string;
  username: string;
  email: string;
  password: string;
  url: string;
  notes: string;
  tags: string[];
  favorite: boolean;
  created: number;
  updated: number;
  passwordChanged: number;
}

export interface EntryInput {
  id?: string | null;
  kind: EntryKind;
  title: string;
  username: string;
  email: string;
  /** null/undefined keeps the current password when editing */
  password?: string | null;
  url: string;
  notes?: string | null;
  tags: string[];
  favorite: boolean;
}

export interface GeneratorOptions {
  length: number;
  lowercase: boolean;
  uppercase: boolean;
  digits: boolean;
  symbols: boolean;
  avoidAmbiguous: boolean;
}

export interface Strength {
  score: 0 | 1 | 2 | 3 | 4;
  bits: number;
  label: string;
  feedback: string[];
}

// ---------- phone companion ----------

export type Bind = 'lan' | 'localhost';
export type BrowseScope = 'userFolders' | 'allDrives' | 'custom';

export interface MonitorInfo {
  index: number;
  name: string;
  x: number;
  y: number;
  width: number;
  height: number;
  primary: boolean;
}

export interface ViewerInfo {
  id: string;
  device: string;
  monitor: MonitorInfo | null;
  since: number;
  controlling: boolean;
}

export interface ServerStatus {
  running: boolean;
  port: number;
  tls: boolean;
  bind: Bind;
  urls: string[];
  fingerprint: string | null;
  error: string | null;
  viewers: ViewerInfo[];
  pairingOpen: boolean;
}

export interface PairingInfo {
  pin: string;
  secret: string;
  expiresAt: number;
  urls: string[];
  /** SVG markup */
  qrSvg: string;
  fingerprint: string | null;
}

export interface Device {
  id: string;
  name: string;
  userAgent: string;
  created: number;
  lastSeen: number;
  lastIp: string;
  revoked: boolean;
}

export interface InboxItem {
  id: string;
  name: string;
  size: number;
  created: number;
  deviceId: string | null;
}

export type PowerAction = 'shutdown' | 'restart' | 'sleep' | 'hibernate' | 'lock' | 'signOut' | 'displayOff';

export interface PendingPower {
  id: string;
  action: PowerAction;
  label: string;
  /** Unix seconds when it fires */
  at: number;
  requestedBy: string;
}

export interface AuditEntry {
  id: number;
  at: number;
  actor: string;
  action: string;
  detail: string;
  ok: boolean;
}

/** Payload of transfer:started / transfer:progress / transfer:done events. */
export interface TransferEvent {
  id: string;
  direction: 'upload' | 'download';
  name: string;
  size?: number;
  done?: number;
  path?: string;
  sha256?: string;
  device?: string;
}

// ---------- screen sharing ----------

export interface Preset {
  id: string;
  label: string;
  maxWidth: number;
  quality: number;
  fps: number;
}

export interface AdbDevice {
  serial: string;
  state: string;
  model: string;
  wireless: boolean;
}

export interface ScrcpyStatus {
  found: boolean;
  path: string | null;
  version: string | null;
  adb: string | null;
  devices: AdbDevice[];
  running: boolean;
  installHint: string;
}

export interface ScrcpyOptions {
  serial?: string | null;
  /** lowLatency | quality | battery | custom */
  preset: string;
  maxSize?: number | null;
  bitrateMbps?: number | null;
  maxFps?: number | null;
  codec?: 'h264' | 'h265' | 'av1' | null;
  audio: boolean;
  turnScreenOff: boolean;
  stayAwake: boolean;
  fullscreen: boolean;
  alwaysOnTop: boolean;
  control: boolean;
}

export interface SunshineStatus {
  installed: boolean;
  path: string | null;
  running: boolean;
  webUi: string;
}

// ---------- settings & app ----------

export type Theme = 'system' | 'dark' | 'light';

export interface Settings {
  general: {
    theme: Theme;
    accent: string;
    reducedMotion: boolean;
    launchAtLogin: boolean;
    startMinimized: boolean;
    closeToTray: boolean;
    onboarded: boolean;
  };
  storage: {
    defaultMode: ScanMode;
    exclude: string[];
    cleanup: { largeFileMin: number; largeFileAgeDays: number; installerAgeDays: number };
    showHidden: boolean;
    sizeMetric: 'size' | 'alloc';
  };
  notes: {
    claudeFolder: string | null;
    sidecarJson: boolean;
    autoExportIdeas: boolean;
    indexFile: boolean;
  };
  vault: {
    autoLockMinutes: number;
    clipboardClearSeconds: number;
    lockOnSessionLock: boolean;
    allowPhone: boolean;
    helloEnabled: boolean;
  };
  remote: {
    enabled: boolean;
    port: number;
    bind: Bind;
    tls: boolean;
    allowTailscale: boolean;
    browseScope: BrowseScope;
    customRoots: string[];
    allowUploads: boolean;
    allowPower: boolean;
    powerDelaySeconds: number;
    allowScreen: boolean;
    allowControl: boolean;
    allowAppLaunch: boolean;
    allowNotes: boolean;
    incomingDir: string | null;
    deviceName: string;
  };
  screenshots: {
    dir: string | null;
    format: 'png' | 'jpeg';
    hotkeyRegion: string;
    hotkeyFull: string;
    hotkeyWindow: string;
    copyToClipboard: boolean;
  };
  screen: {
    preset: string;
    maxFps: number;
    scrcpyPath: string | null;
    sunshinePath: string | null;
  };
}

/** Deep partial used for settings patches (JSON merge patch). */
export type DeepPartial<T> = { [K in keyof T]?: T[K] extends object ? (T[K] extends unknown[] ? T[K] : DeepPartial<T[K]>) : T[K] };

export interface AppInfoDetails {
  version: string;
  elevated: boolean;
  platform: string;
  hostname: string;
  dpapi: boolean;
  dataDir: string;
  configDir: string;
  cacheDir: string;
  screenshotDir: string;
  incomingDir: string;
}

export interface SystemStats {
  cpu: number;
  memory: { used: number; total: number };
  uptime: number;
  os: string;
}
