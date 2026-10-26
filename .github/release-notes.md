## New in this version

**Roblox starts again, and no more duplicate games.**

- **Roblox starts again**: Roblox's recent security changes stop it from starting with ReShade in its folder ("Failed to load library: d3d11.dll", then "Roblox encountered an unexpected error"); Bloxshade is down for the same reason. OmniHub takes out the shaders 0.15.0 put in Roblox as soon as it starts, also from Bloxstrap's and Fishstrap's folders, and no longer installs them. Another tool's ReShade is left alone, but OmniHub tells you if one is still there.
- **No duplicate games**: games in Steam's main folder showed up twice under *On this PC* (Steam writes that folder two ways). Each game is listed once now.

Also in 0.15.0: a much lighter OmniHub (windows you can't see sleep), Fortnite's Max FPS setup, rendering mode and cache cleaner, Roblox opening from Play, free memory before the game, and every Roblox flag with on/off switches.

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
