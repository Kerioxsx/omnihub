## New in this version

**A Minecraft hub: every launcher and client, and Forge, NeoForge, Fabric, Quilt or Legacy Fabric for any version, with mods.**

- **Launchers & clients** (Play → Client): the Minecraft Launcher, Prism Launcher, Modrinth App, CurseForge, ATLauncher, MultiMC and GDLauncher; Lunar Client, Badlion, LabyMod, Feather and Vesper. One click installs it from the publisher (Windows' winget), or opens the official download page, which also works for a Mac or Linux PC.
- **Loaders for any version:** pick Fabric, Forge, NeoForge, Quilt, Legacy Fabric or plain Minecraft and a version; OmniHub adds it as a profile in the official Minecraft Launcher with its own folder, so different versions' mods never mix. Forge and NeoForge use their own installer, with Java fetched when needed.
- **Mods for that version:** search Modrinth and add mods in a click; what a mod needs comes with it.
- Also in this release (from 0.47 and 0.48): the Client tab for Vesper, resource packs inside Minecraft, the FPS boost, redesigned Wraith horns and cars, and the MIT licence.
- Not tested on a real Windows PC yet: the winget installs and the loader installs were built and checked here, but not run end to end against a real Minecraft Launcher.

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
