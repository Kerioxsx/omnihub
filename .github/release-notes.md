## New in this version

**Your monitors' brightness from your phone.**

- **On the phone**: More → **Volume & brightness** now starts with *Monitor brightness*: a slider for every PC monitor, contrast where the monitor has it, and one for all of them. It works with the phone's music control switch (OmniHub → Settings → Music).
- **On the PC**: My PC → **Monitors** has the same sliders: a brightness slider for every monitor, contrast where the monitor has it, and one slider for all of them. OmniHub changes each monitor's own setting over DDC/CI, as its buttons do (the same way Twinkle Tray works, so both run fine side by side); a laptop's built-in screen goes through Windows. A monitor that doesn't answer gets a hint: turn on DDC/CI in its own menu.

Also in 0.15.1: Roblox starts again (OmniHub takes out the ReShade Roblox now blocks) and each game is listed once.

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
