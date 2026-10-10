// Typed access to the Rust side. Every function maps 1:1 to a Tauri command
// in src-tauri/src/commands.rs (argument names are camelCase here; Tauri
// converts them). Outside Tauri (plain `npm run dev` in a browser) calls go
// to the in-memory mock in ./mock.ts so the UI can be developed and
// screenshotted without Windows.

import type * as T from '@shared/types';

type Args = Record<string, unknown>;

export const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

async function call<R>(cmd: string, args: Args = {}): Promise<R> {
  if (inTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<R>(cmd, args);
  }
  const { mockInvoke } = await import('./mock');
  return mockInvoke(cmd, args) as Promise<R>;
}

/** Subscribe to a backend event (see the topic list in README). Returns an unsubscribe function. */
export async function on<P = unknown>(topic: string, handler: (payload: P) => void): Promise<() => void> {
  if (inTauri) {
    const { listen } = await import('@tauri-apps/api/event');
    return listen<P>(topic, (e) => handler(e.payload));
  }
  const { mockListen } = await import('./mock');
  return mockListen(topic, handler as (p: unknown) => void);
}

export const api = {
  app: {
    info: () => call<T.AppInfoDetails>('app_info'),
    /** What is known about the previous run, when it did not end normally (once). */
    lastCrash: () => call<string | null>('app_last_crash'),
    logError: (message: string) => call<void>('app_log_error', { message }),
    stats: () => call<T.SystemStats>('system_stats'),
    processes: (sort: T.ProcessSort, limit = 8) => call<T.ProcessGroup[]>('system_processes', { sort, limit }),
    endProcess: (name: string) => call<number>('system_end_process', { name }),
    usage: (sort: T.ProcessSort, limit = 200) => call<T.Usage>('system_usage', { sort, limit }),
    setPriority: (name: string, priority: T.Priority) => call<number>('system_set_priority', { name, priority }),
    settings: () => call<T.Settings>('settings_get'),
    updateSettings: (patch: T.DeepPartial<T.Settings>) => call<T.Settings>('settings_update', { patch }),
    exportSettings: (path: string) => call<void>('settings_export', { path }),
    importSettings: (path: string) => call<T.Settings>('settings_import', { path }),
    openPath: (path: string) => call<void>('open_path', { path }),
    revealPath: (path: string) => call<void>('reveal_path', { path }),
    openUrl: (url: string) => call<void>('open_url', { url }),
    clipboardText: () => call<string | null>('clipboard_text'),
    takePendingSend: () => call<string[]>('take_pending_send'),
    restartElevated: () => call<void>('restart_elevated'),
    audit: (limit = 100, offset = 0) => call<T.AuditEntry[]>('audit_list', { limit, offset }),
    clearAudit: () => call<void>('audit_clear'),
    /** Native folder picker; null when cancelled. */
    pickFolder: (title?: string) => call<string | null>('pick_folder', { title: title ?? null }),
    /** Native file picker (multiple); empty when cancelled. */
    pickFiles: (title?: string) => call<string[]>('pick_files', { title: title ?? null }),
    saveFile: (title: string, defaultName: string) => call<string | null>('pick_save_file', { title, defaultName }),
    setAutostart: (enabled: boolean) => call<void>('set_autostart', { enabled }),
    hideToTray: () => call<void>('hide_window'),
  },

  storage: {
    scanTaskStatus: () => call<boolean>('storage_scan_task_status'),
    scanTaskSet: (on: boolean) => call<boolean>('storage_scan_task_set', { on }),
    volumes: () => call<T.VolumeInfo[]>('storage_volumes'),
    scan: (request: T.ScanRequest) => call<string>('storage_scan', { request }),
    progress: (jobId: string) => call<T.JobProgress | null>('storage_progress', { jobId }),
    cancel: (jobId: string) => call<void>('storage_cancel', { jobId }),
    scans: () => call<T.ScanSummary[]>('storage_scans'),
    summary: (scanId: string) => call<T.ScanSummary>('storage_summary', { scanId }),
    openCached: (root: string) => call<T.ScanSummary | null>('storage_open_cached', { root }),
    children: (scanId: string, node: number, sort: T.SortKey = 'size', descending = true, offset = 0, limit = 500) =>
      call<T.ChildrenPage>('storage_children', { scanId, node, sort, descending, offset, limit }),
    treemap: (scanId: string, node: number, depth = 3, maxItems = 1500) => call<T.TreemapItem>('storage_treemap', { scanId, node, depth, maxItems }),
    thumb: (path: string, size: number) => call<string | null>('storage_thumb', { path, size }),
    topFiles: (scanId: string, node: number, n = 100) => call<T.PathedNode[]>('storage_top_files', { scanId, node, n }),
    growth: (scanId: string, limit = 15) => call<T.GrowthReport>('storage_growth', { scanId, limit }),
    /** "children": the folder's contents; "largest": its largest files. Returns the row count. */
    exportCsv: (scanId: string, node: number, kind: 'children' | 'largest', path: string) => call<number>('storage_export_csv', { scanId, node, kind, path }),
    extensions: (scanId: string, node: number) => call<T.ExtensionStat[]>('storage_extensions', { scanId, node }),
    search: (scanId: string, query: T.SearchQuery) => call<T.SearchResult>('storage_search', { scanId, query }),
    path: (scanId: string, node: number) => call<string>('storage_path', { scanId, node }),
    cleanup: (scanId: string) => call<T.Suggestion[]>('storage_cleanup', { scanId }),
    delete: (paths: string[], permanent: boolean) => call<T.DeleteResult[]>('storage_delete', { paths, permanent }),
    emptyRecycleBin: () => call<void>('storage_empty_recycle_bin'),
    duplicatesStart: (scanId: string, options: T.DupeOptions) => call<string>('storage_duplicates_start', { scanId, options }),
    duplicatesProgress: (jobId: string) => call<T.DupeJobProgress | null>('storage_duplicates_progress', { jobId }),
    duplicatesResult: (jobId: string) => call<T.DupeGroup[] | null>('storage_duplicates_result', { jobId }),
    duplicatesCancel: (jobId: string) => call<void>('storage_duplicates_cancel', { jobId }),
  },

  apps: {
    list: (refresh = false) => call<T.AppInfo[]>('apps_list', { refresh }),
    icon: (id: string) => call<string | null>('apps_icon', { id }),
    launch: (id: string) => call<void>('apps_launch', { id }),
    uninstall: (id: string) => call<void>('apps_uninstall', { id }),
    screenshots: (id: string) => call<T.Screenshot[]>('apps_screenshots', { id }),
    startup: () => call<T.StartupItem[]>('startup_list'),
    /** Returns the updated list. */
    setStartup: (id: string, enabled: boolean) => call<T.StartupItem[]>('startup_set', { id, enabled }),
  },

  shots: {
    list: (filter: T.ShotFilter = {}) => call<T.Screenshot[]>('shots_list', { filter }),
    capture: (kind: T.CaptureKind, delaySeconds = 0) => call<T.Screenshot>('shots_capture', { kind, delaySeconds }),
    /** Freezes the screen and opens the region overlay window. */
    regionBegin: () => call<void>('shots_region_begin'),
    regionPending: () => call<T.PendingRegion | null>('shots_region_pending'),
    regionCommit: (id: string, rect: T.Rect) => call<T.Screenshot>('shots_region_commit', { id, rect }),
    regionCancel: () => call<void>('shots_region_cancel'),
    thumb: (id: string) => call<string>('shots_thumb', { id }),
    image: (id: string) => call<string>('shots_image', { id }),
    update: (id: string, patch: { tags?: string[]; note?: string; favorite?: boolean }) =>
      call<T.Screenshot>('shots_update', { id, tags: patch.tags ?? null, note: patch.note ?? null, favorite: patch.favorite ?? null }),
    delete: (id: string, trash = true) => call<void>('shots_delete', { id, trash }),
    copy: (id: string) => call<void>('shots_copy', { id }),
    /** Text in the image (Windows OCR). */
    text: (id: string) => call<string>('shots_text', { id }),
    /** Save an edited version (PNG data URL) as a new screenshot. */
    saveEdit: (id: string, png: string) => call<T.Screenshot>('shots_save_edit', { id, png }),
    sync: () => call<number>('shots_sync'),
  },

  notes: {
    list: (filter: T.NoteFilter = {}) => call<T.Note[]>('notes_list', { filter }),
    get: (id: string) => call<T.Note>('notes_get', { id }),
    save: (input: T.NoteInput) => call<T.Note>('notes_save', { input }),
    delete: (id: string) => call<void>('notes_delete', { id }),
    /** Set (Unix seconds) or clear (null) a reminder. */
    remind: (id: string, at: number | null) => call<T.Note>('notes_remind', { id, at }),
    tags: () => call<[string, number][]>('notes_tags'),
    /** Write to the Claude folder from settings. */
    export: (id: string) => call<T.Note>('notes_export', { id }),
    folderFiles: () => call<T.FolderFile[]>('notes_folder_files'),
    folderRead: (path: string) => call<string>('notes_folder_read', { path }),
  },

  vault: {
    status: () => call<T.VaultStatus>('vault_status'),
    create: (password: string) => call<void>('vault_create', { password }),
    unlock: (password: string) => call<void>('vault_unlock', { password }),
    lock: () => call<void>('vault_lock'),
    touch: () => call<void>('vault_touch'),
    list: () => call<T.EntrySummary[]>('vault_list'),
    get: (id: string) => call<T.Entry>('vault_get', { id }),
    save: (input: T.EntryInput) => call<T.EntrySummary>('vault_save', { input }),
    delete: (id: string) => call<void>('vault_delete', { id }),
    copy: (id: string, field: 'password' | 'username' | 'email' | 'url' | 'notes' | 'totp') => call<void>('vault_copy', { id, field }),
    totp: (id: string) => call<T.TotpCode>('vault_totp', { id }),
    health: () => call<T.HealthReport>('vault_health'),
    /** Have I Been Pwned range check (only hash prefixes leave the PC). */
    breachCheck: () => call<T.HealthItem[]>('vault_breach_check'),
    generate: (options: T.GeneratorOptions) => call<string>('vault_generate', { options }),
    strength: (password: string) => call<T.Strength>('vault_strength', { password }),
    changePassword: (oldPassword: string, newPassword: string) => call<void>('vault_change_password', { oldPassword, newPassword }),
    helloEnable: () => call<void>('vault_hello_enable'),
    helloDisable: () => call<void>('vault_hello_disable'),
    helloUnlock: () => call<void>('vault_hello_unlock'),
    exportBackup: (path: string) => call<void>('vault_export', { path }),
    importBackup: (path: string, password: string) => call<number>('vault_import', { path, password }),
  },

  update: {
    state: () => call<T.UpdateInfo>('update_state'),
    check: () => call<T.UpdateInfo>('update_check'),
    install: () => call<void>('update_install'),
  },

  games: {
    list: () => call<{ profiles: T.GameProfile[]; session: T.GameSession | null }>('games_list'),
    create: (kind: T.GameKind) => call<T.GameProfile>('games_create', { kind }),
    save: (profile: T.GameProfile) => call<{ profile: T.GameProfile; warnings: string[] }>('games_save', { profile }),
    remove: (id: string) => call<void>('games_delete', { id }),
    state: (id: string) => call<T.GameState | null>('games_state', { id }),
    play: (id: string, launch: boolean) => call<T.GameSession>('games_play', { id, launch }),
    stop: () => call<boolean>('games_stop'),
    ping: (id: string | null, host?: string) => call<T.PingResult[]>('games_ping', { id, host: host ?? null }),
    pingTargets: (id: string | null) => call<T.PingTarget[]>('games_ping_targets', { id }),
    robloxStatus: () => call<T.RobloxInstall>('games_roblox_status'),
    robloxWrite: (flags: T.RobloxFlags) => call<string[]>('games_roblox_write', { flags }),
    robloxPreview: (flags: T.RobloxFlags) => call<{ flags: Record<string, unknown>; ignored: string[] }>('games_roblox_preview', { flags }),
    configs: () => call<T.GameConfigs>('games_configs'),
    setConfigOptions: (options: T.ConfigOptions) => call<T.GameConfigs>('games_config_options', { options }),
    applyConfig: (game: T.ConfigGame) => call<T.GameConfigs>('games_config_apply', { game }),
    restoreConfig: (game: T.ConfigGame) => call<T.GameConfigs>('games_config_restore', { game }),
    library: () => call<T.InstalledGame[]>('games_library'),
    addInstalled: (key: string) => call<T.GameProfile>('games_add_installed', { key }),
    loadTest: (id: string | null) => call<T.LoadTest>('games_load_test', { id }),
    fps: () => call<T.FpsOverview>('games_fps'),
    fpsInstall: () => call<T.FpsOverview>('games_fps_install'),
    fpsAllow: () => call<T.FpsOverview>('games_fps_allow'),
  },

  pc: {
    status: () => call<T.PcStatus>('pc_status'),
    set: (id: T.TweakId, on: boolean) => call<T.PcStatus>('pc_set', { id, on }),
  },

  media: {
    state: () => call<{ state: T.MediaState | null; nowMs: number }>('media_state'),
    control: (action: T.MediaAction, positionMs = 0) => call<void>('media_control', { action, positionMs: Math.max(0, Math.round(positionMs)) }),
    lyrics: () => call<{ key: string | null; lyrics: T.LyricsStatus }>('media_lyrics'),
    video: () => call<{ key: string | null; video: T.VideoStatus }>('media_video'),
    art: (id: string) => call<string | null>('media_art', { id }),
    audio: () => call<{ volume: { level: number; muted: boolean } | null; eq: T.EqStatus }>('media_audio'),
    setVolume: (level: number | null, muted: boolean | null = null) => call<{ level: number; muted: boolean }>('media_set_volume', { level, muted }),
  },

  /** Aurora: the PC's sound, analysed for the music visuals. */
  visual: {
    /** Listen while a view shows the visuals (call every few seconds); `audio:frame` events follow. */
    hold: (who: string) => call<T.VisualStatus>('visual_hold', { who }),
    release: (who: string) => call<void>('visual_release', { who }),
    status: () => call<T.VisualStatus>('visual_status'),
    /** Keep the screen on (while the player shows Aurora and music plays). */
    keepAwake: (on: boolean) => call<void>('visual_keep_awake', { on }),
  },

  /** The glow around the screen (click-through windows over every app). */
  ambient: {
    displays: () => call<T.AmbientDisplay[]>('ambient_displays'),
  },

  browser: {
    status: () => call<T.BrowserStatus>('browser_status'),
    repair: () => call<T.BrowserRegistration[]>('browser_repair'),
    respond: (id: string, allow: boolean) => call<boolean>('browser_pair_respond', { id, allow }),
    revoke: (id: string) => call<boolean>('browser_revoke', { id }),
  },

  remote: {
    status: () => call<T.ServerStatus>('remote_status'),
    start: () => call<T.ServerStatus>('remote_start'),
    stop: () => call<void>('remote_stop'),
    restart: async () => {
      await call<void>('remote_stop');
      return call<T.ServerStatus>('remote_start');
    },
    pairBegin: () => call<T.PairingInfo>('remote_pair_begin'),
    pairCancel: () => call<void>('remote_pair_cancel'),
    devices: () => call<T.Device[]>('remote_devices'),
    renameDevice: (id: string, name: string) => call<void>('remote_device_rename', { id, name }),
    revokeDevice: (id: string) => call<void>('remote_device_revoke', { id }),
    send: (paths: string[], deviceId: string | null = null) => call<T.InboxItem[]>('remote_send', { paths, deviceId }),
    sendText: (text: string, deviceId: string | null = null) => call<T.InboxItem>('remote_send_text', { text, deviceId }),
    inbox: () => call<T.InboxItem[]>('remote_inbox'),
    inboxRemove: (id: string) => call<void>('remote_inbox_remove', { id }),
    stopViewer: (id: string) => call<void>('remote_stop_viewer', { id }),
    stopAllViewers: () => call<void>('remote_stop_all_viewers'),
    diagnostics: () => call<T.RemoteDiagnostics>('remote_diagnostics'),
    fixFirewall: (includePublic: boolean) => call<T.FirewallReport>('remote_fix_firewall', { includePublic }),
    makeNetworkPrivate: (id: string) => call<void>('network_make_private', { id }),
  },

  power: {
    schedule: (action: T.PowerAction, delaySeconds: number) => call<T.PendingPower>('power_schedule', { action, delaySeconds }),
    cancel: () => call<boolean>('power_cancel'),
    pending: () => call<T.PendingPower | null>('power_pending'),
  },

  screen: {
    monitors: () => call<T.MonitorInfo[]>('screen_monitors'),
    shareState: () => call<T.ShareState>('screen_share_state'),
    windows: () => call<T.WindowInfo[]>('screen_windows'),
    setPaused: (on: boolean) => call<T.ShareState>('screen_set_paused', { on }),
    /** null = the whole display the viewer picks */
    setWindow: (id: number | null) => call<T.ShareState>('screen_set_window', { id }),
    presets: () => call<T.Preset[]>('screen_presets'),
    scrcpyStatus: () => call<T.ScrcpyStatus>('scrcpy_status'),
    scrcpyLaunch: (options: T.ScrcpyOptions) => call<void>('scrcpy_launch', { options }),
    scrcpyStop: () => call<void>('scrcpy_stop'),
    scrcpyWireless: (serial: string) => call<string>('scrcpy_wireless', { serial }),
    scrcpyConnect: (addr: string) => call<string>('scrcpy_connect', { addr }),
    scrcpyPair: (addr: string, code: string) => call<string>('scrcpy_pair', { addr, code }),
    sunshineStatus: () => call<T.SunshineStatus>('sunshine_status'),
    airplayStatus: () => call<T.AirPlayStatus>('airplay_status'),
    airplayInstall: () => call<void>('airplay_install'),
    airplayUninstall: () => call<void>('airplay_uninstall'),
    airplayStart: () => call<string | null>('airplay_start'),
    airplayStop: () => call<void>('airplay_stop'),
    airplayKeepOnTop: (on: boolean) => call<void>('airplay_keep_on_top', { on }),
    airplayPlace: (how: 'pip' | 'center') => call<void>('airplay_place', { how }),
    airplayFirewall: () => call<T.FirewallReport>('airplay_firewall'),
    airplayFixFirewall: (includePublic: boolean) => call<T.FirewallReport>('airplay_fix_firewall', { includePublic }),
  },
};

export type Api = typeof api;

/** Error text from a rejected command (Tauri rejects with a string). */
export function errorText(e: unknown): string {
  if (typeof e === 'string') return e;
  if (e instanceof Error) return e.message;
  return JSON.stringify(e);
}
