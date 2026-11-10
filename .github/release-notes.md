## New in this version

**Redesign, step 2: more themes and a photo gallery.**

- **Four new Bleach-inspired themes**, drawn with shapes and colour only: **Soul Society** (daylight over white walls, spirit lights rising), **Hueco Mundo** (white dunes, a black sky, a crescent moon), **Karakura night** (a town skyline under a full moon) and **Senbonzakura** (a thousand pink blades drifting). Each can show **a picture of your own** faded behind the cards (Settings → Appearance → Your picture). OmniHub still ships no artwork from any series.
- **Photo gallery**: "Your photo" now keeps up to 24 photos. Click one to use it, or let OmniHub **change the photo** every 15 minutes, hour or day.
- **Photo options**: darken, blur and a slow drift.
- The phone's **Match PC** knows the new themes.

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
