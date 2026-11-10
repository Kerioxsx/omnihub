## New in this version

**OmniHub Client for Minecraft 1.8.9, and Fortnite resolution advice.**

- **OmniHub Client (Mods → Minecraft)**: one click sets up Minecraft 1.8.9 with Forge as a profile in the official Minecraft Launcher (sign in with Microsoft as usual; works on Hypixel and any server). Pick from the best open-source PvP mods: **PolyHitbox** (coloured hitboxes), **PolyCrosshair** (draw your own crosshair), **DamageTint** (hit colour), **OverflowAnimations** (1.7 animations), **OverflowParticles**, **PolyPatcher** (FPS fixes), **EvergreenHUD** (FPS, CPS, keystrokes…), **Resourcify** (download resource packs inside Minecraft), **PolySprint** and **Freelook**. PvP-friendly video settings out of the box.
- **OmniHub Kill Effects**: our own mod. Blood, lightning, flames, firework, souls, hearts or an explosion when a player or mob you hit dies, with your colour, sound and size, plus a "☠ Kill" mark with streaks. Change it in OmniHub while you play; it applies within seconds.
- **Resource packs**: search 1.8.9 packs on Modrinth and add them to the client from OmniHub.
- **Fortnite: recommended for your PC**: from your monitor, graphics card and processor, the display resolution, 3D resolution and frame cap that should hold your refresh rate in Performance mode, with the reasons. One click to use it.
- **Fortnite display resolution**: set a lower resolution for more FPS (fullscreen), and HDR output is turned off with the other tweaks.

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
