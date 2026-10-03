# Architecture

```
┌──────────────────────────────── OmniHub.exe (standard user) ─────────────────────────────────┐
│  WebView2: React desktop UI (src/desktop)                                                    │
│        │ invoke / events                                                                     │
│  Tauri shell (src-tauri): commands, tray, global hotkeys, region overlay, notifications      │
│        │                                                                                     │
│  omnihub-core (crates/omnihub-core) ── AppCore ─────────────────────────────────────────────┐ │
│    storage   engine · MFT reader · USN · snapshot cache · walker · tree · cleanup · dupes   │ │
│    vault     Argon2id/AES-GCM · DPAPI · Windows Hello · auto-lock · clipboard               │ │
│    notes     SQLite · Markdown export · Claude folder watcher                               │ │
│    apps      registry · Get-StartApps · Appx · shell icons                                  │ │
│    capture   screenshots library · DXGI/GDI capture · JPEG streamer · input · bridges       │ │
│    system    elevation · power · clipboard · DPAPI · shell                                  │ │
│    remote    axum HTTPS/WS server for the phone ── serves dist-mobile (embedded)            │ │
│    settings · db (SQLite) · audit · events (broadcast bus)                                  │ │
│  └──────────────────────────────────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
        │ UAC (only for fast scans)                         │ LAN, HTTPS, paired-device token
┌───────▼──────────────────────────┐              ┌─────────▼────────────────────────────┐
│ OmniHub.exe --omnihub-helper     │              │ Phone browser: src/mobile (PWA)      │
│ scan-volume C <cache dir>        │              │ files · uploads · power · screen ·   │
│ reads \\.\C:, writes snapshot     │              │ notes · apps · vault                 │
└──────────────────────────────────┘              └──────────────────────────────────────┘
```

## Storage engine

`storage/ntfs/format.rs` decodes the boot sector, FILE records (with update
sequence fixups), attributes and data runs. `mft.rs` opens a volume (or an
image file — the same code is tested against real NTFS images), resolves the
`$MFT` run list (including the continuation in extension records when the
MFT is very fragmented), and streams it in 8 MiB chunks: one thread reads,
a rayon pool parses the previous chunk. Each record becomes a *slot*: parent
record, best name (Win32 over POSIX over DOS), logical size of the unnamed
data stream, allocated size of every stream and index (compressed/sparse
streams count their real allocation, so WOF-compressed system files and
sparse files are right), modification time and attributes. Extension records
are folded into their base record; hard links count once.

The slots are the **snapshot** (`snapshot.rs`): saved compressed (LZ4) in the
cache folder with the volume serial and the USN journal position. The next
scan reads only the change journal from that position, re-reads just the
changed records and saves again (`volume_scan.rs`). If the journal was reset
or more than a fifth of the MFT changed, it does a full read instead.
Because the scanner reads the disk rather than the file system cache, a
record changed a moment ago may not be written back yet; the saved journal
position therefore always points before the changes of the last 15 seconds,
so they are read again on the next refresh (re-reading is idempotent).

The snapshot (or a folder walk) is turned into a **scan tree** (`tree.rs`):
nodes in depth-first pre-order with children sorted by size, so a folder's
subtree is a contiguous slice — subtree totals, search under a folder, top
files and extension statistics are linear scans over a slice (parallel with
rayon), and listing children needs no extra index. 40 bytes per node plus
the names.

**Elevation.** Opening `\\.\C:` needs administrator rights. The UI process
never runs elevated by default; the engine starts the same executable with
`--omnihub-helper scan-volume C <dir>` through `ShellExecuteEx("runas")`.
The helper validates its arguments (one drive letter; the output folder must
be `…\OmniHub\cache\scans` and contain no junctions), writes progress to a
small JSON file the app polls, honours a cancel marker, writes the snapshot
and exits. Declining UAC falls back to the folder walk.

## Phone server

`remote/` is an axum app on its own tokio runtime. Layers, outside in:
peer filter (private addresses only) and Host check → security headers →
public routes (`/api/info`, `/api/pair`, ticketed downloads and sockets,
the web app) → bearer-token routes. WebSockets and download links use
short-lived tickets so tokens never appear in URLs. The phone app is
embedded into the binary at build time (`rust-embed`, from `dist-mobile/`).

Uploads (`transfer.rs`) are written chunk by chunk at the confirmed offset
into a `.part` file next to a JSON state file, each chunk checked with
CRC-32; the phone can always ask for the confirmed offset and resume, also
after the PC app restarted. The finished file is hashed with SHA-256 and
moved into place with a non-clobbering name.

Screen sharing (`capture/stream.rs`) runs capture + encode on a dedicated
thread per viewer and hands complete frame messages to the socket task over
a channel of two. The viewer acknowledges each frame after drawing it; at
most two frames are in flight, so latency stays bounded on slow links (old
frames are skipped, not queued). Quality and size adapt to the measured
round trip.

## Desktop shell

`src-tauri` registers about 100 commands (`commands.rs`) that call into `AppCore`,
forwards every core event to the windows (and shows a notification for a
few when the window is hidden), owns the tray menu and the global hotkeys,
and manages the region-capture overlay window. Closing the window hides it
to the tray by default so the phone server keeps running; autostart passes
`--minimized`.

## Frontend

`src/shared/types.ts` mirrors the Rust types, `src/desktop/api.ts` maps every
command, `src/mobile/client.ts` speaks the phone API. Outside Tauri the
desktop UI runs against an in-memory mock (`src/desktop/mock`), which is how
it is developed and screenshotted on any OS.
