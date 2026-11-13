## New in this version

**A Client tab for Vesper, resource packs inside Minecraft, an FPS boost, and OmniHub is now MIT-licensed.**

- **Client tab:** Vesper Client has its own tab in OmniHub (Play → Client): one-click install as a profile in the official launcher, the jar for doing it by hand, and exactly what it needs.
- **What Vesper needs:** Minecraft Java Edition **1.8.9**, loader **Forge 1.8.9-11.15.1.2318** (not Fabric, Legacy Fabric or NeoForge), Java 8 from the launcher, and **no other mods**. PolyPatcher is optional for more FPS.
- **Vesper 0.4.0 — resource packs inside the game:** Client → Resource packs (or the new button on Minecraft's Resource Packs screen) searches Modrinth's 1.8.9 packs and downloads one in a click, checked against Modrinth's checksum.
- **Vesper 0.4.0 — FPS boost:** one switch for fast video settings; your own settings come back exactly when you turn it off.
- **Open source:** OmniHub, Vesper Client and OmniHub Kill Effects are now under the MIT licence.
- The pack browser was tested in Minecraft; the FPS boost hasn't been measured on a real graphics card yet.

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
