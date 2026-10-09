## New in this version

- **Boost modes for every game.** Pick how far Play goes, per game:
  - **Competitive** — the most FPS and the least delay. Every boost switch on, and the game's own graphics set to their fastest.
  - **Quality** — everything that doesn't change the picture. The game's graphics are never touched, so Cyberpunk 2077 at 4K Ultra stays 4K Ultra.
  - **Custom** — each switch your way.
  
  Everything a boost changes comes back when the game closes.
- **Pro settings for more games.** Besides Fortnite and Minecraft, OmniHub now sets **VALORANT**, **Counter-Strike 2** (every Steam account on the PC), **Apex Legends**, **Overwatch 2** and **Roblox** for frames: VSync off, no frame cap where the file has one, lowest shadows and effects, NVIDIA Reflex in CS2, Reduce buffering in Overwatch. Each game's page says what it can't change for you (for example CS2's `fps_max`). **Optimize PC** lists every game in one place with **Optimize all**. **Put back mine** restores your own settings.
- **Less delay while you play** (only during the game, then put back):
  - closes background junk — OneDrive, Google Drive, Dropbox, Widgets, Phone Link, Adobe/Java/Edge/Google updaters — and starts the sync apps again afterwards (Discord and other chat apps stay open);
  - browsers, Steam/Epic web views, Spotify and sync apps drop to "Below normal" priority, so the game gets the processor first;
  - Windows' finest timer (0.5 ms) for steadier frame pacing;
  - Windows never moves the game to power-saving cores or slows it down.
- **FPS meter.** Every frame, measured by Intel PresentMon (the tool hardware reviewers use): live FPS, 1% low and frame time while you play — on the PC and on your phone — and a summary afterwards (average, 1% and 0.1% lows, stutters), kept per game so you can compare boosts. It works with anti-cheat and doesn't touch the game. Turning it on asks Windows for permission once; it counts from your next sign-in.
- **Lag under load.** The ping helper now pings while the line is busy downloading and then uploading, grades the result (A+ to F) and says what to do about it — usually Smart Queue/SQM on the router, a cable instead of Wi-Fi, or pausing downloads.
- **More in Optimize PC:** a precise timer for games (Windows 11), no network throttling during media, and no Sticky Keys pop-up when you hold Shift. None of the PC-wide settings lowers picture quality.
- Overwatch 2 and Cyberpunk 2077 profiles; Cyberpunk starts in Quality mode.

About ping and frame rates, honestly: no app can make your ping 0 — the distance to the game's server is the floor, so pick the closest region. And FPS tops out where your processor and graphics card do. OmniHub removes everything else in the way, and the FPS meter shows what you actually get.

Updating from 0.2.1 or later: OmniHub installs this by itself (or Settings → About → Install now), or download the setup below and run it.

## Install

1. Download **`OmniHub_{{VERSION}}_x64-setup.exe`** below.
2. Run it. It installs for your Windows user only, so no administrator rights
   are needed (unless an older OmniHub is in Program Files: then it asks once). Windows 10 or 11; WebView2 is installed automatically if it is
   missing.
3. The installer is not code-signed yet, so Windows SmartScreen will warn
   you: click **More info → Run anyway**.

Prefer an MSI (for example for deployment tools)? Use
`OmniHub_{{VERSION}}_x64_en-US.msi` instead.

The phone app needs no install: in OmniHub open **Phone → Pair a phone** and
scan the QR code with your phone (same Wi-Fi network).

iPhone mirroring uses the AirPlay add-on (`OmniHub-AirPlay-addon-x64.zip`,
UxPlay with its GStreamer runtime). You don't need to download it: OmniHub
fetches and checks it when you click **Install AirPlay receiver** on the
Screen share page. Its source is in `OmniHub-AirPlay-addon-sources.zip`.

The FPS meter uses Intel PresentMon {{PRESENTMON}} (`OmniHub-PresentMon-x64.exe`, MIT license in
`OmniHub-PresentMon-LICENSE.txt`). You don't need to download it either: OmniHub fetches and checks
it when you turn on the FPS meter in Games.

`SHA256SUMS.txt` lists the SHA-256 checksum of each file.
