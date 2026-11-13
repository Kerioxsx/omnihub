## New in this version

**Play resolution: switch the whole PC to 4K, 1440p (2K), 1080p or stretched 4:3 for a game.**

- **Where:** Games → Extras → Play resolution. Pick a size from the ones your screen offers (4K, 1440p (2K), 1080p, 900p, 720p, ultrawide, stretched 4:3 and more).
- **Per game:** "Use with this game" switches the PC when that game starts and puts your normal resolution back when it closes. Or press "Switch now" / "Back to normal" yourself.
- **Made for windowed fullscreen (borderless):** the game uses the PC's resolution, so there's nothing to change in the game and alt-tab stays instant.
- **Faster switch:** OmniHub keeps your refresh rate and colour depth and asks the graphics card to do the scaling, so the monitor doesn't have to re-sync (no long black screen). It does nothing if you're already at that size, and it notices a game starting within about 4 seconds instead of 15.
- 4K on a 1080p or 1440p monitor shows up once NVIDIA DSR or AMD VSR is turned on in the driver. For stretched 4:3, set scaling to full screen (GPU) in the driver.
- The change lasts for this session only; Windows keeps your own resolution for the next start.
- Not tested on a real Windows PC yet (built and checked here without a monitor). If a switch still blacks out the screen, your driver may not allow GPU scaling at that size.

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
