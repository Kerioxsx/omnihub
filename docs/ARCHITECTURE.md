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
│    system    elevation · power · clipboard · DPAPI · shell · processes · GPU counters       │ │
│    media     media sessions · lyrics (LRCLIB) · volume · Equalizer APO                      │ │
│    games     profiles · boost/restore journal · Roblox flags · ping                         │ │
│    browser   native-messaging host · pipe to the app · site matching · pairing              │ │
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

## Music, tasks and games

`media/` polls Windows' media sessions (GSMTC) every 250 ms for the track,
cover and position, and sends `media:state` with the PC's clock so the phone
can line up time-synced lyrics (fetched from LRCLIB, cached per song) to
within a frame. `system/procs.rs` groups processes by name; GPU figures come
from the "GPU Engine" and "GPU Process Memory" performance counters (the
busiest engine per program, like Task Manager) via `system/gpu.rs`.

### Aurora

The music visuals are split so nothing time-critical waits on the network
or the UI:

- **Capture** (`media/visual/source.rs`): WASAPI loopback of the default
  output, mixed to mono; a synthesised beat stands in with the pretend
  player. Device changes are noticed every 2 s and the stream reopened.
- **Analysis** (`media/visual/analyzer.rs`, `fft.rs`): 2048-point Hann
  windows, hop of one display frame; RMS, bass (25–160 Hz), mids, highs and
  16 log bands, each normalised by an adaptive peak follower and smoothed
  with separate attack and release; beats from bass-weighted spectral flux
  above a running mean + k·σ, with hysteresis and a 250 ms refractory
  period; tempo from the median gap. Silence switches to a cheap path.
- **Hub** (`media/visual/mod.rs`): one thread at 60 Hz, started by a lease
  and stopped when the last lease expires; emits `audio:frame` while there
  is sound (a heartbeat, then nothing, when quiet) and `audio:status`.
  Track changes (`media:state`) reset the analysis.
- **Lyrics service** (`media/lyrics.rs`, `media/mod.rs`): the user's `.lrc`
  folder first (matched on tags or "Artist - Title" names, length within
  8 s, lines that fit the track), then the cache, then LRCLIB. Lyrics are
  stored per track key, so a new track never gets the last one's lines.
- **Covers** (`media/artwork.rs`): the player's thumbnail is replaced by
  the original-size copy from the iTunes catalogue (asked for at
  10000×10000, which returns the master, usually 3000×3000; at most 16 MB)
  when the song matches (title, artist, length) and the picture looks like
  the thumbnail (16×16 comparison); cached in `data/covers`. The art id
  changes, so views refetch.
- **Music videos** (`media/video.rs`): asked for by the player
  (`media_video`), looked up once per song on YouTube's results page
  (`ytInitialData` parsed, like a browser would get it) and kept only when
  the channel is the artist's own (official artist badge, VEVO, or named
  after the artist), the title names the song, and the upload is not a
  lyric, audio, live, cover, remix, sped-up or vertical version. Music
  videos rank before visualizers (the animated cover); each is marked
  *synced* when its length is within a few seconds of the song's. Answers
  are cached in `data/videos` and announced with `media:video`.
- **Word timing** (`lib/aurora/timing.ts`): for lyrics that only time
  lines, words are spread over each line by syllables (opt-out); real word
  timestamps always win.
- **Frontend** (`src/desktop/lib/aurora`, `components/aurora`): one
  animation loop per view drives two WebGL canvases — the scene (the cover
  art with glitch slices, channel split, a fisheye lens, beat pulses and
  spray-paint edges, or palette fields) and the edge light (a rounded-rect
  distance field with edge, inner highlight, glow and bloom layers, which
  skips pixels away from the edge). Audio frames land in a shared object
  read per frame, so sound never re-renders React. Palettes are extracted
  from the cover with k-means in OKLab and eased between songs in OKLab.
  The lyrics overlay follows the player's clock (`useNowPlaying`), steps
  word by word only with word timestamps, and renders only when the line or
  word changes.
- **Cinema backdrop** (`components/aurora/AuroraBackdrop.tsx`,
  `lib/aurora/cinema.ts`, `youtube.ts`, `coverLook.ts`): behind the lyrics,
  in order, the music video, the visualizer, the full-size cover, or the
  light show above. The video plays in the youtube-nocookie embed, muted,
  steered with the IFrame API's postMessage protocol; a sync loop compares
  the player's reported time with the song's (plus a per-video offset the
  user can nudge, kept in local storage), seeks beyond 1 s of drift and
  sets the rate to 0.94/1.06 between 0.12 s and 1 s. The quality cap is the
  iframe's device-pixel size. When YouTube refuses a video (embedding off,
  removed) the next candidate is tried, then the cover. The cover is drawn
  full size with a slow drift, cropped around its detail (`coverLook`:
  per-row detail and brightness from a 48×48 copy); the same study picks
  the calmest band for the lyrics and how bright it is there, which sets
  the scrim, the frosted patch and the text's shadow. Lyric ink is a light
  tone of the most colourful palette colour (OKLab), its shadow a deep one.
  Corner light is four radial gradients in the palette's colours, scaled by
  the bass.
- **Emojis** (`lib/aurora/emoji.ts`): a word list (with simple lemmas and
  phrases) maps lyric words to emojis; a planner picks at most one per line
  and spaces them out (*Now and then*: at least one line without between
  them, two after a common word like "love"; *More often*: half that),
  never the same twice running. The pictures are
  Fluent Emoji 3D WebPs in `public/emoji/3d`, rebuilt by
  `scripts/emoji-3d.py`.
- **Screen glow** (`src-tauri/src/ambient.rs`, `pages/overlay/AmbientOverlay.tsx`):
  a transparent, click-through, always-on-top window per chosen monitor,
  placed in physical pixels on the monitor or its work area; a watcher
  follows settings, monitors, full-screen apps (`SHQueryUserNotificationState`)
  and game boosts once a second.

`games/` keeps profiles in `games.json`. Play runs a session on its own
thread: each step (close apps, power plan, notifications, per-game registry
settings, admin-only settings in one helper call, Wi-Fi low-latency handle,
Roblox flags, launch) is recorded as it happens and in a journal on disk;
the thread then follows the game's process and restores everything when it
exits or on Stop. Boost modes (Competitive, Quality, Custom) decide which
switches are on; Quality never touches the game's own settings. While the
game runs, `games/fps.rs` runs PresentMon on its process and sends
`games:fps` once a second (to the phone too); the session's summary is kept
per profile. The ping helper uses `IcmpSendEcho` (no admin) and TCP connect
timing for `host:port` targets; lag under load pings while saturating the
line with downloads, then uploads.

## Desktop shell

`src-tauri` registers about 160 commands (`commands.rs`) that call into `AppCore`,
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
