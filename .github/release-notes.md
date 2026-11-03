## New in this version

**Updates that always finish, and three Bleach themes.**

- **Fixed: an update could close OmniHub without installing the new version**, so you had to download it again from the website. When OmniHub took a few seconds to close (saving Minecraft worlds, resetting fans), the installer closed itself along with it. Now the installer waits for OmniHub to close by itself, and only then closes anything still left. If an update ever doesn't finish, OmniHub says so on its next start and offers to try again, and it won't keep retrying that version on its own.
- **Bleach themes** (Settings → Appearance), in three modes you can switch from the sidebar while one is on:
  - **Bankai**: black and orange, a Getsuga crescent of energy and rising reiatsu.
  - **Hollow**: bone white with manga ink and crimson mask stripes.
  - **Mugetsu**: ink black with a sparse purple glow.
  Condensed headings, angular accents, and a quick slash across the buttons on hover and when you switch modes (none if you turn on reduced motion). Everything is drawn by OmniHub; no artwork from the series is included.

The fix is in the installer, so it already applies to this update.

Also in 0.26.0: quick actions (a macro deck for the PC and phone, a quick-launch wheel, clipboard history), focus sounds, batteries, the PC health score and Tidy Downloads.

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
