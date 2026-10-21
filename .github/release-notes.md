## New in this version

**See the modpacks you already have** — Prism Launcher, CurseForge and the Modrinth App.

- **Mods → Minecraft → Installed on this PC** lists every modpack those launchers installed (also PolyMC and MultiMC): its picture, its Minecraft version and mod loader, how many mods it has (and how many are turned off), the modpack and version it came from, and when you last played it. Filter by launcher or search.
- **Mix** any of them with other packs: its mods and configs are copied into the mixer, and its version and loader come from the launcher, so it works even when a mod isn't on Modrinth.
- **Host it as a server** (or New server → *Installed modpack*): the server gets the same loader and version, the pack's mods straight from your PC (the ones that only run in the game itself are left out) and its configs.
- OmniHub only reads those folders; nothing in them is changed. Moved your instances or CurseForge folder to another drive? **Look in a folder** finds them there, and keeps looking there.

Also in 0.12.0: host Minecraft servers on your PC 24/7 (a new **Servers** tab, also on your phone), with players' locations and inventories, the whitelist, a live map, mods and plugins, and players without a paid account if you choose.

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
