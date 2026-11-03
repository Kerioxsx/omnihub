## New in this version

**Quiet mode: the screens off and the fans at their lowest, from your phone, with the PC still on.**

- **Quiet mode** (phone: Power; PC: My PC → Fans; or a deck or wheel button): turns every screen off and drops the fans to their lowest safe speed. Downloads, Minecraft servers and your phone keep working.
- **Fans never stop completely**: a running processor would overheat within minutes. They sit at their minimum (30%, pumps 50%) and still speed up if the PC gets warm. The graphics card goes back to its own control, which on most cards stops its fans entirely while it's cool.
- **Ends by itself** when someone touches the PC's mouse or keyboard, or with one tap on the phone. The fans go back to the profile they had.
- **Lowest** is also a fan profile of its own (My PC → Fans), for whenever you want the PC as quiet as is safe.
- Fan control needs LibreHardwareMonitor (or a supported cooler), set up under My PC → Fans. Without it, quiet mode just turns the screens off.

Also in 0.28.0: Fortnite presets and checks, OBS on the deck, a Discord status, a network monitor, backups, controller mode, and screenshots searchable by their text.

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
