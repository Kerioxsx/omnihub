// In-browser mock of the Rust backend, used when the UI runs outside Tauri
// (`npm run dev` in a normal browser, screenshots, UI development). Every
// command in api.ts is implemented against realistic, seeded in-memory data.
// Like Tauri, failures reject with a plain string.

import type { CaptureKind, DeepPartial, DupeOptions, EntryInput, GeneratorOptions, NoteFilter, NoteInput, PowerAction, Rect, ScanRequest, ScrcpyOptions, SearchQuery, Settings, ShotFilter, SortKey } from '@shared/types';
import { type Args, bool, emit, listen, num, obj, optStr, str, strList } from './mock/bus';
import * as core from './mock/core';
import { volumes } from './mock/drives';
import * as media from './mock/media';
import * as notes from './mock/notes';
import * as remote from './mock/remote';
import { latency } from './mock/rng';
import * as storage from './mock/storage';
import * as vault from './mock/vault';

type Handler = (args: Args) => unknown;

const SORT_KEYS: readonly SortKey[] = ['size', 'alloc', 'name', 'modified', 'files'];
const sortKey = (args: Args): SortKey => {
  const v = args.sort;
  return SORT_KEYS.find((k) => k === v) ?? 'size';
};

const handlers: Record<string, Handler> = {
  // app
  app_info: () => core.appInfo,
  system_stats: () => core.systemStats(),
  settings_get: () => core.settings,
  settings_update: (a) => core.updateSettings(obj<DeepPartial<Settings>>(a, 'patch')),
  open_path: (a) => void str(a, 'path'),
  reveal_path: (a) => void str(a, 'path'),
  open_url: (a) => void str(a, 'url'),
  restart_elevated: () => {
    core.audit('desktop', 'app.elevate', 'Restart as administrator requested');
  },
  audit_list: (a) => core.auditLog.slice(num(a, 'offset', 0), num(a, 'offset', 0) + num(a, 'limit', 100)),
  audit_clear: () => {
    core.auditLog.length = 0;
  },
  pick_folder: (a) => core.pickFolder(optStr(a, 'title')),
  pick_files: () => core.pickFiles(),
  pick_save_file: (a) => `C:\\Users\\Alex\\Documents\\${str(a, 'defaultName')}`,
  set_autostart: (a) => {
    core.updateSettings({ general: { launchAtLogin: bool(a, 'enabled') } });
  },
  hide_window: () => undefined,

  // storage
  storage_volumes: () => volumes(),
  storage_scan: (a) => storage.scanStart(obj<ScanRequest>(a, 'request')),
  storage_progress: (a) => storage.scanProgress(str(a, 'jobId')),
  storage_cancel: (a) => storage.scanCancel(str(a, 'jobId')),
  storage_scans: () => storage.scanList(),
  storage_summary: (a) => storage.scanSummary(str(a, 'scanId')),
  storage_open_cached: (a) => storage.openCached(str(a, 'root')),
  storage_children: (a) => storage.children(str(a, 'scanId'), num(a, 'node'), sortKey(a), bool(a, 'descending', true), num(a, 'offset', 0), num(a, 'limit', 500)),
  storage_treemap: (a) => storage.treemap(str(a, 'scanId'), num(a, 'node'), num(a, 'depth', 3), num(a, 'maxItems', 1500)),
  storage_top_files: (a) => storage.topFiles(str(a, 'scanId'), num(a, 'node'), num(a, 'n', 100)),
  storage_extensions: (a) => storage.extensions(str(a, 'scanId'), num(a, 'node')),
  storage_search: (a) => storage.search(str(a, 'scanId'), obj<SearchQuery>(a, 'query')),
  storage_path: (a) => storage.nodePath(str(a, 'scanId'), num(a, 'node')),
  storage_cleanup: (a) => storage.cleanup(str(a, 'scanId')),
  storage_delete: (a) => storage.deletePaths(strList(a, 'paths'), bool(a, 'permanent')),
  storage_empty_recycle_bin: () => storage.emptyRecycleBin(),
  storage_duplicates_start: (a) => storage.dupesStart(str(a, 'scanId'), obj<DupeOptions>(a, 'options')),
  storage_duplicates_progress: (a) => storage.dupesProgress(str(a, 'jobId')),
  storage_duplicates_result: (a) => storage.dupesResult(str(a, 'jobId')),
  storage_duplicates_cancel: (a) => storage.dupesCancel(str(a, 'jobId')),

  // apps
  apps_list: () => media.appsList(),
  apps_icon: (a) => media.appIconFor(str(a, 'id')),
  apps_launch: (a) => media.appLaunch(str(a, 'id')),
  apps_uninstall: (a) => media.appUninstall(str(a, 'id')),
  apps_screenshots: (a) => media.appScreenshots(str(a, 'id')),

  // screenshots
  shots_list: (a) => media.shotsList(obj<ShotFilter>(a, 'filter')),
  shots_capture: (a) => media.shotCapture(str(a, 'kind') as CaptureKind, num(a, 'delaySeconds', 0)),
  shots_region_begin: () => media.regionBegin(),
  shots_region_pending: () => media.regionPending(),
  shots_region_commit: (a) => media.regionCommit(str(a, 'id'), obj<Rect>(a, 'rect')),
  shots_region_cancel: () => media.regionCancel(),
  shots_thumb: (a) => media.shotThumb(str(a, 'id')),
  shots_image: (a) => media.shotImage(str(a, 'id')),
  shots_update: (a) => media.shotUpdate(str(a, 'id'), { tags: Array.isArray(a.tags) ? strList(a, 'tags') : null, note: optStr(a, 'note'), favorite: typeof a.favorite === 'boolean' ? a.favorite : null }),
  shots_delete: (a) => media.shotDelete(str(a, 'id')),
  shots_copy: (a) => void str(a, 'id'),
  shots_sync: () => media.shotsSync(),

  // notes
  notes_list: (a) => notes.notesList(obj<NoteFilter>(a, 'filter')),
  notes_get: (a) => notes.noteGet(str(a, 'id')),
  notes_save: (a) => notes.noteSave(obj<NoteInput>(a, 'input')),
  notes_delete: (a) => notes.noteDelete(str(a, 'id')),
  notes_tags: () => notes.noteTags(),
  notes_export: (a) => notes.noteExport(str(a, 'id')),
  notes_folder_files: () => notes.folderFiles(),
  notes_folder_read: (a) => notes.folderRead(str(a, 'path')),

  // vault
  vault_status: () => vault.status(),
  vault_create: (a) => vault.create(str(a, 'password')),
  vault_unlock: (a) => vault.unlock(str(a, 'password')),
  vault_lock: () => vault.lock('manual'),
  vault_touch: () => vault.touch(),
  vault_list: () => vault.list(),
  vault_get: (a) => vault.get(str(a, 'id')),
  vault_save: (a) => vault.save(obj<EntryInput>(a, 'input')),
  vault_delete: (a) => vault.remove(str(a, 'id')),
  vault_copy: (a) => vault.copy(str(a, 'id'), str(a, 'field')),
  vault_generate: (a) => vault.generate(obj<GeneratorOptions>(a, 'options')),
  vault_strength: (a) => vault.strength(str(a, 'password')),
  vault_change_password: (a) => vault.changePassword(str(a, 'oldPassword'), str(a, 'newPassword')),
  vault_hello_enable: () => vault.helloEnable(),
  vault_hello_disable: () => vault.helloDisable(),
  vault_hello_unlock: () => vault.helloUnlock(),
  vault_export: (a) => void str(a, 'path'),
  vault_import: (a) => vault.importBackup(str(a, 'password')),

  // phone companion
  remote_status: () => remote.status(),
  remote_start: () => {
    core.updateSettings({ remote: { enabled: true } });
    return remote.status();
  },
  remote_stop: () => {
    core.updateSettings({ remote: { enabled: false } });
  },
  remote_pair_begin: () => remote.pairBegin(),
  remote_pair_cancel: () => remote.pairCancel(),
  remote_devices: () => remote.deviceList(),
  remote_device_rename: (a) => remote.renameDevice(str(a, 'id'), str(a, 'name')),
  remote_device_revoke: (a) => remote.revokeDevice(str(a, 'id')),
  remote_send: (a) => remote.send(strList(a, 'paths'), optStr(a, 'deviceId')),
  remote_inbox: () => remote.inboxList(),
  remote_stop_viewer: (a) => remote.stopViewer(str(a, 'id')),
  remote_stop_all_viewers: () => remote.stopAllViewers(),

  // power
  power_schedule: (a) => remote.powerSchedule(str(a, 'action') as PowerAction, num(a, 'delaySeconds', 10)),
  power_cancel: () => remote.powerCancel(),
  power_pending: () => remote.powerPending(),

  // screen sharing
  screen_monitors: () => remote.monitors,
  screen_presets: () => remote.presets,
  scrcpy_status: () => remote.scrcpyStatus(),
  scrcpy_launch: (a) => remote.scrcpyLaunch(obj<ScrcpyOptions>(a, 'options')),
  scrcpy_stop: () => remote.scrcpyStop(),
  scrcpy_wireless: (a) => remote.scrcpyWireless(str(a, 'serial')),
  scrcpy_connect: (a) => remote.scrcpyConnect(str(a, 'addr')),
  scrcpy_pair: (a) => remote.scrcpyPair(str(a, 'addr'), str(a, 'code')),
  sunshine_status: () => remote.sunshineStatus(),
};

const FAST = new Set(['vault_strength', 'vault_touch', 'vault_status', 'system_stats', 'vault_generate', 'storage_progress', 'storage_duplicates_progress', 'remote_status']);

export async function mockInvoke(cmd: string, args: Record<string, unknown>): Promise<unknown> {
  const handler = handlers[cmd];
  if (!handler) throw `mock: unknown command ${cmd}`;
  await (FAST.has(cmd) ? latency(5, 25) : latency());
  try {
    const result = await handler(args);
    return result === undefined ? null : structuredClone(result);
  } catch (e) {
    // Tauri rejects with the error string.
    throw e instanceof Error ? e.message : String(e);
  }
}

export function mockListen(topic: string, handler: (payload: unknown) => void): () => void {
  return listen(topic, handler);
}

export function mockEmit(topic: string, payload: unknown): void {
  emit(topic, payload);
}
