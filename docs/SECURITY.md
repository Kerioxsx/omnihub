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
- FPS meter: Intel PresentMon, downloaded from this repository's release and
  checked against a SHA-256 built into the app, runs as you while the game
  plays and reads the frame events Windows records (ETW); it never opens the
  game. Windows allows that to administrators and "Performance Log Users":
  turning the meter on asks once (UAC) to add your account to that group
  (`--omnihub-helper fps-admin allow <account>`, which runs `net localgroup`
  with the group's name looked up from its SID). Remove yourself from the
  group in Computer Management to undo it.
- Nothing touches a game's memory or program files, so anti-cheat has
  nothing to object to. Besides the settings files above, the exception is
  Roblox's own settings file,
  `ClientSettings\ClientAppSettings.json`, where OmniHub writes Fast Flags
  (keeping any flags it did not write).

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
  album and length of the playing song go to [LRCLIB](https://lrclib.net);
  results are cached on the PC.
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
