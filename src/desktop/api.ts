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
    stats: () => call<T.SystemStats>('system_stats'),
    settings: () => call<T.Settings>('settings_get'),
    updateSettings: (patch: T.DeepPartial<T.Settings>) => call<T.Settings>('settings_update', { patch }),
    openPath: (path: string) => call<void>('open_path', { path }),
    revealPath: (path: string) => call<void>('reveal_path', { path }),
    openUrl: (url: string) => call<void>('open_url', { url }),
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
    topFiles: (scanId: string, node: number, n = 100) => call<T.PathedNode[]>('storage_top_files', { scanId, node, n }),
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
    sync: () => call<number>('shots_sync'),
  },

  notes: {
    list: (filter: T.NoteFilter = {}) => call<T.Note[]>('notes_list', { filter }),
    get: (id: string) => call<T.Note>('notes_get', { id }),
    save: (input: T.NoteInput) => call<T.Note>('notes_save', { input }),
    delete: (id: string) => call<void>('notes_delete', { id }),
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
    copy: (id: string, field: 'password' | 'username' | 'email' | 'url' | 'notes') => call<void>('vault_copy', { id, field }),
    generate: (options: T.GeneratorOptions) => call<string>('vault_generate', { options }),
    strength: (password: string) => call<T.Strength>('vault_strength', { password }),
    changePassword: (oldPassword: string, newPassword: string) => call<void>('vault_change_password', { oldPassword, newPassword }),
    helloEnable: () => call<void>('vault_hello_enable'),
    helloDisable: () => call<void>('vault_hello_disable'),
    helloUnlock: () => call<void>('vault_hello_unlock'),
    exportBackup: (path: string) => call<void>('vault_export', { path }),
    importBackup: (path: string, password: string) => call<number>('vault_import', { path, password }),
  },

  remote: {
    status: () => call<T.ServerStatus>('remote_status'),
    start: () => call<T.ServerStatus>('remote_start'),
    stop: () => call<void>('remote_stop'),
    pairBegin: () => call<T.PairingInfo>('remote_pair_begin'),
    pairCancel: () => call<void>('remote_pair_cancel'),
    devices: () => call<T.Device[]>('remote_devices'),
    renameDevice: (id: string, name: string) => call<void>('remote_device_rename', { id, name }),
    revokeDevice: (id: string) => call<void>('remote_device_revoke', { id }),
    send: (paths: string[], deviceId: string | null = null) => call<T.InboxItem[]>('remote_send', { paths, deviceId }),
    inbox: () => call<T.InboxItem[]>('remote_inbox'),
    stopViewer: (id: string) => call<void>('remote_stop_viewer', { id }),
    stopAllViewers: () => call<void>('remote_stop_all_viewers'),
  },

  power: {
    schedule: (action: T.PowerAction, delaySeconds: number) => call<T.PendingPower>('power_schedule', { action, delaySeconds }),
    cancel: () => call<boolean>('power_cancel'),
    pending: () => call<T.PendingPower | null>('power_pending'),
  },

  screen: {
    monitors: () => call<T.MonitorInfo[]>('screen_monitors'),
    presets: () => call<T.Preset[]>('screen_presets'),
    scrcpyStatus: () => call<T.ScrcpyStatus>('scrcpy_status'),
    scrcpyLaunch: (options: T.ScrcpyOptions) => call<void>('scrcpy_launch', { options }),
    scrcpyStop: () => call<void>('scrcpy_stop'),
    scrcpyWireless: (serial: string) => call<string>('scrcpy_wireless', { serial }),
    scrcpyConnect: (addr: string) => call<string>('scrcpy_connect', { addr }),
    scrcpyPair: (addr: string, code: string) => call<string>('scrcpy_pair', { addr, code }),
    sunshineStatus: () => call<T.SunshineStatus>('sunshine_status'),
  },
};

export type Api = typeof api;

/** Error text from a rejected command (Tauri rejects with a string). */
export function errorText(e: unknown): string {
  if (typeof e === 'string') return e;
  if (e instanceof Error) return e.message;
  return JSON.stringify(e);
}
