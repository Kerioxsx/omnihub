# Security model

OmniHub touches your whole disk, your passwords and your PC's power button,
and lets a phone do some of that. This is what protects each part.

## Privileges

- The app runs as a **standard user**. Nothing in normal use needs
  administrator rights: power actions use the interactive user's shutdown
  privilege; screenshots, apps, notes and the vault are per-user.
- **Fast scan** (MFT) needs a raw volume handle. Each fast scan asks through
  UAC and runs a short-lived helper (`--omnihub-helper scan-volume`) that
  only reads the volume and writes a snapshot. Its arguments are validated
  (single drive letter, output folder must be OmniHub's scan cache, no
  junctions or symlinks on the path) so another program cannot use the
  helper to write elsewhere with admin rights. The snapshot holds file names
  and sizes only.
- The same helper does a few other one-off admin jobs, each asked for
  through UAC and each with strictly validated arguments: allow OmniHub
  through the firewall / mark a network private (`firewall-allow`,
  `network-private`), all-users startup entries (`startup-set`), the
  Equalizer APO include file (`eq-write`, two numbers and a flag), and a
  game's network and start priority (`game-admin`, see Games below).
- **Approve fast scans once** (off by default): one UAC prompt creates a
  Windows scheduled task, "OmniHub Fast Scan", that runs
  `OmniHub.exe --omnihub-helper scan-queue <scan cache>` with the user's
  highest privileges. To scan, the app drops a request (an id and a drive
  letter) into the cache's `queue` folder and starts the task; the task
  runs only the same validated scan job as the UAC helper and writes the
  result back. Any program running as you could also queue a scan, which
  would put file names from the whole drive (other users' folders too)
  into your scan cache — so it is meant for PCs only you use. Turning it
  off deletes the task (one more prompt).
- "Restart as administrator" exists for people who prefer one prompt per
  session; drag-and-drop from Explorer does not work into an elevated app.

## Vault

- Entries are serialised and encrypted with **AES-256-GCM** under a random
  256-bit vault key. The vault key is wrapped with a key derived from the
  master password by **Argon2id** (64 MiB, 3 passes, 4 lanes). Both
  ciphertexts authenticate the format header.
- On Windows the whole file is additionally wrapped with **DPAPI** (current
  user), so a copied `vault.ohv` cannot even be attacked offline without the
  Windows account's credentials. Encrypted backups (export) omit DPAPI so
  they can be restored on another PC with the master password.
- Decrypted entries exist only in memory while unlocked; secret strings are
  zeroed on drop. The vault locks after idle time (default 5 minutes), when
  Windows locks, or by hand. Wrong passwords are throttled (exponential
  back-off after three failures).
- **Clipboard:** copied secrets are marked to stay out of Windows clipboard
  history and cloud clipboard, and cleared after 20 s if still on the
  clipboard.
- **Windows Hello** (optional): a Hello key pair signs a fixed random
  challenge; the SHA-256 of that deterministic signature wraps the vault key
  (`vault.hello`, also DPAPI-wrapped). Unwrapping needs a visible Hello
  prompt, unlike a plain DPAPI copy of the key, which any program running as
  you could decrypt silently. The master password always works.
- **Primary accounts:** the UI warns when an entry looks like a Google,
  Microsoft, Apple, Yahoo or Proton account and recommends passkeys, app
  passwords and 2-step verification instead of storing that password.

## Browser autofill

- The extension (in `browser-extension/`, loaded unpacked) has no access to
  the vault file. It talks to the OmniHub app through the browser's
  **native messaging**: the browser starts OmniHub's executable as a host
  (on Windows a copy of it in OmniHub's data folder, so it never locks the
  installed program), and the host relays requests to the running app over a per-user named
  pipe. The host manifest allows exactly one origin, the extension's fixed
  ID (`chrome-extension://hfkbdbcemgoondnmkeeoclpcmcjjbdeg/`).
- Each browser profile must be **paired**: the app shows a 4-digit code that
  must match the one in the extension, and the PC user approves it. Paired
  browsers can be revoked in Vault → Browser autofill.
- The vault must be unlocked in the app. Logins are offered only on pages
  whose registrable domain (public-suffix aware) matches the entry's site,
  so `netflix.com.evil.io` never sees a Netflix login; a site's subdomains
  share its logins. The fill menu lives in a closed shadow root.

## Phone companion

- **Off by default.** When on, it listens on the LAN (or only on localhost)
  and **rejects every peer that is not a private address** (RFC 1918,
  link-local, loopback, IPv6 ULA; Tailscale's 100.64.0.0/10 only if you
  allow it), regardless of firewall settings.
- **DNS rebinding:** requests must carry an IP literal, `localhost` or this
  PC's own name in the `Host` header.
- **HTTPS** by default with a certificate generated on this PC (its SHA-256
  fingerprint is shown in the app so you can compare it with what the phone
  shows). Plain HTTP is a setting for networks you trust; vault access is
  refused over HTTP.
- **Pairing** needs someone at the PC: the desktop opens a 5-minute window
  with a QR code (128-bit secret) and a 6-digit PIN; one device per window,
  five wrong PINs close it, attempts are rate-limited per address. Pairing
  returns a 256-bit device token; only its SHA-256 is stored. Devices can be
  renamed and revoked; revocation also ends their sockets and vault
  sessions.
- **Least privilege for files:** by default the phone sees only Desktop,
  Documents, Downloads, Pictures, Videos, Music and the "From phone" folder.
  Every path is canonicalised and must stay inside a shared root (no `..`,
  no following junctions out). Download links and socket URLs carry
  short-lived tickets, never the token.
- **Power** actions that end the session need an explicit confirmation from
  the phone and run after a countdown (default 10 s) shown on the PC with a
  Cancel button.
- **Remote control** (mouse/keyboard) is a separate opt-in, off by default;
  the PC shows who is viewing and controlling and can stop it. Windows does
  not let a standard-user app inject input into elevated windows.
- **Tasks** (on by default, can be turned off): the phone sees running
  programs and can change priority or end one; Windows' own processes and
  OmniHub itself are refused, priority never goes to Realtime. **Open apps**
  follows the same permission: it lists programs with a window (names and
  window titles, never paths) and can ask one to close (`WM_CLOSE`, the same
  as clicking ×) or quit it; Explorer is never ended.
- **Music** control and **Games** (start a game with its boost) follow the
  "Music" and "Launch apps" permissions. **Volume & calls** follows "Music":
  each app's volume and mute, the master volume, and muting every recording
  device. Which apps are in a call comes from Windows' audio sessions (a
  calling app with an active recording session); OmniHub never records or
  listens. It cannot hang up a call — no app offers that to others.
- **Vault on the phone** is a separate opt-in, HTTPS only, needs the master
  password on the phone each time, and gives that phone its own decrypted
  copy for 5 idle minutes (the PC's vault stays locked). Listing never
  includes secrets; every reveal is logged.
- **Audit log:** pairing, revocation, downloads, uploads, power requests and
  cancellations, app launches, apps closed or quit from the phone,
  microphone muted or unmuted, screen sharing and control, vault unlocks and
  reveals, deletions from the desktop. Kept 180 days.
- Content-Security-Policy, `X-Frame-Options: DENY`, `nosniff` and
  `no-referrer` on every response.

## Games

- A boost changes only documented Windows settings and puts the session
  ones back when the game closes: the power plan (it may add Windows'
  built-in "Ultimate Performance" plan once), notification pop-ups
  (`ToastEnabled`), Game Mode, programs you chose to close and the fixed
  list of background programs "Close background junk" closes (cloud sync,
  Widgets, Phone Link, updaters — `games/junk.rs`; sync apps are reopened
  afterwards, chat apps are never on it), browsers and similar programs
  set to "Below normal" priority and back to "Normal", the game's priority
  and power-throttling opt-out (`SetProcessInformation`), a 0.5 ms timer
  request (`NtSetTimerResolution`, released with the session, or by Windows
  when OmniHub exits), and Wi-Fi background scanning / streaming mode (held
  through a WLAN handle; Windows reverts it when the handle closes, even if
  OmniHub crashes). What it changed is written to a journal first, so a
  crash or power cut is undone at the next start. Quality mode never
  changes a game's own graphics settings.
- Per-game settings stay until turned off in the profile: the
  high-performance GPU choice and "disable fullscreen optimizations" (both
  per-user registry, the same keys Windows' own settings write), and — with
  one UAC prompt — a QoS policy marking the game's traffic DSCP 46 and an
  Image File Execution Options `PerfOptions\CpuPriorityClass` so Windows
  starts the game at High priority. The helper accepts only plain `.exe`
  names for these.
- Game settings (Fortnite, VALORANT, Counter-Strike 2, Apex Legends,
  Overwatch 2, Roblox, Minecraft): OmniHub writes values the game's own
  settings menu writes, into the game's settings files in your profile
  (for example
  `%LOCALAPPDATA%\FortniteGame\Saved\Config\WindowsClient\GameUserSettings.ini`,
  Steam's `userdata\<account>\730\local\cfg\cs2_video.txt`,
  `%APPDATA%\.minecraft\options.txt`) — never the game's program files —
  and only while the game is closed. Only Fortnite's standard Unreal Engine
  keys are added when missing; other values change only when the game
  already wrote them. A copy of each file from before its first change is
  kept in `data\game-settings-backup` for "Put back mine".
- Optimize PC changes, one click each and journaled in `data\pc-tweaks.json`
  so each can be undone: the monitor's refresh rate (the same as Windows'
  display settings), the active power plan, Game DVR
  (`GameDVR_Enabled`, `AppCaptureEnabled`), Game Mode, the DirectX
  `SwapEffectUpgradeEnable` setting, mouse acceleration (`SPI_SETMOUSE`),
  the Sticky Keys shortcut (`SPI_SETSTICKYKEYS`), and — with a UAC prompt —
  `HwSchMode` for GPU scheduling, `GlobalTimerResolutionRequests` (Windows
  11) and the multimedia `NetworkThrottlingIndex`/`SystemResponsiveness`.
  Memory Integrity is only reported, with a link to Windows Security.
  Memory speed is only reported too: the speed each stick runs at
  (SMBIOS type 17) against the speed its part number says the kit is sold
  at, with a link to Windows' Recovery settings; EXPO and XMP are BIOS
  settings OmniHub never changes.
- FPS meter: Intel PresentMon, downloaded from this repository's release and
  checked against a SHA-256 built into the app, runs as you while the game
  plays and reads the frame events Windows records (ETW); it never opens the
  game. Windows allows that to administrators and "Performance Log Users":
  turning the meter on asks once (UAC) to add your account to that group
  (`--omnihub-helper fps-admin allow <account>`, which runs `net localgroup`
  with the group's name looked up from its SID). Remove yourself from the
  group in Computer Management to undo it.
- NVIDIA settings for the game (on in Competitive mode): with the same
  one-time UAC prompt, the elevated helper (`nv+:Game.exe`) sets five
  values on the game's profile in the NVIDIA driver through NVAPI's
  documented driver-settings interface (`nvapi64.dll`, function and setting
  ids from NVIDIA's open SDK headers): power management, maximum
  pre-rendered frames, texture filtering quality, vertical sync and the
  driver frame cap. A value the user set on that profile themselves is left
  alone. Turning it off (or deleting the profile) restores NVIDIA's
  defaults for those five (`nv-:`). Reading them needs no elevation.
- The boost no longer waits behind the administrator prompt: the prompt is
  owned by OmniHub's window (so it opens in front), and after 25 seconds
  the game starts without it; the prompt's answer still lands in its step.
- Nothing touches a game's memory or program files, so anti-cheat has
  nothing to object to. Besides the settings files above, the exception is
  Roblox's own settings file,
  `ClientSettings\ClientAppSettings.json`, where OmniHub writes Fast Flags
  (keeping any flags it did not write).

## Lights (Govee)

- Local network only, unless the user adds a Govee API key: discovery is the
  documented LAN API scan (multicast to 239.255.255.250:4001 from each local
  network, plus any addresses the user adds), answers arrive on UDP 4002,
  commands go to the light's UDP 4003. Live colours use the `razer`
  (DreamView) sub-protocol to the same port. Nothing is sent to other hosts.
- Windows Firewall: the page checks whether answers on UDP 4002 get through
  and can add an allow rule for OmniHub on private and domain networks (the
  same `firewall-allow` helper as the phone companion, one UAC prompt).
- Screen sync reads the chosen monitor through Desktop Duplication and keeps
  only a grid of about 100 × 60 average colours, overwritten every frame;
  nothing is stored or sent except the lights' colours.
- Music sync uses the same sound analysis as the music visuals (levels and
  beats only, nothing recorded) and the current song's cover from the media
  module, reduced to five colours in memory.
- The Govee API key is optional. It is stored with DPAPI for the Windows user
  (entropy `omnihub-govee`), never shown to the window again, never in
  logs, and sent only in the `Govee-API-Key` header to
  `https://openapi.api.govee.com`. Settings patches from the window cannot
  write it. Adding, replacing or removing it is in the audit log.
- Cloud commands are queued one at a time and spaced ~0.5 s apart (Govee
  allows about two a second per light); a dragged slider sends only its last
  value.

## My PC: lighting, fans, screens and specs

- Specs are read, never written: the firmware's SMBIOS table
  (`GetSystemFirmwareTable('RSMB')`: motherboard, case type, memory sticks),
  the registry (processor, `HARDWARE\DEVICEMAP\Scsi` disk names), DXGI
  (graphics cards) and the list of USB/HID devices present
  (`CM_Get_Device_ID_List`, matched against a fixed table of coolers and
  hubs). No administrator rights, no driver, nothing sent anywhere.
- OmniHub loads no driver of its own for lighting or fans. It talks to
  programs the user installs and runs: **OpenRGB** over its SDK protocol on
  `127.0.0.1` (port 6742 by default; colours and "direct" mode only — no
  profiles, settings or firmware commands are sent), **LibreHardwareMonitor**
  over its remote web server, and **liquidctl** as a command-line program.
  OmniHub only ever connects to LibreHardwareMonitor on `127.0.0.1`,
  `localhost` or `[::1]`: another address is refused, so fan controls are
  never sent to, or read from, another machine. Its optional password is
  kept encrypted for the Windows account (DPAPI).
- Fan safety: no fan is set under 30% (pumps 50%), every controlled fan runs
  at 100% from 90 °C, a fan with no temperature to follow is handed back to
  the hardware, speeds fall at most 3% per two seconds, and on quit every fan
  OmniHub set goes back to the hardware's own control (`value=null`).
  Coolers driven through liquidctl get a temperature profile they run
  themselves; opening the page changes nothing until a profile is picked.
- "Start OpenRGB" runs OpenRGB from where its installer put it (Program
  Files or the user's programs folder), as the user, with `--server
  --startminimized`.
- Screen media: a picture or video the user picks in Windows' file dialog is
  copied into `data\rig-media` only if its first bytes are a PNG, JPEG,
  GIF, WebP, MP4 or WebM (40 MB for pictures, 1 GB for videos); the window
  never passes a path. The case-screen window gets files through
  `rigmedia://` by library id only (a UUID and a known extension; nothing
  else resolves), with byte ranges.

## Aurora and the PC's sound

Aurora (the music player's light, visual and floating lyrics) reacts to what
the PC plays. How that works, and what it never does:

- **What it hears.** WASAPI *loopback* on the default output device: the
  mix Windows is already sending to your speakers or headphones. It is not
  the microphone, Windows asks no permission for it, and OmniHub never opens
  a recording device for Aurora.
- **When.** Only while a view shows Aurora (the player, the Music page or
  the glow around the screen). Each view holds a lease it renews every few
  seconds; ten seconds after the last one goes, capture stops and the
  device is released. Nothing listens in the background.
- **Where it goes.** Nowhere. Samples live in memory for one 2048-sample
  analysis window (about 43 ms) and become a handful of numbers (loudness,
  bass, mids, highs, beats, 16 bands) that go to OmniHub's own windows as
  `audio:frame` events. Sound is never written to disk, never recorded and
  never sent anywhere; the phone does not receive these events.
- **Off.** *Follow the music* (in Aurora's settings) stops capture at once;
  *Aurora on* off stops capture and all drawing. Without capture (turned
  off, no output device, an unsupported format) the light moves on its own.
- **Lyrics.** Your own `.lrc` folder is only read. Online lookups send the
  song's title, artist, album and length to LRCLIB (see Internet).
- **The glow around the screen** is a see-through window per monitor that
  never takes focus and lets every click through. It hides while a game,
  video or presentation is full screen and during game boosts.
- **Keep the screen on** asks Windows not to turn the display off only while
  the player shows Aurora and music plays, and lets go when either stops.
- **Music videos.** With *Music videos from YouTube* on (the default) and a
  background that can show one, OmniHub looks the song up on YouTube: the
  song's title and artist go to `www.youtube.com/results` as a search, sent
  with a "reject all" consent cookie and no account. Only the answer page is
  read (the video ids, titles, channels and lengths); only uploads by the
  artist's own channel or VEVO are kept, and the answer is cached on the PC
  (a month; three days when there is none). The video then plays in
  YouTube's embedded player from `www.youtube-nocookie.com`, muted. YouTube
  refuses official videos to players embedded by a local address (the app's
  window is `http://tauri.localhost`), so the player sits in a small static
  page on OmniHub's own website (`site/player.html`, served by GitHub
  Pages), which embeds it as any website does and only relays the player's
  own messages (play, pause, seek, speed, mute; back: time, state, quality)
  between it and the app — it accepts messages only from OmniHub's windows
  and only from YouTube's frame, and stores nothing. No YouTube script runs
  in OmniHub's page; the app's content security policy lets frames come
  only from that page. GitHub sees the request for the page (like any
  visit). Turn the toggle off, or pick *Cover* or *Light show*, and nothing
  is searched or played.
- **Videos YouTube will not embed** (*Play videos YouTube won't show in
  apps*, on by default with music videos): when the artist allows a video
  only on YouTube itself, OmniHub asks **yt-dlp** for the video's stream
  address and plays it muted in a `<video>`. yt-dlp and **Deno** (the
  JavaScript runtime yt-dlp needs for YouTube) are downloaded the first time
  this is needed, from their projects' latest GitHub releases, into
  `data/tools/video`, and kept only when they match the SHA-256 published
  with that release (yt-dlp's `SHA2-256SUMS`, Deno's `.sha256sum`). yt-dlp
  updates itself (`-U`, at most once a day; its updater checks the
  release's checksums too). They run hidden, with the video's id only; the
  stream is never saved. The content security policy allows media only from
  `*.googlevideo.com` (and Apple's video servers, below).
- **Animated covers** (*Animated covers*, on by default): the song's title
  and artist go to Apple Music's catalogue (`amp-api.music.apple.com`), with
  the public key Apple's own web player at music.apple.com uses, read from
  that page and kept until it expires (no account). The album's "editorial
  video" plays muted from `mvod.itunes.apple.com` (hls.js, in the page; no
  worker). Answers are cached for a month (misses for three days).
- **Emojis** beside lyrics are Microsoft's Fluent Emoji (MIT licence: the
  3D pictures in `public/emoji/3d`, the one-colour drawings in
  `public/emoji/plain`, each with its `LICENSE.txt`), shipped inside the
  app; none are fetched.
- **Matching videos to the song** (*Match videos to the song by ear*, on
  with music videos): the video's soundtrack is downloaded with yt-dlp (the
  smallest AAC stream, at most 40 MB) into `data/videos/soundtracks`,
  decoded in-process (symphonia) and deleted at once; while it listens, the
  light show's loopback capture also hands its sound to the matcher (onset
  envelopes only, in memory). Only the offset found is kept
  (`data/videos/sync.json`, per video and song length).
- **Mirroring the music app** (*Mirror Spotify's Canvas and videos*): while
  Aurora shows and Spotify or Apple Music plays, their window is captured
  (the same window capture as screen sharing, `xcap`) about 12 times a
  second, only to find and crop the moving picture; the crop is kept in
  memory as one JPEG for the window to fetch and is never saved or sent.
  Only the playing app's own window is captured.
- **Your font & emoji pack**: a font file and an emoji pack (a zip of
  pictures, or an emoji font) the user picks in Windows' own file dialog (the
  window never hands Rust a path). Fonts are kept only if their first bytes
  say TrueType, OpenType, a collection or WOFF (20 MB at most). From a zip
  only pictures are taken, each checked by its first bytes (PNG, GIF, WebP,
  JPEG; SVG must be an `<svg>` document), by its file name alone (lower case,
  letters, digits and `_`, other characters as code points; folders are
  ignored, so nothing can be written outside `data/aurora-look/textures`),
  each up to 4 MB, 400 MB and 60,000 entries in all, into a temporary folder
  that replaces the last set only when the import succeeds. They are read
  back as data URLs and shown only as images or `@font-face` fonts (SVG in an
  `<img>` runs no script; the content security policy already allows `data:`
  fonts and images). They never leave the PC, and OmniHub ships none.

## Deleting files

Cleanup only suggests. Deletion goes to the Recycle Bin unless you choose
"Delete permanently" and type a confirmation. Windows, Program Files,
ProgramData\Microsoft, drive roots, system files at the root and your user
folder itself are refused outright; Windows temp, update download cache,
crash dumps and delivery optimisation cache are the only allowed system
locations. Names that could not be decoded exactly are never passed back to
the OS.

## Internet

There is no cloud service and no account. OmniHub itself contacts the
internet only for:

- **Lyrics** (Music, on by default, can be turned off): the title, artist,
  album and length of the playing song go to [LRCLIB](https://lrclib.net),
  and, when LRCLIB has nothing timed for it, the title and artist go to
  NetEase Cloud Music's public search (`music.163.com`); results are cached
  on the PC (a song without lyrics is asked again after three days).
- **Sharper covers** (Music, on by default, can be turned off): the title,
  artist and album of the playing song go to Apple's iTunes Search API and
  to Deezer's public search (`api.deezer.com`), and the matching cover is
  downloaded at its original size (Apple: usually 3000×3000; Deezer:
  1900×1900; at most 16 MB); covers are cached on the PC.
- **Music videos** (Aurora, on by default, can be turned off): the title and
  artist of the playing song go to YouTube's search page, and the artist's
  video plays from youtube-nocookie.com through OmniHub's relay page, or
  directly with yt-dlp (see Aurora above).
- **Animated covers** (Aurora, on by default, can be turned off): the title
  and artist go to Apple Music's catalogue (see Aurora above).
- **Breach check** (Vault → Health, only when you run it): the first five
  characters of each password's SHA-1 go to Have I Been Pwned's range API
  (k-anonymity); passwords never leave the PC.
- **AirPlay add-on** (only when you install it): downloaded from this
  repository's releases and checked against a SHA-256 built into the app.
- **Ping helper** (only when you run it): ICMP echo requests to the game's
  regions or the host you entered. **Lag under load** (only when you run
  it) also downloads from and uploads zeros to Cloudflare's speed test
  (`speed.cloudflare.com`) for about 16 seconds.
- **FPS meter** (only when you turn it on): PresentMon from this
  repository's release, checked like the AirPlay add-on.
- **Updates** (on by default, can be turned off): GitHub's "latest release"
  API for this repository, and then the installer and `SHA256SUMS.txt` from
  that release. The installer must come from this repository's releases and
  match its listed SHA-256 before it runs. The checksum guards against
  broken or swapped downloads, not against someone who controls the
  repository itself; code signing is not set up yet. Automatic installs
  wait while a game boost, screen sharing or a phone upload is running.

## Not in v1

No cloud relay. No Windows service (a service would run
the phone server as SYSTEM; instead the app starts at login, minimised to
the tray). The installer is not code-signed yet, so SmartScreen will warn.
