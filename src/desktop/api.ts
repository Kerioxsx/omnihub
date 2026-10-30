// Typed access to the Rust side. Every function maps 1:1 to a Tauri command
// in src-tauri/src/commands.rs (argument names are camelCase here; Tauri
// converts them). Outside Tauri (plain `npm run dev` in a browser) calls go
// to the in-memory mock in ./mock.ts so the UI can be developed and
// screenshotted without Windows.

import type * as T from '@shared/types';
import { demoArt } from './lib/rig/demoArt';

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
    const { getCurrentWebview } = await import('@tauri-apps/api/webview');
    // Bound to this window, so a window resting in the background is skipped
    // (the backend hands it what it missed when it wakes).
    return listen<P>(topic, (e) => handler(e.payload), { target: getCurrentWebview().label });
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
    robloxShaders: () => call<T.ShaderStatus>('games_roblox_shaders'),
    robloxShadersApply: (shaders: T.RobloxShaders) => call<T.ShaderStatus>('games_roblox_shaders_apply', { shaders }),
    robloxFlagList: (refresh = false) => call<T.KnownFlag[]>('games_roblox_flag_list', { refresh }),
    configs: () => call<T.GameConfigs>('games_configs'),
    setConfigOptions: (options: T.ConfigOptions) => call<T.GameConfigs>('games_config_options', { options }),
    applyConfig: (game: T.ConfigGame) => call<T.GameConfigs>('games_config_apply', { game }),
    restoreConfig: (game: T.ConfigGame) => call<T.GameConfigs>('games_config_restore', { game }),
    fortniteCleanCaches: () => call<T.CacheClean>('games_fortnite_clean'),
    library: () => call<T.InstalledGame[]>('games_library'),
    addInstalled: (key: string) => call<T.GameProfile>('games_add_installed', { key }),
    loadTest: (id: string | null) => call<T.LoadTest>('games_load_test', { id }),
    fps: () => call<T.FpsOverview>('games_fps'),
    fpsInstall: () => call<T.FpsOverview>('games_fps_install'),
    fpsAllow: () => call<T.FpsOverview>('games_fps_allow'),
  },
  capri: {
    status: (check = false) => call<T.CapriStatus>('capri_status', { check }),
    set: (on: boolean) => call<T.CapriStatus>('capri_set', { on }),
    update: () => call<T.CapriStatus>('capri_update'),
  },
  displays: {
    list: () => call<T.Display[]>('displays_list'),
    set: (id: string, setting: T.DisplaySetting, value: number) => call<void>('displays_set', { id, setting, value }),
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
    /** The stream of a video YouTube will not embed (downloads yt-dlp and Deno the first time). */
    videoStream: (id: string, maxHeight: number) => call<string>('media_video_stream', { id, maxHeight }),
    /** Mirror the playing app's Canvas or video from its window (asked again every second or two). */
    mirror: () => call<T.MirrorStatus>('media_mirror'),
    /** The mirror's newest picture after `after`: [number, JPEG], or null. */
    mirrorFrame: async (after: number): Promise<{ seq: number; jpeg: Uint8Array } | null> => {
      const buf = await call<ArrayBuffer | number[] | null>('media_mirror_frame', { after });
      if (!buf) return null;
      const bytes = buf instanceof ArrayBuffer ? new Uint8Array(buf) : new Uint8Array(buf);
      if (bytes.length < 5) return null;
      return { seq: new DataView(bytes.buffer, bytes.byteOffset, 4).getUint32(0, true), jpeg: bytes.subarray(4) };
    },
    /** Match the music video to the playing song by ear (asked again while it works). */
    videoSync: (id: string) => call<{ key: string | null; sync: T.VideoSync }>('media_video_sync', { id }),
    streamStatus: () => call<T.StreamToolStatus>('media_stream_status'),
    /** The playing song's animated album cover (looked up once per song). */
    motion: () => call<{ key: string | null; motion: T.MotionStatus }>('media_motion'),
    /** The user's own lyric font and emoji textures. */
    look: () => call<T.AuroraLook>('aurora_look'),
    /** Pick a font file and use it for the lyrics (null when cancelled). */
    chooseFont: () => call<string | null>('aurora_choose_font'),
    /** Pick an emoji pack (a zip of pictures, or an emoji font) and use it (null when cancelled). */
    chooseEmojiPack: () => call<T.EmojiPack | null>('aurora_choose_emoji_pack'),
    emojiFont: () => call<string | null>('aurora_emoji_font'),
    forgetLook: (font: boolean, textures: boolean) => call<void>('aurora_forget_look', { font, textures }),
    font: () => call<string | null>('aurora_font'),
    texture: (name: string) => call<string | null>('aurora_texture', { name }),
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

  /** My PC: lighting (OpenRGB), fans (LibreHardwareMonitor, liquidctl), screens and specs. */
  rig: {
    specs: (refresh = false) => call<T.Specs>('rig_specs', { refresh }),
    settings: () => call<T.RigSettings>('rig_settings'),
    update: (patch: T.DeepPartial<T.RigSettings>) => call<T.RigSettings>('rig_update', { patch }),
    /** The page is open (call every few seconds); `rig:status` events follow. */
    hold: () => call<T.RigStatus>('rig_hold'),
    profiles: () => call<T.LightProfile[]>('rig_profiles'),
    saveProfile: (profile: T.LightProfile) => call<T.LightProfile>('rig_save_profile', { profile }),
    deleteProfile: (id: string) => call<void>('rig_delete_profile', { id }),
    lhmLogin: (user: string, password: string) => call<void>('rig_lhm_login', { user, password }),
    startOpenRgb: () => call<void>('rig_start_openrgb'),
    media: () => call<T.RigMedia[]>('rig_media'),
    /** Pick a picture, GIF or video (null when cancelled). */
    addMedia: () => call<T.RigMedia | null>('rig_media_add'),
    removeMedia: (id: string) => call<void>('rig_media_remove', { id }),
    displays: () => call<T.RigDisplay[]>('rig_displays'),
  },

  /** Govee lights: on the network (LAN Control) and, with a key, Govee's cloud. */
  govee: {
    settings: () => call<T.GoveeSettings>('govee_settings'),
    update: (patch: T.DeepPartial<T.GoveeSettings>) => call<T.GoveeSettings>('govee_update', { patch }),
    updateLight: (id: string, patch: T.DeepPartial<T.GoveeLightSettings>) => call<T.GoveeSettings>('govee_update_light', { id, patch }),
    /** The page is open (call every few seconds); `govee:status` and `govee:frame` events follow. */
    hold: () => call<T.GoveeStatus>('govee_hold'),
    scan: () => call<void>('govee_scan'),
    /** Kept encrypted for this Windows account; empty removes it. */
    setKey: (key: string) => call<void>('govee_set_key', { key }),
    control: (id: string, capType: string, instance: string, value: unknown) => call<void>('govee_control', { id, capType, instance, value }),
    identify: (id: string) => call<void>('govee_identify', { id }),
    presets: () => call<T.GoveePreset[]>('govee_presets'),
    savePreset: (preset: T.GoveePreset) => call<T.GoveePreset>('govee_save_preset', { preset }),
    deletePreset: (id: string) => call<void>('govee_delete_preset', { id }),
    applyPreset: (id: string, lights: string[] = []) => call<T.GoveeSettings>('govee_apply_preset', { id, lights }),
    saveTimer: (timer: T.GoveeTimer) => call<T.GoveeTimer>('govee_save_timer', { timer }),
    deleteTimer: (id: string) => call<void>('govee_delete_timer', { id }),
    sleep: (minutes: number | null) => call<T.GoveeStatus>('govee_sleep', { minutes }),
    monitors: () => call<T.MonitorInfo[]>('govee_monitors'),
    firewall: () => call<T.FirewallReport>('govee_firewall'),
    fixFirewall: () => call<T.FirewallReport>('govee_fix_firewall'),
  },

  /** Mods for the Steam library (and Epic, and added folders): a mod manager, Thunderstore and the Workshop. */
  mods: {
    games: () => call<T.ModGame[]>('mods_games'),
    list: (key: string) => call<T.InstalledMod[]>('mods_list', { key }),
    /** `mods:listing` events follow while a game's Thunderstore list loads, then `mods:changed`. */
    browse: (key: string, query: string, category: string | null, sort: T.ModSort, offset = 0, limit = 30) => call<T.ModBrowse>('mods_browse', { key, query, category, sort, offset, limit }),
    installPackage: (key: string, fullName: string) => call<T.InstalledMod[]>('mods_install_package', { key, fullName }),
    installFile: (key: string, path: string) => call<T.InstalledMod>('mods_install_file', { key, path }),
    installLoader: (key: string, kind: T.LoaderKind) => call<T.InstalledMod>('mods_install_loader', { key, kind }),
    setEnabled: (id: string, on: boolean) => call<void>('mods_set_enabled', { id, on }),
    setAll: (key: string, on: boolean) => call<number>('mods_set_all', { key, on }),
    uninstall: (id: string) => call<void>('mods_uninstall', { id }),
    update: (id: string) => call<T.InstalledMod>('mods_update', { id }),
    workshop: (key: string) => call<T.WorkshopItem[]>('mods_workshop', { key }),
    addFolder: (name: string, dir: string) => call<T.ModGame>('mods_add_folder', { name, dir }),
    removeFolder: (key: string) => call<void>('mods_remove_folder', { key }),
  },

  /** Claude with the user's own Anthropic API key (never shown to the window). */
  claude: {
    settings: () => call<T.ClaudeSettings>('claude_settings'),
    update: (patch: Partial<T.ClaudeSettings>) => call<T.ClaudeSettings>('claude_update', { patch }),
    setKey: (key: string) => call<void>('claude_set_key', { key }),
    test: () => call<void>('claude_test'),
  },

  /** Claude making mods: `maker:delta` streams text, `maker:changed` says when to reload. */
  maker: {
    projects: (game: string | null = null) => call<T.MakerProject[]>('maker_projects', { game }),
    create: (game: string, text: string) => call<T.MakerProject>('maker_create', { game, text }),
    send: (id: string, text: string) => call<void>('maker_send', { id, text }),
    stop: (id: string) => call<void>('maker_stop', { id }),
    approve: (id: string, yes: boolean) => call<void>('maker_approve', { id, yes }),
    view: (id: string) => call<T.MakerView>('maker_view', { id }),
    file: (id: string, path: string) => call<string>('maker_file', { id, path }),
    remove: (id: string) => call<void>('maker_delete', { id }),
    rename: (id: string, name: string) => call<T.MakerProject>('maker_rename', { id, name }),
    install: (id: string) => call<T.MakerProject>('maker_install', { id }),
  },

  /** Chats: `chats:delta` streams answers, `chats:changed` says when to reload. */
  chats: {
    list: () => call<T.ChatSummary[]>('chats_list'),
    view: (source: T.ChatSource, id: string) => call<T.ChatView>('chats_view', { source, id }),
    send: (source: T.ChatSource, id: string, text: string, mode = '') => call<void>('chats_send', { source, id, text, mode }),
    create: (kind: T.NewChatKind, text: string, folder: string | null = null, mode = '') => call<T.ChatSummary>('chats_create', { kind, folder, text, mode }),
    stop: (source: T.ChatSource, id: string) => call<void>('chats_stop', { source, id }),
    rename: (id: string, title: string) => call<void>('chats_rename', { id, title }),
    remove: (id: string) => call<void>('chats_delete', { id }),
    folders: () => call<T.ChatFolder[]>('chats_folders'),
    tools: () => call<T.ChatTools>('chats_tools'),
    usage: () => call<T.UsageReport>('chats_usage'),
  },

  ideas: {
    list: () => call<T.Note[]>('ideas_list'),
    save: (text: string, from: string | null = null) => call<T.Note>('ideas_save', { text, from }),
  },

  openai: {
    settings: () => call<T.OpenAiSettings>('openai_settings'),
    update: (patch: Partial<T.OpenAiSettings>) => call<T.OpenAiSettings>('openai_update', { patch }),
    setKey: (key: string) => call<void>('openai_set_key', { key }),
    models: () => call<string[]>('openai_models'),
  },

  /** Minecraft (Java) modpacks, mixed. */
  minecraft: {
    status: () => call<T.McStatus>('mc_status'),
    search: (query: string, offset = 0) => call<{ hits: T.McPackHit[]; total: number }>('mc_search', { query, offset }),
    versions: (project: string) => call<T.McPackVersion[]>('mc_versions', { project }),
    addModrinth: (project: string, version: string) => call<T.McSource>('mc_add_modrinth', { project, version }),
    addFile: (path: string) => call<T.McSource>('mc_add_file', { path }),
    addFolder: (path: string) => call<T.McSource>('mc_add_folder', { path }),
    /** Modpacks Prism Launcher, CurseForge and the Modrinth App installed (looked for again). */
    launcherPacks: () => call<T.McLauncherPacks>('mc_launcher_packs'),
    /** That modpack's picture as a data: address, if it has one. */
    launcherIcon: (id: string) => call<string | null>('mc_launcher_icon', { id }),
    addLauncherPack: (id: string) => call<T.McSource>('mc_add_launcher_pack', { id }),
    /** Also look in this folder (a launcher's folder, its instances folder or one instance). */
    addLauncherPlace: (path: string) => call<T.McLauncherPacks>('mc_add_launcher_place', { path }),
    removeLauncherPlace: (path: string) => call<T.McLauncherPacks>('mc_remove_launcher_place', { path }),
    removeSource: (id: string) => call<void>('mc_remove_source', { id }),
    plan: (name: string, ids: string[], mc: string, loader: T.McLoader) => call<T.McMix>('mc_plan', { name, ids, mc, loader }),
    /** Save a mix as a .mrpack (its cover inside), optionally with your server in its list. */
    exportPack: (id: string, path: string, server?: T.McServerEntry | null) => call<void>('mc_export', { id, path, server: server ?? null }),
    /** A pack a friend shared, made into a mix ready to install. */
    importShared: (path: string) => call<T.McMix>('mc_import_shared', { path }),
    launcherDetails: (id: string) => call<T.McLauncherDetails>('mc_launcher_details', { id }),
    launcherWorldIcon: (id: string, folder: string) => call<string | null>('mc_launcher_world_icon', { id, folder }),
    /** Mods with a newer version on Modrinth: file name → version. */
    launcherUpdates: (id: string) => call<Record<string, string>>('mc_launcher_updates', { id }),
    shareLauncherPack: (id: string, path: string, format: T.McPackFormat, server?: T.McServerEntry | null) => call<T.McShared>('mc_share_launcher_pack', { id, path, format, server: server ?? null }),
    install: (id: string) => call<T.McInstalled>('mc_install', { id }),
    uninstall: (profile: string) => call<void>('mc_uninstall', { profile }),
    /** Mixes worked out so far, newest first. */
    mixes: () => call<T.McMix[]>('mc_mixes'),
    /** The mix's cover as a data: address (blended from its packs the first time). */
    cover: (id: string) => call<string>('mc_cover', { id }),
    remakeCover: (id: string) => call<string>('mc_remake_cover', { id }),
    aiCover: (id: string) => call<string>('mc_ai_cover', { id }),
    saveCover: (id: string, path: string) => call<void>('mc_save_cover', { id, path }),
  },

  /** Minecraft servers hosted on this PC. */
  servers: {
    list: () => call<T.SrvView[]>('srv_list'),
    get: (id: string) => call<T.SrvView>('srv_get', { id }),
    versions: (flavor: T.SrvFlavor) => call<string[]>('srv_versions', { flavor }),
    create: (server: T.SrvNew) => call<T.SrvView>('srv_create', { server }),
    adopt: (path: string) => call<T.SrvView>('srv_adopt', { path }),
    update: (id: string, patch: T.SrvPatch) => call<T.SrvView>('srv_update', { id, patch }),
    remove: (id: string, files: boolean) => call<void>('srv_delete', { id, files }),
    start: (id: string) => call<T.SrvView>('srv_start', { id }),
    stop: (id: string) => call<T.SrvView>('srv_stop', { id }),
    restart: (id: string) => call<T.SrvView>('srv_restart', { id }),
    kill: (id: string) => call<void>('srv_kill', { id }),
    console: (id: string, after: number | null = null) => call<T.SrvLine[]>('srv_console', { id, after }),
    command: (id: string, text: string) => call<void>('srv_command', { id, text }),
    properties: (id: string) => call<[string, string][]>('srv_properties', { id }),
    setProperties: (id: string, changes: [string, string][]) => call<T.SrvView>('srv_set_properties', { id, changes }),
    setIcon: (id: string, path: string) => call<T.SrvView>('srv_set_icon', { id, path }),
    players: (id: string) => call<T.SrvPlayers>('srv_players', { id }),
    player: (id: string, name: string) => call<T.SrvPlayer>('srv_player', { id, name }),
    act: (id: string, name: string, action: T.SrvAction) => call<string>('srv_act', { id, name, action }),
    whitelist: (id: string, on: boolean) => call<T.SrvView>('srv_whitelist', { id, on }),
    /** Players' positions; asking keeps them updating for a while. */
    watch: (id: string) => call<T.SrvOnline[]>('srv_watch', { id }),
    map: (id: string, dim: string) => call<T.SrvMap>('srv_map', { id, dim }),
    /** One 512×512-block region as a data: address. */
    mapTile: (id: string, dim: string, x: number, z: number) => call<string>('srv_map_tile', { id, dim, x, z }),
    content: (id: string) => call<T.SrvContent[]>('srv_content', { id }),
    identify: (id: string) => call<T.SrvContent[]>('srv_identify', { id }),
    search: (id: string, kind: T.SrvKind, query: string, offset = 0) => call<{ hits: T.SrvHit[]; total: number }>('srv_search', { id, kind, query, offset }),
    install: (id: string, kind: T.SrvKind, project: string) => call<T.SrvContent[]>('srv_install', { id, kind, project }),
    addFile: (id: string, path: string) => call<T.SrvContent>('srv_add_file', { id, path }),
    setContent: (id: string, file: string, on: boolean) => call<void>('srv_set_content', { id, file, on }),
    removeContent: (id: string, file: string) => call<void>('srv_remove_content', { id, file }),
    backups: (id: string) => call<T.SrvBackup[]>('srv_backups', { id }),
    backup: (id: string) => call<T.SrvBackup>('srv_backup', { id }),
    restore: (id: string, name: string) => call<string>('srv_restore', { id, name }),
    removeBackup: (id: string, name: string) => call<void>('srv_delete_backup', { id, name }),
    network: (id: string) => call<T.SrvNet>('srv_network', { id }),
    setUpnp: (id: string, on: boolean) => call<T.SrvNet>('srv_set_upnp', { id, on }),
    fixFirewall: (id: string) => call<T.SrvNet>('srv_fix_firewall', { id }),
  },

  /** The user's own Pollinations account, for painted mix covers. */
  pollinations: {
    status: () => call<T.PollStatus>('pollinations_status'),
    signIn: () => call<T.PollSignIn>('pollinations_sign_in'),
    check: () => call<T.PollStatus>('pollinations_check'),
    setKey: (key: string) => call<T.PollStatus>('pollinations_set_key', { key }),
    forget: () => call<T.PollStatus>('pollinations_forget'),
  },
};

export type Api = typeof api;

/** Where the window loads a library picture or video from. */
export function rigMediaUrl(id: string): string {
  if (!inTauri) return demoArt(id);
  return `http://rigmedia.localhost/${encodeURIComponent(id)}`;
}

/** Error text from a rejected command (Tauri rejects with a string). */
export function errorText(e: unknown): string {
  if (typeof e === 'string') return e;
  if (e instanceof Error) return e.message;
  return JSON.stringify(e);
}
