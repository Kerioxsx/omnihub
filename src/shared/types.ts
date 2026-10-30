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

export type ExplorerView = 'split' | 'list' | 'grid' | 'treemap' | 'sunburst';

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
  /** Leave out hidden and system items (the desktop also applies Settings → Storage → Show hidden). */
  excludeHidden?: boolean;
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
  /** text was read from it (Copy text); it is then searchable */
  hasText: boolean;
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
  /** reminder time (Unix seconds), null = none */
  remindAt: number | null;
  /** the reminder was shown */
  reminded: boolean;
  /** locked with the vault password: `body` is empty here (read it with notes.readLocked) */
  locked: boolean;
}

export interface NoteInput {
  id?: string | null;
  kind: NoteKind;
  title: string;
  body: string;
  tags: string[];
  pinned: boolean;
  color?: string | null;
  /** keep (or make) it locked; needs the vault unlocked */
  locked?: boolean;
}

/** A file hidden in the vault (decrypted from its encrypted header). */
export interface HiddenFile {
  id: string;
  name: string;
  size: number;
  /** the original file's last change (Unix seconds) */
  modified: number;
  hiddenAt: number;
  /** where it came from: Unhide puts it back there */
  folder: string;
}

export interface HiddenResult {
  file: HiddenFile;
  /** the original couldn't be removed (open in another app) */
  originalLeft: boolean;
}

/** `hidden:progress` */
export interface HiddenProgress {
  job: string;
  name: string;
  done: number;
  total: number;
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
  /** A 2FA (TOTP) secret is stored. */
  hasTotp: boolean;
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
  /** 2FA secret (otpauth:// link or base32 key), empty when none. */
  totp: string;
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
  /** null/undefined keeps the current 2FA secret; "" removes it */
  totp?: string | null;
}

export interface HealthItem {
  id: string;
  title: string;
  username: string;
  url: string;
  /** strength 0–4 (weak), days since changed (old), or times seen in breaches */
  detail: number;
}

export interface HealthReport {
  checked: number;
  /** 0–100 */
  score: number;
  weak: HealthItem[];
  reused: HealthItem[][];
  old: HealthItem[];
  missingTwoFactor: HealthItem[];
}

export interface TotpCode {
  code: string;
  /** seconds until the code changes */
  remaining: number;
}

// ---------- browser autofill ----------

export interface BrowserClient {
  id: string;
  name: string;
  created: number;
  lastSeen: number;
}

export interface BrowserPairRequest {
  id: string;
  name: string;
  code: string;
  created: number;
}

export interface BrowserRegistration {
  browser: string;
  registered: boolean;
}

export interface BrowserStatus {
  enabled: boolean;
  listening: boolean;
  host: string | null;
  browsers: BrowserRegistration[];
  clients: BrowserClient[];
  pending: BrowserPairRequest | null;
  extensionDir: string | null;
  extensionId: string;
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
  /** Devices that reached the server in the last 15 minutes, newest first. */
  visitors: Visit[];
}

export interface Visit {
  ip: string;
  at: number;
  userAgent: string;
  /** False when refused because the address is not on the local network. */
  allowed: boolean;
}

export interface LanAddress {
  interface: string;
  ip: string;
  /** The Wi-Fi/Ethernet address Windows uses for its default route. */
  primary: boolean;
  /** Hyper-V, WSL, VirtualBox, VPN… — usually not reachable from a phone. */
  virtualAdapter: boolean;
}

export interface PairingInfo {
  pin: string;
  secret: string;
  expiresAt: number;
  urls: string[];
  /** SVG markup */
  qrSvg: string;
  /** One QR code per entry in `urls`. */
  qrSvgs: string[];
  addresses: LanAddress[];
  fingerprint: string | null;
}

export type NetworkCategory = 'public' | 'private' | 'domain';

export interface NetworkInfo {
  id: string;
  name: string;
  category: NetworkCategory;
}

export type FirewallVerdict = 'allowed' | 'blocked' | 'noRule' | 'off' | 'unknown';

export interface FirewallRuleInfo {
  name: string;
  allow: boolean;
  enabled: boolean;
  /** Bitmask: 1 domain, 2 private, 4 public. */
  profiles: number;
  protocol: string;
  ports: string;
}

export interface FirewallReport {
  supported: boolean;
  program: string;
  networks: NetworkInfo[];
  activeProfiles: number;
  verdict: FirewallVerdict;
  rules: FirewallRuleInfo[];
  message: string;
}

export interface RemoteDiagnostics {
  running: boolean;
  port: number;
  tls: boolean;
  addresses: LanAddress[];
  firewall: FirewallReport;
  visitors: Visit[];
  /** This PC connecting to itself on each address (empty when off). */
  selfTest: { ip: string; ok: boolean; error: string | null }[];
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

export type InboxKind = 'file' | 'text';

export interface InboxItem {
  id: string;
  name: string;
  size: number;
  created: number;
  deviceId: string | null;
  kind: InboxKind;
  /** The text of a `text` item. */
  text?: string;
  /** Name of the folder a zip was made from. */
  folder?: string;
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

export interface AirPlayOptions {
  /** Name the iPhone shows in Screen Mirroring. */
  name: string;
  quality: '1080p' | '1440p' | '4k';
  fps: number;
  audio: boolean;
  requirePin: boolean;
  lowLatency: boolean;
  fullscreen: boolean;
  /** Software video decoding: slower, avoids graphics-driver crashes. */
  safeMode?: boolean;
}

export interface AirPlayClient {
  name: string;
  model: string;
  deviceId: string;
}

export interface InstallProgress {
  phase: 'download' | 'verify' | 'unpack' | string;
  done: number;
  total: number;
}

export interface AirPlayStatus {
  supported: boolean;
  installed: boolean;
  source: 'addon' | 'custom' | 'found' | null;
  path: string | null;
  version: string | null;
  running: boolean;
  /** An iPhone is mirroring right now (the video window is open). */
  mirroring: boolean;
  name: string;
  pin: string | null;
  client: AirPlayClient | null;
  error: string | null;
  log: string[];
  install: InstallProgress | null;
  downloadUrl: string;
  /** The installed add-on predates a fix this version needs. */
  outdated: boolean;
  /** The Wi-Fi/Ethernet address the receiver announces to iPhones. */
  address: string | null;
  /** What OmniHub saw looking for the receiver the way an iPhone does. */
  check: AirPlayCheck | null;
}

export interface AirPlayCheck {
  ip: string;
  /** It answered an mDNS search for AirPlay receivers on that network. */
  announced: boolean;
  /** The address in that answer is this one. */
  rightAddress: boolean;
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
    /** greeting name; empty = Windows account's first name */
    displayName: string;
    /** interface size in percent (80–150) */
    uiScale: number;
  };
  storage: {
    defaultMode: ScanMode;
    exclude: string[];
    cleanup: { largeFileMin: number; largeFileAgeDays: number; installerAgeDays: number };
    showHidden: boolean;
    sizeMetric: 'size' | 'alloc';
    /** How the Explorer tab shows a folder. */
    explorerView: ExplorerView;
    gridSize: 'sm' | 'md' | 'lg';
    gridPreviews: boolean;
    lowSpaceAlert: boolean;
    lowSpacePercent: number;
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
    /** Brave/Chrome/Edge extension may fill logins. */
    browserAutofill: boolean;
    /** Offer to save logins typed in the browser. */
    browserOfferSave: boolean;
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
    /** Paired phones may put text on this PC's clipboard. */
    allowClipboard: boolean;
    allowTasks: boolean;
    allowServers: boolean;
    /** “Send to → OmniHub (phone)” in Explorer. */
    sendToMenu: boolean;
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
    airplay: AirPlayOptions;
    airplayKeepOnTop: boolean;
    airplayPip: boolean;
    airplayAutoStart: boolean;
    uxplayPath: string | null;
  };
  apps: {
    /** IDs of apps pinned as favourites */
    favorites: string[];
  };
  media: {
    allowPhone: boolean;
    lyricsOnline: boolean;
    eqEnabled: boolean;
    bassDb: number;
    trebleDb: number;
    /** A folder of your own .lrc files, checked before looking online. */
    lrcFolder: string | null;
    /** Sharper covers from the iTunes catalogue (the player's are small). */
    hiresArt: boolean;
  };
  visuals: VisualSettings;
  updates: {
    check: boolean;
    autoInstall: boolean;
  };
  chats: ChatSettings;
}

export interface ChatSettings {
  /** Paired phones may read the chats and send messages. */
  allowPhone: boolean;
  /** Paired phones may run Claude Code and Codex on this PC. */
  phoneAgents: boolean;
  /** Claude Code's permission mode unless a message picks another. */
  claudeMode: ClaudeCodeMode;
  /** Codex's sandbox unless a message picks another. */
  codexSandbox: CodexSandbox;
  /** Capri, the desktop companion who narrates Claude Code: started with OmniHub while on. */
  capri: boolean;
}

// ---------- Aurora: music-reactive light, scene and lyrics ----------

/** `aurora`: light, scene and floating lyrics; `lyrics`: the Apple Music–style view. */
export type PlayerView = 'aurora' | 'lyrics';
export type ColorMode = 'album' | 'duo' | 'multi' | 'single';

/** How strongly each effect plays on the cover, 0–1 (0 = off). */
export interface Effects {
  zoom: number;
  glitch: number;
  split: number;
  fisheye: number;
  ripple: number;
  kaleidoscope: number;
  halftone: number;
  pixelate: number;
  shake: number;
  echo: number;
  scanlines: number;
  duotone: number;
  twist: number;
  bloom: number;
  /** The colours swinging round for a moment on hard beats. */
  colorflash: number;
}

export type SceneStyle = 'visual' | 'fisheye' | 'fisheyeVisual' | 'minimal' | 'ambient';
export type LyricFont = 'display' | 'condensed' | 'heavy' | 'serif' | 'impact' | 'custom';

export interface VisualSettings {
  /** Which release's defaults these settings were last brought up to (the core upgrades them). */
  revision?: number;
  view: PlayerView;
  /** Everything Aurora draws (the lyrics stay). */
  enabled: boolean;
  /** Listen to what the PC plays so the light follows the music. */
  audioReactive: boolean;
  /** 0.2–3 */
  sensitivity: number;
  /** 0–2 */
  bass: number;
  /** 0–1 */
  smoothing: number;
  reducedMotion: boolean;
  /** No bright pulses on beats. */
  noFlashes: boolean;
  /** × animation speed */
  speed: number;
  /** keep the screen on while the player shows Aurora and music plays */
  keepAwake: boolean;
  color: { mode: ColorMode; primary: string; secondary: string; colors: string[]; vividness: number };
  glow: { enabled: boolean; animation: 'music' | 'idle' | 'none'; thickness: number; intensity: number; edge: number; highlight: number; glow: number; glowSize: number; bloom: number; bloomSize: number; radius: number };
  lyrics: {
    /** stack: one word (or line) at a time, huge; lines: the current line and the next ones */
    layout: 'stack' | 'lines';
    emphasis: 'glow' | 'box' | 'color';
    font: LyricFont;
    visible: boolean;
    autoShow: boolean;
    size: number;
    weight: number;
    wordHighlight: boolean;
    /** Lyrics that only time lines: spread the words over each line by syllables (close, not exact). */
    estimateWords: boolean;
    timing: 'auto' | 'line';
    place: 'auto' | 'center' | 'upper' | 'lower';
    offsetX: number;
    offsetY: number;
    lines: number;
    backing: number;
    highlightColor: string | null;
    glow: number;
    transition: number;
    /** 0–1.5: how much the lyrics move with the music (bounce, sway, pop) */
    motion: number;
    /** Emojis beside words they match. */
    emoji: EmojiAmount;
    emojiStyle: EmojiStyle;
    /** 0–1: the sung word splits into coloured copies and jolts on hard beats. */
    glitch: number;
    /** A new word arrives with fading copies trailing behind it. */
    echo: boolean;
  };
  scene: {
    enabled: boolean;
    style: SceneStyle;
    intensity: number;
    speed: number;
    blur: number;
    saturation: number;
    opacity: number;
    artwork: boolean;
    waveform: boolean;
    effects: Effects;
    /** What fills the screen: the music video or the cover (auto), one of them, or the light show. */
    backdrop: Backdrop;
    /** Look the song's music video up on YouTube. */
    musicVideos: boolean;
    /** When YouTube will not embed a video, play its stream directly (yt-dlp, downloaded once). */
    directVideos: boolean;
    /** Match a video whose song starts later to the song, by ear (its soundtrack via yt-dlp). */
    syncVideos: boolean;
    /** Show the music app's own Canvas or video, mirrored from its window. */
    mirrorApp: boolean;
    /** The album's animated cover from Apple Music, when it has one. */
    animatedCovers: boolean;
    /** 'whole': all of the cover, its sides blending into a blurred extension; 'fill': edge to edge, cropped. */
    coverFit: 'whole' | 'fill';
    /** 0–1: how much the full-screen cover drifts and breathes. */
    coverMotion: number;
    /** 0–1: how much the cover or video is darkened behind the lyrics. */
    dim: number;
    /** 0–1: soft light in the song's colours from the corners. */
    corners: number;
    videoFit: VideoFit;
    /** 0 (top) – 1 (bottom): the part of a cropped video that stays. */
    videoFocus: number;
    videoQuality: VideoQuality;
  };
  desktop: { enabled: boolean; lyrics: boolean; display: string; clearTaskbar: boolean; hideFullscreen: boolean; startWithApp: boolean };
}

export interface VisualStatus {
  state: 'off' | 'disabled' | 'starting' | 'listening' | 'unavailable';
  /** What is listened to. */
  source: string | null;
  sampleRate: number;
  error: string | null;
}

/** A monitor the desktop glow can light. */
export interface AmbientDisplay {
  /** Windows' device name (\\.\DISPLAY1) */
  name: string;
  /** "Display 1 · 2560×1440 (main)" */
  label: string;
  width: number;
  height: number;
  scale: number;
  primary: boolean;
}

/** One analysis frame of the PC's sound (`audio:frame`, 60 a second while there is sound). */
export interface AudioFrame {
  /** PC time, Unix ms */
  t: number;
  /** 0–1 loudness (adaptive) */
  level: number;
  low: number;
  mid: number;
  high: number;
  /** 1 on a beat, decaying */
  beat: number;
  flux: number;
  /** 16 log-spaced bands, 40 Hz–16 kHz */
  bands: number[];
  /** sound is playing */
  active: boolean;
  /** beats counted since the track began */
  beats: number;
  bpm: number | null;
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
  /** First name of the signed-in user, for greetings. */
  userName: string;
}

export interface GrowthItem {
  path: string;
  node: number | null;
  before: number;
  after: number;
  delta: number;
  isNew: boolean;
}

export interface GrowthReport {
  /** when the previous scan was made; null = nothing to compare with yet */
  since: number | null;
  totalBefore: number;
  totalAfter: number;
  grew: GrowthItem[];
  shrank: GrowthItem[];
}

export interface WindowInfo {
  id: number;
  title: string;
  app: string;
  width: number;
  height: number;
}

export interface ShareState {
  paused: boolean;
  windowId: number | null;
  /** the shared window, if it still exists */
  window: WindowInfo | null;
}

export type StartupLocation = 'runUser' | 'runMachine' | 'runMachine32' | 'folderUser' | 'folderCommon';

export interface StartupItem {
  id: string;
  name: string;
  displayName: string;
  command: string;
  target: string | null;
  location: StartupLocation;
  enabled: boolean;
  /** changing it asks for administrator approval */
  needsAdmin: boolean;
}

export type ProcessSort = 'cpu' | 'memory' | 'gpu' | 'disk' | 'name';
export type Priority = 'low' | 'belowNormal' | 'normal' | 'aboveNormal' | 'high';

export interface ProcessGroup {
  name: string;
  count: number;
  /** share of the whole CPU, 0–100 */
  cpu: number;
  memory: number;
  /** busiest GPU engine, 0–100 */
  gpu: number;
  gpuMemory: number;
  /** disk reads + writes, bytes/s */
  disk: number;
  exe: string | null;
  pids: number[];
  canEnd: boolean;
  priority: Priority | null;
}

export interface GpuAdapter {
  name: string;
  percent: number;
  memoryUsed: number;
  memoryTotal: number;
}

/** The whole PC and the programs using it. */
export interface Usage {
  cpu: number;
  cpuName: string;
  cores: number;
  memoryUsed: number;
  memoryTotal: number;
  gpuSupported: boolean;
  gpus: GpuAdapter[];
  disk: number;
  processCount: number;
  processes: ProcessGroup[];
}

export interface SystemStats {
  cpu: number;
  memory: { used: number; total: number };
  uptime: number;
  os: string;
}

// ---------- music ----------

export interface MediaState {
  key: string;
  app: string;
  appName: string;
  title: string;
  artist: string;
  album: string;
  durationMs: number;
  /** track position at `positionAt` (PC clock, Unix ms) */
  positionMs: number;
  positionAt: number;
  playing: boolean;
  rate: number;
  positionSource: 'player' | 'estimated';
  canPlayPause: boolean;
  canNext: boolean;
  canPrevious: boolean;
  canSeek: boolean;
  art: string | null;
}

export interface LyricWord {
  ms: number;
  text: string;
}

export interface LyricLine {
  ms: number;
  text: string;
  words?: LyricWord[];
  /** Word timings estimated here, not from the lyrics (desktop only). */
  estimated?: boolean;
}

export type Backdrop = 'auto' | 'video' | 'cover' | 'visual';
export type EmojiAmount = 'off' | 'some' | 'more';
/** 'plain': one colour, the lyrics' own (default); '3d': glossy Fluent 3D; 'system': Windows' emoji font. */
export type EmojiStyle = 'plain' | '3d' | 'system' | 'textures';
export type VideoFit = 'fill' | 'fit';
export type VideoQuality = 'best' | '1080' | '720';

/** A song's music video on YouTube (the artist's own upload). */
export interface MusicVideo {
  id: string;
  title: string;
  channel: string;
  durationMs: number;
  /** "video", or "visualizer": the artist's animated cover. */
  kind: 'video' | 'visualizer';
  uhd: boolean;
  /** As long as the song, so it plays in step with it. */
  synced: boolean;
}

export type VideoStatus = { status: 'found'; videos: MusicVideo[] } | { status: 'searching' | 'none' | 'off' | 'nothingPlaying' };

/** The album's animated cover (Apple Music): an HLS playlist, square. */
export type MotionStatus = { status: 'found'; url: string } | { status: 'searching' | 'none' | 'off' | 'nothingPlaying' };

export type LyricsStatus =
  | { status: 'ready'; lyrics: { lines: LyricLine[]; plain: string | null; instrumental: boolean; source: string } }
  | { status: 'searching' | 'none' | 'off' | 'nothingPlaying' };

export interface AudioInfo {
  volume: { level: number; muted: boolean } | null;
  eq: { status: { available: boolean; hooked: boolean; configDir: string | null }; enabled: boolean; bass: number; treble: number; maxDb: number };
}

export type MediaAction = 'play' | 'pause' | 'toggle' | 'next' | 'previous' | 'seek';

export interface EqStatus {
  /** Equalizer APO is installed */
  available: boolean;
  /** OmniHub's include line is in its config */
  hooked: boolean;
  configDir: string | null;
}

// ---------- games ----------

export type GameKind = 'fortnite' | 'roblox' | 'valorant' | 'cs2' | 'apex' | 'overwatch' | 'rocketLeague' | 'gta5' | 'callOfDuty' | 'league' | 'minecraft' | 'cyberpunk' | 'custom';

export type GameLaunch =
  | { type: 'none' }
  | { type: 'exe'; path: string; args: string }
  | { type: 'url'; url: string }
  | { type: 'steam'; appId: number }
  | { type: 'epic'; app: string }
  | { type: 'riot'; product: string }
  | { type: 'roblox'; placeId: number | null };

export type PowerPlanChoice = 'keep' | 'high' | 'ultimate';

/** competitive: most FPS, also the game's own graphics · quality: nothing that changes the picture · custom: each switch as set. */
export type BoostMode = 'competitive' | 'quality' | 'custom';

export interface GameBoost {
  mode: BoostMode;
  powerPlan: PowerPlanChoice;
  priority: Priority | null;
  closeApps: string[];
  reopenApps: boolean;
  silenceNotifications: boolean;
  gameMode: boolean;
  gpuHighPerformance: boolean;
  fullscreenOptimizationsOff: boolean;
  wifiLowLatency: boolean;
  networkPriority: boolean;
  startHighPriority: boolean;
  /** Games OmniHub knows the settings of: write the fastest in-game settings before each launch. */
  gameSettings: boolean;
  /** Close cloud sync, Windows extras and updaters for the session. */
  closeJunk: boolean;
  /** Browsers, launchers' web views and sync apps at Below normal while playing. */
  lowerBackground: boolean;
  /** Before the game starts, background programs hand unused memory back to Windows. */
  freeMemory: boolean;
  /** Windows' finest timer (0.5 ms) while playing. */
  preciseTimer: boolean;
  /** Windows never slows the game down to save power. */
  fullSpeed: boolean;
  /** Measure FPS while playing (PresentMon). */
  fpsMeter: boolean;
  /** Per game, admin once: the game's NVIDIA driver profile set for speed. */
  nvidiaProfile: boolean;
}

// ---------- the games' own settings, installed games, PC tweaks ----------

export type ConfigGame = 'fortnite' | 'valorant' | 'cs2' | 'apex' | 'overwatch' | 'roblox' | 'minecraft';

export interface FortniteOptions {
  /** 0 = unlimited */
  frameLimit: number;
  performanceMode: boolean;
  lowestQuality: boolean;
  /** 0 near … 3 epic */
  viewDistance: number;
  resolutionScale: number;
  showFps: boolean;
  fullscreen: boolean;
  /** NVIDIA Reflex Low Latency: 0 off, 1 on, 2 on + boost. */
  reflex: number;
  /** Rendering mode; null follows performanceMode. */
  renderMode?: FortniteRenderMode | null;
}

export type FortniteRenderMode = 'performance' | 'dx12' | 'dx11' | 'keep';

export interface CacheClean {
  freedBytes: number;
  files: number;
  /** Files in use, left alone. */
  skipped: number;
}

export interface MinecraftOptions {
  unlimitedFps: boolean;
  fastGraphics: boolean;
  minimalParticles: boolean;
  noClouds: boolean;
  renderDistance: number | null;
  simulationDistance: number | null;
}

/** The switches for games without options of their own. */
export interface ProOptions {
  uncapped: boolean;
  lowestQuality: boolean;
  lowLatency: boolean;
}

export interface ConfigOptions {
  fortnite: FortniteOptions;
  minecraft: MinecraftOptions;
  valorant: ProOptions;
  cs2: ProOptions;
  apex: ProOptions;
  overwatch: ProOptions;
  roblox: ProOptions;
}

export interface ConfigChange {
  label: string;
  key: string;
  from: string | null;
  to: string;
}

export interface ConfigSetting {
  label: string;
  value: string;
  good: boolean;
}

export interface ConfigStatus {
  game: ConfigGame;
  label: string;
  /** Several for games with one file per account. */
  paths: string[];
  found: boolean;
  running: boolean;
  settings: ConfigSetting[];
  pending: ConfigChange[];
  backupAt: number | null;
  note: string | null;
}

export interface GameConfigs {
  options: ConfigOptions;
  games: ConfigStatus[];
}

export interface InstalledGame {
  key: string;
  name: string;
  source: 'epic' | 'steam' | 'riot' | 'roblox' | 'minecraft';
  kind: GameKind;
  launch: GameLaunch;
  process: string;
  exePath: string | null;
  installDir: string | null;
  profileId: string | null;
}

export type TweakId = 'refreshRate' | 'memorySpeed' | 'powerPlan' | 'gameDvr' | 'gameMode' | 'windowedGames' | 'gpuScheduling' | 'preciseTimer' | 'networkThrottling' | 'mouseAcceleration' | 'stickyKeys' | 'memoryIntegrity';

export interface PcTweak {
  id: TweakId;
  title: string;
  description: string;
  impact: 'high' | 'medium' | 'low';
  available: boolean;
  optimized: boolean;
  current: string;
  admin: boolean;
  restart: boolean;
  settingsLink: string | null;
  canUndo: boolean;
}

export interface PcDisplay {
  name: string;
  width: number;
  height: number;
  hz: number;
  maxHz: number;
}

export interface PcStatus {
  tweaks: PcTweak[];
  machine: { cpu: string; cores: number; threads: number; ramGb: number; gpus: string[]; displays: PcDisplay[] };
}

export type RobloxPreset = 'maxFps' | 'balanced' | 'quality' | 'custom';
export type RobloxRenderer = 'auto' | 'd3d11' | 'vulkan' | 'openGl';

export interface RobloxFlags {
  enabled: boolean;
  preset: RobloxPreset;
  renderer: RobloxRenderer;
  msaa: number | null;
  textureQuality: number | null;
  noGrass: boolean;
  graySky: boolean;
  lowDetailDistance: boolean;
  /** Lighting worked out once (voxel lighting stops updating). */
  pauseVoxelizer: boolean;
  qualityLevel: number | null;
  exclusiveFullscreen: boolean;
  ignoreDisplayScaling: boolean;
  custom: Record<string, string | number | boolean>;
  /** custom flags kept but switched off */
  customOff: string[];
  shaders: RobloxShaders;
}

export type ShaderPreset = 'realistic' | 'vivid' | 'cinematic' | 'soft' | 'sharp' | 'custom';

/** Shaders through ReShade, like RoShade or Bloxshade. */
export interface RobloxShaders {
  enabled: boolean;
  preset: ShaderPreset;
  /** 25–150 % */
  strength: number;
}

export interface ShaderStatus {
  /** ReShade's version once downloaded */
  version: string | null;
  /** Roblox folders with OmniHub's ReShade */
  applied: number;
  /** Roblox folders with another tool's dxgi.dll, left alone */
  foreign: string[];
}

export interface KnownFlag {
  name: string;
  /** what Roblox sets it to now */
  value: string | null;
  /** on the allowlist: Roblox still reads it */
  allowed: boolean;
}

export interface GameProfile {
  id: string;
  name: string;
  kind: GameKind;
  launch: GameLaunch;
  process: string;
  exePath: string | null;
  boost: GameBoost;
  pingHost: string | null;
  roblox: RobloxFlags;
  lastPlayed: number | null;
}

export type GamePhase = 'starting' | 'waiting' | 'playing' | 'boosted' | 'restoring' | 'ended';

export interface GameStep {
  id: string;
  label: string;
  status: 'done' | 'skipped' | 'failed' | 'waiting';
  detail: string;
}

export interface GameSession {
  profileId: string;
  name: string;
  mode: BoostMode;
  phase: GamePhase;
  startedAt: number;
  endedAt: number | null;
  launched: boolean;
  steps: GameStep[];
  restored: GameStep[];
  message: string | null;
  /** Frames per second over the session, when the FPS meter ran. */
  fps: FpsSummary | null;
}

// ---------- FPS meter, lag under load ----------

export interface FpsLive {
  fps: number;
  /** 1% low over the last ten seconds. */
  low1: number;
  frameMs: number;
  worstMs: number;
}

export interface FpsSummary {
  frames: number;
  seconds: number;
  avg: number;
  low1: number;
  low01: number;
  /** Frames over 2.5× the average (and 8 ms). */
  hitches: number;
}

export interface FpsStatus {
  installed: boolean;
  allowed: boolean;
  signOutNeeded: boolean;
  supported: boolean;
}

export interface FpsRecord {
  profileId: string;
  name: string;
  mode: BoostMode;
  at: number;
  summary: FpsSummary;
}

export interface FpsOverview {
  status: FpsStatus;
  live: FpsLive | null;
  history: FpsRecord[];
}

export interface LoadTest {
  target: string;
  idleMs: number | null;
  downloadMs: number | null;
  uploadMs: number | null;
  downloadMbps: number | null;
  uploadMbps: number | null;
  extraMs: number | null;
  grade: string | null;
  advice: string[];
  error: string | null;
}

export interface GameState {
  networkPriority: boolean;
  startHighPriority: boolean;
  /** null without an NVIDIA driver. */
  nvidiaProfile: boolean | null;
  onWifi: boolean | null;
  running: boolean;
}

export interface PingTarget {
  id: string;
  label: string;
  host: string;
}

export interface PingResult {
  id: string;
  label: string;
  host: string;
  address: string | null;
  sent: number;
  received: number;
  avgMs: number | null;
  minMs: number | null;
  maxMs: number | null;
  jitterMs: number | null;
  loss: number;
  samples: (number | null)[];
  error: string | null;
}

export interface RobloxInstall {
  found: boolean;
  player: string | null;
  version: string | null;
  versionDirs: string[];
  bootstrapper: string | null;
  running: boolean;
}

// ---------- updates ----------

export interface ReleaseAsset {
  name: string;
  url: string;
  size: number;
}

export interface Release {
  version: string;
  notes: string;
  pageUrl: string;
  publishedAt: string | null;
  setup: ReleaseAsset | null;
  msi: ReleaseAsset | null;
  sumsUrl: string | null;
}

export type UpdateState =
  | { state: 'idle' }
  | { state: 'checking' }
  | { state: 'upToDate'; checkedAt: number }
  | { state: 'available'; release: Release }
  | { state: 'downloading'; version: string; done: number; total: number }
  | { state: 'installing'; version: string }
  | { state: 'failed'; message: string };

export interface UpdateInfo {
  current: string;
  state: UpdateState;
  /** The version that ran before, on the first start after an update. */
  updatedFrom?: string | null;
}

/** yt-dlp and Deno, for videos YouTube will not embed. */
export interface StreamToolStatus {
  installed: boolean;
  installing: boolean;
}

/** The user's own lyric font and emoji textures, kept on this PC. */
export interface AuroraLook {
  /** The font's name (its file name), if one was chosen. */
  font: string | null;
  /** The emoji pack's picture names (file names without the extension). */
  textures: string[];
  /** The emoji pack, when it is a font: its name. */
  emojiFont: string | null;
}

/** What an emoji pack turned out to be. */
/** The music app's own moving picture, mirrored from its window (see media/mirror.rs). */
export interface MirrorStatus {
  state: 'off' | 'looking' | 'mirroring' | 'hidden' | 'none' | 'unavailable';
  kind: 'canvas' | 'video' | null;
  app: string | null;
  width: number;
  height: number;
}

/** How far a music video is from being matched to the song (see media/align.rs). */
export type VideoSync = { state: 'off' } | { state: 'matching'; note: string } | { state: 'matched'; offsetMs: number; score: number } | { state: 'unsure'; note: string };

export type EmojiPack = { kind: 'pictures'; count: number } | { kind: 'font'; name: string };

// ---------- My PC (lighting, fans, screens, specs) ----------

export type LightEffect = 'off' | 'static' | 'breathing' | 'spectrum' | 'wave' | 'gradient' | 'music' | 'temperature' | 'govee';

export interface LightProfile {
  id: string;
  name: string;
  effect: LightEffect;
  /** "#rrggbb". */
  colors: string[];
  /** 0.2 (slow) to 3 (fast). */
  speed: number;
  /** 0–100. */
  brightness: number;
}

export type FanProfile = 'silent' | 'auto' | 'fast';
export type FanRole = 'case' | 'gpu' | 'pump';
export type CoolerKind = 'unknown' | 'air' | 'aio';
export type BoardSize = 'eAtx' | 'atx' | 'microAtx' | 'miniItx';
export type Chassis = 'unknown' | 'desktop' | 'tower' | 'miniTower' | 'smallFormFactor' | 'allInOne' | 'laptop';
export type DeviceRole = 'other' | 'cooler' | 'hub' | 'screen';
export type LitKind = 'motherboard' | 'memory' | 'gpu' | 'cooler' | 'ledStrip' | 'keyboard' | 'mouse' | 'mousemat' | 'headset' | 'headsetStand' | 'gamepad' | 'light' | 'speaker' | 'virtual' | 'storage' | 'case' | 'microphone' | 'accessory' | 'keypad' | 'other';
export type LinkState = 'off' | 'on' | 'missing' | 'error';

export interface RamStick {
  slot: string;
  sizeGb: number;
  speed: number;
  /** The stick's own (standard) speed; EXPO/XMP kits run at it until turned on. */
  rated: number;
  maker: string;
  part: string;
  kind: string;
}

export interface KnownDevice {
  vendorId: number;
  productId: number;
  name: string;
  role: DeviceRole;
  radiatorMm: number | null;
  lcd: boolean;
}

export interface Specs {
  cpu: string;
  cores: number;
  threads: number;
  gpus: string[];
  ram: RamStick[];
  ramGb: number;
  boardMaker: string;
  board: string;
  boardSize: BoardSize;
  system: string;
  chassis: Chassis;
  cooler: CoolerKind;
  coolerName: string;
  radiatorMm: number | null;
  devices: KnownDevice[];
  disks: { name: string; nvme: boolean }[];
  displays: PcDisplay[];
}

export type ScreenShow = { kind: 'off' } | { kind: 'media'; id: string } | { kind: 'stats' } | { kind: 'nowPlaying' } | { kind: 'liquid' };

export interface RigSettings {
  lighting: { enabled: boolean; profile: string; custom: LightProfile[]; port: number; peripherals: boolean; skip: string[] };
  fans: { profile: FanProfile; lhmUrl: string; lhmUser: string; lhmSecret: string; skip: string[]; coolers: boolean; liquidctlPath: string | null };
  screens: { monitor: string; monitorShow: ScreenShow; fill: boolean; lcd: string; lcdShow: ScreenShow; lcdBrightness: number };
  model: { caseColor: 'black' | 'white'; glass: boolean; cooler: CoolerKind | null; radiatorMm: number | null; frontFans: number; topFans: number; rearFans: number; bottomFans: number; ramRgb: boolean; verticalGpu: boolean; caseScreen: boolean };
}

export interface LitDevice {
  key: string;
  name: string;
  kind: LitKind;
  leds: number;
  zones: string[];
  on: boolean;
}

export interface FanControl {
  id: string;
  name: string;
  hardware: string;
  role: FanRole | null;
  percent: number | null;
  skipped: boolean;
}

export interface CoolerReading {
  key: string;
  value: number;
  unit: string;
}

export interface CoolerDevice {
  description: string;
  vendorId: number;
  productId: number;
  address: string;
  driver: string;
  lcd: boolean;
  status: CoolerReading[];
}

export interface RigStatus {
  lighting: LinkState;
  lightingMessage: string | null;
  /** other lighting programs running (they fight over the same lights) */
  lightingRivals: string[];
  devices: LitDevice[];
  openrgbInstalled: string | null;
  fans: LinkState;
  fansMessage: string | null;
  controls: FanControl[];
  /** [name, rpm]. */
  rpm: [string, number][];
  cpuTemp: number | null;
  gpuTemp: number | null;
  coolers: LinkState;
  coolersMessage: string | null;
  coolerDevices: CoolerDevice[];
  liquidTemp: number | null;
  lcdMessage: string | null;
}

export interface RigMedia {
  id: string;
  name: string;
  kind: 'picture' | 'gif' | 'video';
  mime: string;
  bytes: number;
}

export interface RigDisplay {
  name: string;
  label: string;
  width: number;
  height: number;
  scale: number;
  primary: boolean;
}

// ---------- Govee lights ----------

export type GoveeMode = 'color' | 'white' | 'effect' | 'screen' | 'music' | 'scene' | 'deviceMusic';
export type GoveeZone = 'whole' | 'left' | 'right' | 'top' | 'bottom' | 'around' | 'across';
export type MusicStyle = 'pulse' | 'spectrum' | 'flow';

/** What lights show. */
export interface GoveeLook {
  mode: GoveeMode;
  /** color: "#rrggbb" */
  color: string;
  /** white: 2000–9000 K */
  kelvin: number;
  effect: LightEffect;
  colors: string[];
  speed: number;
  /** screen: the monitor's name ('' = the main one) */
  monitor: string;
  smoothness: number;
  saturation: number;
  skipDark: boolean;
  /** screen: 0 = as bright as on screen, 100 = dim colours lifted to full */
  boost: number;
  style: MusicStyle;
  albumColors: boolean;
  sensitivity: number;
  /** scene: each light's own scene of this name */
  scene: string;
  /** deviceMusic: the light's own music mode by name */
  deviceMusic: string;
}

export interface GoveeLightSettings {
  id: string;
  name: string;
  control: boolean;
  follow: boolean;
  look: GoveeLook;
  on: boolean;
  brightness: number;
  /** 0 = what Govee says (or 10) */
  segments: number;
  /** auto: per segment when Govee's account (or the model) says the light streams them, else one colour */
  stream: 'auto' | 'segments' | 'segmentsLite' | 'single';
  zone: GoveeZone;
  reverse: boolean;
  gradient: boolean;
}

export interface GoveePreset {
  id: string;
  name: string;
  look: GoveeLook;
  brightness: number | null;
}

export type TimerAction = 'on' | 'off' | 'preset' | 'brightness';

export interface GoveeTimer {
  id: string;
  name: string;
  enabled: boolean;
  /** "07:30" */
  at: string;
  /** 1 = Monday … 7 = Sunday; empty = every day */
  days: number[];
  action: TimerAction;
  preset: string;
  brightness: number;
  /** device ids; empty = all */
  lights: string[];
  /** minutes to fade in or out */
  fade: number;
}

export interface GoveeSettings {
  enabled: boolean;
  on: boolean;
  brightness: number;
  look: GoveeLook;
  lights: GoveeLightSettings[];
  presets: GoveePreset[];
  preset: string;
  timers: GoveeTimer[];
  /** always empty in the window */
  apiSecret: string;
  manual: string[];
}

export interface GoveeChoice {
  name: string;
  value: unknown;
}

export interface GoveeExtra {
  kind: 'toggle' | 'range' | 'choice';
  type: string;
  instance: string;
  label: string;
  min: number | null;
  max: number | null;
  unit: string | null;
  choices: GoveeChoice[];
  /** volume, speaker, white noise */
  sound: boolean;
}

export interface GoveeLight {
  id: string;
  sku: string;
  name: string;
  ip: string | null;
  lan: boolean;
  cloud: boolean;
  on: boolean | null;
  brightness: number | null;
  color: string | null;
  kelvin: number | null;
  segments: number | null;
  kelvinRange: [number, number] | null;
  scenes: GoveeChoice[];
  diyScenes: GoveeChoice[];
  snapshots: GoveeChoice[];
  musicModes: GoveeChoice[];
  extras: GoveeExtra[];
  values: Record<string, unknown>;
  streaming: boolean;
  /** a colour per segment (else one colour) */
  perSegment: boolean;
  message: string | null;
}

export interface GoveeStatus {
  lan: LinkState;
  lanMessage: string | null;
  cloud: LinkState;
  cloudMessage: string | null;
  hasKey: boolean;
  lights: GoveeLight[];
  screenMessage: string | null;
  musicMessage: string | null;
  palette: string[];
  sleepUntil: number | null;
}

// ---------- Mods (0.10.0) ----------

export type ModEngine = 'unityMono' | 'unityIl2cpp' | 'unreal' | 'love' | 'rage' | 'source' | 'source2' | 'godot' | 'gameMaker' | 'renPy' | 'rpgMaker' | 'creation' | 'unknown';
export type LoaderKind = 'bepInEx' | 'bepInExIl2cpp' | 'melonLoader' | 'lovely' | 'scriptHookV' | 'scriptHookVDotNet' | 'ue4ss';
export type ModMethod = 'bepInEx' | 'bepInExIl2cpp' | 'melon' | 'melonLegacy' | 'lovely' | 'gta' | 'unreal' | 'files' | 'none';
export type MakerKind = 'steamodded' | 'lovely' | 'shvdnScript' | 'ue4ssLua' | 'bepInExPlugin' | 'melonMod' | 'files' | 'datapack' | 'paperPlugin';
export type ModRoot = 'game' | 'data';
export type ModSort = 'downloads' | 'rating' | 'updated' | 'name';

export interface ModNote {
  tone: 'info' | 'warn' | 'bad';
  text: string;
  link: string | null;
}

export interface ModLoaderInfo {
  kind: LoaderKind;
  label: string;
  installed: boolean;
  managed: boolean;
  auto: boolean;
  link: string | null;
}

export interface MakerInfo {
  available: boolean;
  kind: MakerKind | null;
  label: string;
  why: string | null;
}

export interface ModGame {
  key: string;
  name: string;
  source: 'steam' | 'epic' | 'folder';
  appId: number | null;
  dir: string;
  exe: string | null;
  engine: ModEngine;
  engineLabel: string;
  antiCheat: string[];
  onlineOnly: boolean;
  method: ModMethod;
  methodLabel: string;
  loaders: ModLoaderInfo[];
  thunderstore: string | null;
  tsLoader: string | null;
  tsInstalls: boolean;
  dataDir: string | null;
  mods: number;
  enabledMods: number;
  image: string | null;
  notes: ModNote[];
  maker: MakerInfo;
}

export type ModSource = { type: 'thunderstore'; fullName: string; url: string } | { type: 'file'; name: string } | { type: 'claude'; project: string } | { type: 'github'; repo: string };

export interface ModDeployed {
  root: ModRoot;
  path: string;
  size: number;
  replaced: boolean;
}

export interface InstalledMod {
  id: string;
  game: string;
  name: string;
  version: string;
  author: string;
  description: string;
  icon: string;
  source: ModSource;
  enabled: boolean;
  loader: LoaderKind | null;
  dependency: boolean;
  installedAt: number;
  gameDir: string;
  dataDir: string | null;
  files: ModDeployed[];
  created: [ModRoot, string][];
  moved: [ModRoot, string][];
  update: string | null;
}

export interface TsPackage {
  fullName: string;
  name: string;
  owner: string;
  description: string;
  icon: string;
  version: string;
  downloadUrl: string;
  dependencies: string[];
  downloads: number;
  rating: number;
  categories: string[];
  updated: string;
  deprecated: boolean;
  nsfw: boolean;
  pinned: boolean;
  website: string;
  size: number;
  url: string;
}

export interface BrowsePackage extends TsPackage {
  installed: string | null;
  isLoader: boolean;
  managerLink: string;
}

export interface ModBrowse {
  packages: BrowsePackage[];
  total: number;
  loading: boolean;
  progress: [number, number] | null;
  error: string | null;
  categories: string[];
}

export interface WorkshopItem {
  id: string;
  title: string;
  image: string;
  size: number;
  updated: number;
  url: string;
}

export interface ClaudeSettings {
  apiSecret: string;
  model: string;
  effort: 'low' | 'medium' | 'high' | 'xhigh' | 'max';
  web: boolean;
  hasKey: boolean;
}

export interface ClaudeUsage {
  input: number;
  output: number;
  cacheRead: number;
  cacheWrite: number;
  webSearches: number;
}

export type MakerStatus = 'idle' | 'working' | 'waiting' | 'error';

export interface MakerProject {
  id: string;
  game: string;
  gameName: string;
  name: string;
  kind: MakerKind;
  created: number;
  updated: number;
  status: MakerStatus;
  ready: boolean;
  version: number;
  summary: string;
  howToUse: string;
  installed: string | null;
  installedVersion: number;
  usage: ClaudeUsage;
  cost: number;
  model: string;
  web: boolean;
  approval: { id: string; what: string } | null;
  error: string | null;
}

export interface MakerItem {
  kind: 'user' | 'text' | 'note' | 'tool' | 'web' | 'notice';
  text: string;
  detail?: string;
  tone?: 'info' | 'warn' | 'bad' | 'good';
  at: number;
}

export interface MakerView {
  project: MakerProject;
  items: MakerItem[];
  files: { path: string; size: number }[];
}

export interface MakerDelta {
  project: string;
  kind: 'text' | 'note' | 'tool' | 'web' | 'fallback';
  text: string;
}

export type McLoader = 'fabric' | 'quilt' | 'forge' | 'neoforge';

export interface McPackFile {
  path: string;
  sha1: string;
  sha512: string;
  size: number;
  urls: string[];
  client: boolean;
  local: string | null;
}

export interface McSource {
  id: string;
  name: string;
  kind: { type: 'modrinth'; project: string; version: string } | { type: 'file'; path: string } | { type: 'folder'; path: string } | { type: 'launcher'; launcher: string; path: string };
  icon: string | null;
  mc: string;
  loader: string;
  loaderVersion: string;
  files: McPackFile[];
  overrides: number;
}

export type McLauncher = 'prism' | 'curseforge' | 'modrinth' | 'minecraft';

/** A mod in an installed modpack, as its jar describes itself. */
export interface McModInfo {
  file: string;
  id: string;
  name: string;
  version: string;
  /** client: only runs in the game itself; server; or '' (both, or it doesn't say). */
  side: '' | 'client' | 'server';
  disabled: boolean;
  size: number;
}

/** A single-player world in an installed modpack. */
export interface McWorld {
  folder: string;
  name: string;
  version: string;
  lastPlayed: number | null;
  gameMode: string;
  hardcore: boolean;
  difficulty: string;
  cheats: boolean;
  size: number;
  /** It keeps the single player's things, which can become yours on a server. */
  hasPlayer: boolean;
  hasIcon: boolean;
}

export interface McLauncherDetails {
  mods: McModInfo[];
  worlds: McWorld[];
  size: number;
}

export type McPackFormat = 'mrpack' | 'curseforge';

/** A server put in a shared pack's server list. */
export interface McServerEntry {
  name: string;
  address: string;
}

export interface McShared {
  mods: number;
  fromModrinth: number;
  included: number;
  bytes: number;
}

/** A modpack Prism Launcher (or PolyMC/MultiMC), CurseForge or the Modrinth App installed. */
export interface McLauncherPack {
  id: string;
  launcher: McLauncher;
  /** "Prism Launcher", "PolyMC", "MultiMC", "CurseForge", "Modrinth App". */
  launcherName: string;
  name: string;
  path: string;
  /** Where its mods/ and config/ are. */
  gameDir: string;
  mc: string;
  /** Empty for plain Minecraft. */
  loader: string;
  loaderVersion: string;
  mods: number;
  /** Turned off in the launcher. */
  disabled: number;
  /** The modpack it was installed from, with its version. */
  pack: string | null;
  /** Unix seconds. */
  lastPlayed: number | null;
  hasIcon: boolean;
}

export interface McLauncherPacks {
  packs: McLauncherPack[];
  /** Folders the user added to look in. */
  places: string[];
}

export type McEntryStatus = 'kept' | 'updated' | 'dependency' | 'asIs' | 'missing' | 'conflict';

export interface McEntry {
  project: string | null;
  title: string;
  icon: string | null;
  folder: string;
  version: string;
  file: McPackFile | null;
  from: string[];
  status: McEntryStatus;
  note: string | null;
}

export interface McMix {
  id: string;
  name: string;
  mc: string;
  loader: string;
  loaderVersion: string;
  sources: string[];
  entries: McEntry[];
  clashes: string[];
  problems: string[];
  installable: boolean;
  /** The packs' ids, in the same order as `sources`. */
  sourceIds: string[];
  /** How its cover was made: blended here, or painted by Pollinations. */
  /** blend: made here; ai: painted by Pollinations; shared: the picture a friend's pack came with. */
  cover: 'blend' | 'ai' | 'shared' | null;
}

/** The user's own Pollinations account, for painted mix covers. */
export interface PollStatus {
  connected: boolean;
  user: string | null;
  via: 'signIn' | 'key' | null;
  connectedAt: number | null;
  /** A sign-in is waiting to be approved on Pollinations' site. */
  waiting: boolean;
}

export interface PollSignIn {
  userCode: string;
  /** Pollinations' approval page with the code filled in. */
  url: string;
  expiresIn: number;
  interval: number;
}

export interface McPackHit {
  project: string;
  slug: string;
  title: string;
  description: string;
  icon: string | null;
  downloads: number;
  versions: string[];
  loaders: string[];
}

export interface McPackVersion {
  id: string;
  name: string;
  number: string;
  mc: string[];
  loaders: string[];
  date: string;
}

export interface McInstalled {
  name: string;
  profile: string;
  dir: string;
  versionId: string;
  mc: string;
  loader: string;
  mods: number;
  at: number;
}

export interface McStatus {
  found: boolean;
  dir: string | null;
  sources: McSource[];
  installed: McInstalled[];
}

export interface ModProgress {
  game: string;
  text: string;
  done: number;
  total: number;
}

// ---------- Chats ----------

/** OmniHub's own chats, Claude Code's and Codex's conversations on this PC. */
export type ChatSource = 'omni' | 'claudeCode' | 'codex';
export type ChatRole = 'user' | 'assistant' | 'tools' | 'notice';
export type ClaudeCodeMode = 'plan' | 'default' | 'acceptEdits' | 'auto';
export type CodexSandbox = 'read-only' | 'workspace-write';
export type NewChatKind = 'claude' | 'openai' | 'claudeCode' | 'codex';

export interface ChatMessage {
  role: ChatRole;
  text: string;
  /** Unix seconds */
  at: number;
  tools?: string[];
  tone?: 'info' | 'warn' | 'bad';
}

export interface ChatSummary {
  source: ChatSource;
  id: string;
  title: string;
  preview: string;
  updated: number;
  /** The folder a Claude Code or Codex conversation works in. */
  folder: string | null;
  model: string | null;
  /** OmniHub's own chats: claude or openai. */
  provider: 'claude' | 'openai' | null;
  branch: string | null;
  running: boolean;
}

export interface ChatLive {
  prompt: string;
  text: string;
  tools: string[];
  note: string;
  started: number;
}

export interface ChatView {
  summary: ChatSummary;
  messages: ChatMessage[];
  /** Older messages weren't included. */
  earlier: boolean;
  live: ChatLive | null;
  /** Changed in the last two minutes, not through OmniHub: probably open on the PC. */
  busyElsewhere: boolean;
}

export interface ChatDelta {
  source: ChatSource;
  id: string;
  kind: 'text' | 'tool' | 'note' | 'notice' | 'done' | 'error';
  text: string;
}

export interface ChatFolder {
  path: string;
  name: string;
  last: number;
  claudeCode: boolean;
  codex: boolean;
}

export interface ChatTool {
  installed: boolean;
  path: string | null;
  version: string | null;
  hasChats: boolean;
}

export interface ChatTools {
  claudeCode: ChatTool;
  codex: ChatTool;
  claudeKey: boolean;
  openaiKey: boolean;
}

export interface OpenAiSettings {
  model: string;
  effort: 'default' | 'low' | 'medium' | 'high';
  web: boolean;
  hasKey: boolean;
}

export interface UsageTokens {
  input: number;
  output: number;
  cacheRead: number;
  cacheWrite: number;
  /** US dollars (Claude Code: what the same tokens would cost on the API). */
  cost: number;
}

export interface UsageDay extends UsageTokens {
  date: string;
}

export interface UsagePeriod {
  today: UsageTokens;
  week: UsageTokens;
  month: UsageTokens;
  days: UsageDay[];
}

export interface UsageLimit {
  name: string;
  usedPercent: number | null;
  reached: boolean;
  resetsAt: number | null;
}

export interface AgentUsage {
  found: boolean;
  period: UsagePeriod;
  window: { start: number; end: number; tokens: UsageTokens } | null;
  limits: UsageLimit[];
  limitsAt: number | null;
  plan: string | null;
}

export interface UsageReport {
  omni: { claude: UsagePeriod; openai: UsagePeriod; maker: UsageTokens };
  claudeCode: AgentUsage;
  codex: AgentUsage;
}

// ---------- Minecraft servers ----------

export type SrvFlavor = 'vanilla' | 'paper' | 'fabric' | 'quilt' | 'forge' | 'neoForge';
export type SrvState = 'stopped' | 'installing' | 'starting' | 'running' | 'stopping' | 'crashed';
export type SrvKind = 'mod' | 'plugin' | 'datapack';
export type SrvGamemode = 'survival' | 'creative' | 'adventure' | 'spectator';
export type SrvDifficulty = 'peaceful' | 'easy' | 'normal' | 'hard';

export interface SrvServer {
  id: string;
  name: string;
  flavor: SrvFlavor;
  mc: string;
  /** Paper build, or the Fabric, Quilt, Forge or NeoForge version. */
  loader: string;
  dir: string;
  launch: { type: 'jar'; jar: string } | { type: 'args'; args: string } | null;
  javaMajor: number;
  javaPath: string | null;
  flags: string[];
  memoryMb: number;
  autoStart: boolean;
  autoRestart: boolean;
  keepAwake: boolean;
  backupHours: number;
  backupKeep: number;
  lastBackup: number;
  upnp: boolean;
  from: string | null;
  created: number;
  eulaAt: number | null;
  owned: boolean;
}

export interface SrvOnline {
  name: string;
  uuid: string | null;
  since: number;
  pos: [number, number, number] | null;
  dimension: string | null;
}

export interface SrvView extends SrvServer {
  state: SrvState;
  since: number;
  online: SrvOnline[];
  maxPlayers: number;
  port: number;
  motd: string;
  whitelist: boolean;
  onlineMode: boolean;
  problem: string | null;
  work: { text: string; done: number; total: number } | null;
  memoryUsedMb: number;
  cpu: number;
  restartNeeded: boolean;
  /** server-icon.png as a data: address. */
  icon: string | null;
  /** This PC's address on the home network, with the port. */
  lan: string | null;
}

export type SrvFrom = { type: 'modrinth'; project: string; version: string } | { type: 'mix'; id: string } | { type: 'file'; path: string } | { type: 'launcher'; id: string };

export interface SrvNew {
  name: string;
  flavor: SrvFlavor;
  mc: string;
  loader?: string;
  memoryMb?: number;
  port?: number;
  eula: boolean;
  onlineMode: boolean;
  maxPlayers?: number;
  motd?: string;
  whitelist: boolean;
  seed?: string;
  gamemode?: SrvGamemode;
  difficulty?: SrvDifficulty;
  from?: SrvFrom;
  /** A single-player world to start from, in an installed modpack. */
  world?: { pack: string; folder: string; owner?: string };
  start: boolean;
}

export interface SrvPatch {
  name?: string;
  memoryMb?: number;
  /** "" for OmniHub's own Java. */
  javaPath?: string;
  flags?: string[];
  autoStart?: boolean;
  autoRestart?: boolean;
  keepAwake?: boolean;
  backupHours?: number;
  backupKeep?: number;
  upnp?: boolean;
  eula?: boolean;
}

export interface SrvLine {
  n: number;
  at: number;
  text: string;
  level: 'info' | 'warn' | 'error' | 'chat' | 'cmd';
}

export type SrvAction =
  | { type: 'give'; item: string; count: number }
  | { type: 'clear'; item: string | null; count: number | null }
  | { type: 'clearSlot'; slot: number }
  | { type: 'teleport'; x: number; y: number; z: number; dimension: string | null }
  | { type: 'teleportTo'; player: string }
  | { type: 'spawn' }
  | { type: 'gamemode'; mode: SrvGamemode }
  | { type: 'heal' }
  | { type: 'feed' }
  | { type: 'message'; text: string }
  | { type: 'kick'; reason: string | null }
  | { type: 'ban'; reason: string | null }
  | { type: 'pardon' }
  | { type: 'op' }
  | { type: 'deop' }
  | { type: 'whitelist'; on: boolean }
  /** An operator's name may join from this place (servers that don't check accounts). */
  | { type: 'allow'; place: string }
  | { type: 'refuse'; place: string };

export interface SrvKnown {
  name: string;
  uuid: string;
  online: boolean;
  op: boolean;
  whitelisted: boolean;
  banned: boolean;
  banReason: string | null;
}

/** An operator's name that joined from a new place and was kept out. */
export interface SrvWaiting {
  name: string;
  place: string;
  at: number;
}

export interface SrvPlayers {
  online: SrvOnline[];
  known: SrvKnown[];
  whitelistOn: boolean;
  enforce: boolean;
  maxPlayers: number;
  /** false: players without a paid account can join too (operators guarded). */
  onlineMode: boolean;
  waiting: SrvWaiting[];
}

export interface SrvItem {
  /** 0–8 hotbar, 9–35 inventory, 100–103 armour (feet to head), -106 off hand. */
  slot: number;
  id: string;
  label: string;
  count: number;
  custom: string | null;
  enchanted: boolean;
}

export interface SrvPlayer {
  name: string;
  uuid: string | null;
  online: boolean;
  /** Read now from the server, or from the player's saved file. */
  live: boolean;
  pos: [number, number, number] | null;
  dimension: string | null;
  health: number | null;
  food: number | null;
  xpLevel: number | null;
  gamemode: SrvGamemode | null;
  selected: number | null;
  items: SrvItem[];
  ender: SrvItem[];
}

export interface SrvMap {
  regions: [number, number][];
  spawn: [number, number, number] | null;
  players: SrvOnline[];
  supported: boolean;
}

export interface SrvContent {
  file: string;
  kind: SrvKind;
  title: string;
  version: string;
  icon: string | null;
  project: string | null;
  source: 'modrinth' | 'pack' | 'mix' | 'claude' | 'file';
  from: string | null;
  dependency: boolean;
  enabled: boolean;
  size: number;
  sha1: string;
}

export interface SrvHit {
  project: string;
  slug: string;
  title: string;
  description: string;
  author: string;
  icon: string | null;
  downloads: number;
  categories: string[];
  serverSide: string;
  clientSide: string;
  installed: boolean;
}

export interface SrvBackup {
  name: string;
  at: number;
  size: number;
  auto: boolean;
}

export interface SrvNet {
  port: number;
  lan: string[];
  publicIp: string | null;
  router: string | null;
  routerIp: string | null;
  upnp: boolean;
  portOpen: boolean;
  upnpError: string | null;
  /** The internet provider shares one public address between homes. */
  cgnat: boolean;
  java: string | null;
  firewall: FirewallReport | null;
}

/** A monitor whose brightness OmniHub can set (DDC/CI, or Windows for a laptop screen). */
export interface Display {
  id: string;
  name: string;
  primary: boolean;
  builtin: boolean;
  /** 0–100; null when the monitor doesn't answer (DDC/CI off in its menu). */
  brightness: number | null;
  contrast: number | null;
}

export type DisplaySetting = 'brightness' | 'contrast';

/** Capri (github.com/mochiiwuff/capri-desktop), installed and run by OmniHub on request. */
export interface CapriStatus {
  installed: boolean;
  version: string | null;
  running: boolean;
  /** The newest release on her page, when asked for. */
  latest: string | null;
}

/** An installed app winget has a newer version of. */
export interface AppUpdate {
  name: string;
  id: string;
  version: string;
  available: string;
  source: string;
}
