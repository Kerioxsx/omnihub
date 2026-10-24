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

`media/` polls Windows' media sessions (GSMTC) every 250 ms while music plays (750 ms otherwise, and quickly again for 3 s after a control) for the track,
cover and position, and sends `media:state` with the PC's clock so the phone
can line up time-synced lyrics (fetched from LRCLIB, cached per song) to
within a frame. `system/procs.rs` groups processes by name; GPU figures come
from the "GPU Engine" and "GPU Process Memory" performance counters (the
busiest engine per program, like Task Manager) via `system/gpu.rs`.

### My PC (`rig/`)

`RigHub` owns `data/rig.json` and one worker thread that runs only while it
is needed (lighting on, a fan profile other than Auto, or the page/case
screen holding a lease) and otherwise exits. `specs.rs` parses SMBIOS and
matches present USB ids; `openrgb.rs` is an OpenRGB SDK client (protocol 3:
controller list, `SETCUSTOMMODE`, `UPDATELEDS`); `lighting.rs` holds the
effects, which `src/desktop/lib/rig/effects.ts` repeats for the 3D preview
(same values); `lhm.rs` reads LibreHardwareMonitor's `data.json` and sets
controls; `fans.rs` holds the curves, floors and smoothing; `liquidctl.rs`
runs liquidctl; `screens.rs` keeps the media library. The worker sends
frames at 30 fps for moving effects (once a second otherwise), reads the
sensors every 2 s, and pushes `rig:status` only while a view holds it. The
shell adds the `casescreen` window (on the chosen monitor) and the
`rigmedia://` protocol. The 3D view (`lib/rig/scene3d.ts`, three.js, loaded
with the page) builds the PC from the specs and the user's description,
draws every LED of a part in one instanced mesh, and pauses when hidden.

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
  10000×10000, which returns the master, usually 3000×3000) or Deezer
  (1900×1900). Candidates from both are ranked by title, artist, length
  and version words (a remix is not the original; "Dame Amor 2" is another
  song), and the first whose picture looks like the thumbnail (16×16
  comparison) is kept; cached in `data/covers` (key prefix `v3`). The art
  id changes, so views refetch.
- **Music videos** (`media/video.rs`): asked for by the player
  (`media_video`), looked up once per song on YouTube's results page
  (`ytInitialData` parsed, like a browser would get it) and kept only when
  the channel is the artist's own (official artist badge, VEVO, or named
  after the artist), the title names the song, and the upload is not a
  lyric, audio, live, cover, remix, sped-up or vertical version. Music
  videos rank before visualizers, the most viewed first; each is marked
  *synced* when its length is within a few seconds of the song's. Answers
  are cached in `data/videos` and announced with `media:video`.
- **Video streams** (`media/stream.rs`, `media_video_stream`): for videos
  YouTube will not embed, yt-dlp (`-g`, best video-only stream up to the
  quality cap, AV1 avoided) with Deno as its JavaScript runtime; both
  downloaded on first use and checked against their releases' SHA-256
  lists; the address is cached for three hours.
- **Animated covers** (`media/motion.rs`, `media_motion`): the web player's
  key from music.apple.com's script (an `AMPWebPlay` JWT, kept until it
  expires, renewed on 401), a catalogue search matched like covers, then
  the album's `editorialVideo.motionDetailSquare` HLS playlist; cached in
  `data/motion` and announced with `media:motion`.
- **Lyrics sources** (`media/lyrics.rs`): LRCLIB (exact, then search), then
  NetEase Cloud Music (search, matched by title, artist, length and
  version; its credit lines dropped). Misses are cached for three days.
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
  word changes. Words leaving the stack fade where they stood, each on its
  own timer and at most three at once (motion's AnimatePresence removes
  leaving children only when all have finished, which never happens while
  words keep coming).
- **Cinema backdrop** (`components/aurora/AuroraBackdrop.tsx`,
  `lib/aurora/cinema.ts`, `youtube.ts`, `motion.ts`, `coverLook.ts`):
  behind the lyrics, in order, the music video, the album's animated cover
  (hls.js, over the still one; it rests while a video shows), the
  visualizer (only without an animated cover), the full-size cover, or the
  light show above (which then draws the animated cover's frames as its
  texture). The cover shows whole (a square as big as the screen's shorter
  side, its edges feathered into a blurred, enlarged copy) or fills the
  screen. The video plays in the youtube-nocookie embed inside OmniHub's
  relay page (`site/player.html` on GitHub Pages: YouTube answers error 150
  to local origins), muted, steered with the IFrame API's postMessage
  protocol, or, when YouTube still refuses (error, or never starts), as its
  own stream in a `<video>` driven through the same interface
  (`DirectLink`); a sync loop compares
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
  and spaces them out (*Some*: any line with a picturable word, a common
  one like "love" or "amor" every other line; *Lots*: every line that has
  one), never the same twice running, and never two lines in a row
  without one (a line with nothing to picture gets a sparkle, a note…).
  English and Spanish (accents dropped, plurals and the commonest verb
  forms). The pictures are Fluent Emoji: plain one-colour SVGs in
  `public/emoji/plain`, drawn as a mask over the lyric ink (the default),
  or 3D WebPs in `public/emoji/3d`; both rebuilt by `scripts/emoji-3d.py`.
- **Video matching** (`media/align.rs`): onset envelopes (log-band spectral
  flux, 50/s) of the video's soundtrack (yt-dlp + symphonia) and of the
  live loopback (tapped from `VisualHub`, mapped to song time through
  playback checkpoints) are cross-correlated (normalised, ±1 s exclusion for
  the runner-up); a clear fit twice in a row is the offset, cached per video
  and song length and announced with `media:videoSync`. The user's own
  nudge wins over it.
- **App mirror** (`media/mirror.rs`): the playing app's window (Spotify,
  Apple Music) is captured at ~12 fps; a 160-px grey copy tracks per-pixel
  change; the largest dense moving block (not text) is the picture, settled
  over a few detections, cropped and JPEG-encoded; the window fetches it
  with `media_mirror_frame` (binary). A picture that stops moving while the
  song plays means the app stopped drawing ("hidden"). Its video beats
  YouTube's; its Canvas beats Apple's cover.
- **Your font & emoji pack** (`media/look.rs`, `lib/aurora/textures.ts`): the
  user's font and emoji pack (a zip of pictures, or an emoji font) are copied
  into `data/aurora-look` and read back as data URLs (fonts as `@font-face`).
  Pictures are kept under a normalised name (lower case, words joined by
  `_`, non-ASCII as code points: `🔥.png` and `1F525.png` are both `1f525`).
  With *Your pack* as the emoji look, each emoji is drawn as the pack's
  picture for the word ("apple" → `apple.png`), else for its code points
  (`1f525`, `emoji_u1f525`), else for one of its names
  (`lib/aurora/emojiNames.ts`, generated by `scripts/emoji-names.py`: CLDR
  names and GitHub/Discord short codes; packs' usual prefixes and suffixes
  like `_3d` are ignored), else for a Minecraft item (❤️ →
  `heart_pottery_sherd`); an emoji font draws them all; else the plain emoji.
  Pixel art (blocks of a 16/24/32-pixel icon) is drawn without smoothing.
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

## Mods

`mods/` is the mod manager. `detect.rs` reads a game folder's names (never
its contents) for its engine, mod loaders and anti-cheat. `thunderstore.rs`
reads Thunderstore's ecosystem schema (games by Steam app id, their loaders
and loader packages) and a community's package listing (gzip chunks, each
reduced to the latest version as it arrives), searches it and resolves
dependencies. `install.rs` unpacks (zip, 7z, folders), plans where each file
goes for the loader (`Layout`: BepInEx's r2modman rules, MelonLoader,
Lovely, GTA, Unreal, a loader's own package…), deploys with backups and
rollback, and switches mods off and on. `workshop.rs` lists Steam Workshop
items. `ModHub` ties these to the games from `games::library`, keeps the
installed mods in `data/mods/state.json` and emits `mods:changed`,
`mods:progress` and `mods:listing`.

`mods/maker.rs` is the mod maker: a project per mod (a conversation kept
append-only, exactly as the API returned it, and a folder of files),
an agent loop over the Messages API (`claude/api.rs`: raw HTTPS, server-sent
events reassembled into content blocks, retries, refusals, fallbacks) with
client tools confined to the game (read-only) and the project folder
(writable), plus web search and fetch. `claude/mod.rs` keeps the key (DPAPI),
model and effort, and prices usage. Builds of C# projects go through a
checked `dotnet build`, after the user approves. Installing a project hands
its folder (or its built assembly) to `ModHub::install_made`.

`mods/minecraft.rs` mixes Minecraft modpacks with Modrinth's API: packs come
from Modrinth, .mrpack files or instance folders; mods are matched by SHA-1,
moved to the chosen version with `version_files/update`, dependencies added,
incompatibilities resolved by pack order, overrides merged; the result is
written as a .mrpack or installed as a Minecraft Launcher profile (Fabric or
Quilt version JSON from their meta services).

`mods/launchers.rs` finds the modpacks other launchers installed: Prism
Launcher, PolyMC and MultiMC (`instance.cfg` as a Qt settings file, the
loader and version from `mmc-pack.json`'s components, the icon by `iconKey`
in the icons folder its settings name), CurseForge (`minecraftinstance.json`:
`baseModLoader`'s name, then its type) and the Modrinth App (`app.db`, read
only: the `instances` table and the content set each uses since mid-2026, the
`profiles` table before that, `profile.json` files before 2024; its folder
moves with `settings.custom_dir`). Each gets a stable id (a hash of its game
folder). `Mixer::add_launcher_pack` adds one as a pack with the launcher's
version and loader; `FromPack::Launcher` makes a server from one (its jars
read locally, `config`, `defaultconfigs`, `kubejs` and `global_packs` along).

0.13: `launchers::worlds`/`world` read `level.dat` (`Data`: name, version,
game type, difficulty, hardcore, commands, whether a `Player` is kept);
`launchers::mods_in` reads each jar's `fabric.mod.json`, `quilt.mod.json` or
`(neoforge.)mods.toml` (version from the manifest for `${file.jarVersion}`);
the Minecraft Launcher's `.minecraft` is an instance too (version from the
profile used last, `version_parts`). `Mixer::share_launcher_pack` and
`export(…, server)` write the friends' file (`servers_dat` makes a one-server
list with `nbt::write`); `import_shared` adds a friend's .mrpack and plans it
as a mix with its `icon.png` as cover (`cover: "shared"`).
`NewServer.world` (`WorldFrom`) makes `bring_world` copy the world in and
`nbt::level_player` move the single player's compound, byte for byte, to the
owner's player file.

Mix covers (`mods/cover.rs`): each pack's icon (Modrinth's, or an instance
folder's) becomes a square tile (pixel art enlarged block by block), then
`blend` gives each pack its own part (a soft diagonal for two, wedges for
three to six) filled with a blurred wash of its colours, with its icon whole
as a rounded tile with a shadow. The cover is kept as the mix's
`cover.png` and written into the launcher profile as a `data:` PNG.
`Pollinations` is the optional painted version: the device sign-in
(`/api/device/code`, `/api/device/token`) or a pasted `sk_` key under DPAPI,
then `/v1/images/edits` with `black-forest-labs/flux.2-klein-4b` and the
tiles as inline images.

## Chats

`chats/` brings conversations together. `ChatHub` lists OmniHub's own chats
(`data/chats/omni/<id>.json`) with Claude (through `claude/api.rs`, the same
client as the mod maker) or OpenAI (`openai.rs`: the Responses API streamed,
`store: false`, the key under DPAPI), and the conversations Claude Code and
Codex keep on the PC. `claude_code.rs` reads `projects/*/<session>.jsonl`
(titles, the user's own prompts without reminders or tool output, answers,
tool calls as one-line labels) and reads `claude -p --output-format
stream-json --include-partial-messages`; `codex.rs` reads
`sessions/**/rollout-*.jsonl` (and `.jsonl.zst`, with `ruzstd`), the
`session_index.jsonl` names, and `codex exec --json` events. `agent.rs` runs
either program (message on stdin, no window, the process tree stopped on
Stop) and finds it on PATH, in the registry's PATH and the installers'
folders. Long transcripts are read from their start and end for the list
and from their last 24 MB for a conversation. Answers stream as
`chats:delta` (`text`, `tool`, `note`, `notice`, then `done` or `error`);
`chats:changed` says when to reload. `usage.rs` adds up Claude Code's token
counts by hour (reading only new lines of each file, the scan state kept in
`data/chats/usage-scan.json`), prices them at API rates, works out the
current 5-hour window and the last limit Claude Code noted, and reads
Codex's `token_count` events (its totals and the plan's rate limits).
OmniHub's own use goes into a ledger per day. The phone reaches all of this
through `/api/chats*` and `/api/ideas`.

## Lights (Govee)

`govee/` drives Govee lights: `lan.rs` (scan, JSON commands, `razer` frames
and `ptReal` packets over UDP), `cloud.rs` (Govee's OpenAPI with the user's
key: devices, capabilities, scenes, control), `look.rs` (pure colour maths:
screen zones from a `capture::sampler::Grid`, music styles from the visual
analysis and the cover palette, effects from `rig::lighting`, smoothing) and
`schedule.rs` (timers). `GoveeHub` runs one `govee` thread while OmniHub
controls the lights, a timer is set or the page is open: it scans now and
then, reads states, streams live looks at up to 30 frames a second (a frame
per light only when it changed, a keep-alive each second), sends still looks
once, and fires timers on the local clock. A light's live colours go per
segment (`razer`) or as one colour; `StreamKind::Auto` picks per segment
only when Govee's account lists `dreamViewToggle` for it (or doesn't know
it). Scans go out twice, plus a unicast sweep of the local /24 networks
every five minutes for routers that drop multicast. Cloud calls run on a separate
`govee-cloud` thread with a coalescing queue. `capture::sampler` shrinks the
desktop on the GPU (Desktop Duplication → mipmapped copy → one small level
read back), falling back to GDI capture.


## Minecraft servers

`mcserver/` hosts Minecraft servers. `install.rs` knows each server type's
download site (Mojang's piston-meta, PaperMC's Fill API, Fabric and Quilt
meta and Maven, Forge and NeoForge Maven, Adoptium for Java), checks every
file against its published checksum, runs the loaders' installers, and
works out how each server starts (`Launch`: a jar, or Forge/NeoForge's
argument file) and which Java it needs (`Runtimes` keeps them in
`data/mcserver/java`). `process.rs` runs the server with its console on
pipes: each line is filed (`split`, `read` → `Said`: started, logged in from
an address, joined, left, the player list, chat, a plain-words problem),
and `Proc::ask` sends commands and collects their answers without showing
them in the console. `props.rs` reads and writes `server.properties` the way
Java does. `players.rs` keeps the whitelist, operators and bans, works out
name-made UUIDs (MD5, as servers without account checks do), reads a
player's saved file (`nbt.rs` reads NBT and the SNBT `data get entity`
prints) into position, health and inventory, and asks Mojang's API for a
name's UUID. `guard.rs` is for servers that let players without a paid
account join: it holds the operators (`.omnihub/guard.json`) and the places
each may join from, and moves lists and saved players between UUID kinds
when the server switches. `map.rs` draws a region (512×512 blocks) from the
Anvil files: chunk sections' palettes and packed block states, the
`MOTION_BLOCKING` height map, a colour per block, height shading, water
depth, a scan from below for the Nether; tiles are cached as PNGs in the
server's `.omnihub/map`. `content.rs` adds mods, plugins and data packs
(Modrinth, a file, a modpack's server-side files and overrides, or Claude's
projects) and switches them off by moving them to `.omnihub/disabled`.
`backup.rs` zips worlds (saving paused) and restores them. `net.rs` finds
the PC's addresses, the public address, and talks UPnP (SSDP and SOAP) to
the router.

`ServerHub` keeps the servers in `data/mcserver/servers.json` and their live
state in memory, ticks four times a second (crash restarts, a player list
each minute, positions every two seconds while a window or phone looks,
scheduled backups, the router lease, keeping the PC awake), and emits
`mcserver:changed`, `mcserver:work`, `mcserver:console`, `mcserver:players`,
`mcserver:content` and `mcserver:guard`. Servers that ran when OmniHub
updated itself start again with it (`resume.json`), as do servers set to
start with OmniHub, and ones left running by a crash are ended at start
(`running.json`). The phone reaches
it through `/api/servers*`.
