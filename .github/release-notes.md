## New in this version

**The rest of the redesign plan: find a setting, search that does more, two more themes.**

- **Find a setting**: a search box in Settings shows only the rows that match.
- **Search (Ctrl+K) does more**: play/pause, next and previous track, mute; `timer 10` starts a timer; your light presets, smart home devices and other PCs by name. Real matches now come first.
- **Two more Bleach-inspired themes**, shapes and colour only: **Dangai** (the tunnel between worlds) and **Sōkyoku Hill** (a cliff at sunset). That makes six, each with an optional picture of your own.
- **Photo gallery**: **Add a folder** brings in all its pictures at once, and there's a **Dark edges** option.
- **Big Picture**: **Karaoke** from the Room tab.
- Fixed: emulator disc images were given the wrong console when a folder far above them (even your Windows user name) contained "ps2", "psp" and the like.

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
