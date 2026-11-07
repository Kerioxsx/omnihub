## New in this version

**Faster games, emulators done properly, a second PC, a weekly report and your smart home.**

Performance
- **Latency to the screen** next to FPS (from PresentMon), with how frames reach the screen.
- **Smart frame cap** (a game's boost settings): a cap just under your monitor's refresh rate in the game's NVIDIA profile, so G-Sync/FreeSync stays engaged.
- **Variable refresh rate in windowed games** (Games → Optimize PC).
- **Pause background apps while playing** and **every core awake** (core parking off) as boost options; **clear standby memory** (Tools → PC checks).
- **Heat report** after each boosted game: how hot the processor and graphics card got, and whether they likely slowed down.
- **Overlay and shader-cache checks** in "Ready to play?", a **repeatable benchmark** (Tools → PC checks), a **mouse polling-rate test** and **stretched resolution per game** (Games → Extras).
- **Fortnite update and downtime alerts** (Games → Extras).

Emulators (Games → Emulators)
- **Install and update emulators** from their official GitHub releases (DuckStation, PCSX2, melonDS, mGBA, Cemu, Azahar, RPCS3), each file checked against GitHub's checksum.
- **BIOS checker** for the BIOS files you dumped yourself (nothing is downloaded), and a **ROM organizer** that checks a No-Intro or Redump DAT, renames to the proper names (saves follow) and finds duplicates.
- **Saves and memory cards** in one list with a one-click backup; **controller setup** for DuckStation and PCSX2.
- **Per-game options**: fullscreen, a RetroArch shader, extra options, **cheats**, and **RetroArch netplay**.
- **RetroAchievements** with your own key, **ROMs in the Library**, and **your phone as a controller** (More → Controller).

Everything else
- **Hide a whole folder** in the vault (each file encrypted, put back where it was).
- **Game deals** across stores (Games → Extras, CheapShark, US dollars).
- **Your other PCs** (Phone page): pair a second PC running OmniHub, see how it's doing, lock, sleep, restart or shut it down, and control its music.
- **Weekly report** every Sunday evening (Games → Extras).
- **Smart home** (Lights → Smart home): Home Assistant and Shelly plugs, from scenes and deck buttons too.

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

Instant replay uses FFmpeg {{FFMPEG}} (`OmniHub-ffmpeg-x64.exe`, LGPL; license and source links in
`OmniHub-ffmpeg-LICENSE.txt`). OmniHub fetches and checks it when you turn on instant replay.

The FPS meter uses Intel PresentMon {{PRESENTMON}} (`OmniHub-PresentMon-x64.exe`, MIT license in
`OmniHub-PresentMon-LICENSE.txt`). You don't need to download it either: OmniHub fetches and checks
it when you turn on the FPS meter in Games.

`SHA256SUMS.txt` lists the SHA-256 checksum of each file.
