## New in this version

**Change your vault's master password, and six animated themes.**

- **Change master password** (Vault → the key button, or Settings → Vault → Master password): type the current one, then the new one twice. Only the key that unlocks the vault is re-wrapped, so your logins, hidden files and locked notes stay as they are and Windows Hello keeps working.
- **Animated themes** (Settings → Appearance → Theme): Aurora (northern lights), Synthwave (a neon sun over a moving grid), Ocean (slow waves), Galaxy (twinkling stars and nebulas), Sakura (falling cherry blossom, light) and Ember (rising sparks). Each brings its own colours. Classic keeps light, dark and your accent colour.
- The backgrounds only move while OmniHub is on screen, hold still with Reduce motion (OmniHub's or Windows'), and can be stopped with “Animate the background”. They use the graphics card for transforms only, so they stay light on games running alongside.

Also in 0.22.0: scan with your phone, barcode to PC, whiteboard, approve on your phone, downloads, and image and video tools.

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
