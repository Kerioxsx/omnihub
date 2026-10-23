## New in this version

**Share your mixes with friends, and host the worlds you played alone.**

- **Share with friends**: on a mix, or on any installed modpack (the new details window), save one file with every mod, the configs, resource and shader packs, **and the cover picture**. Your own settings, keys and server list stay out. Turn on *Put my server in their server list* and it's there in their Multiplayer, ready to join. Mods on Modrinth are downloaded by your friend's launcher; the rest ride inside the file. Installed modpacks can also be saved as a CurseForge zip.
- **A friend's pack**: in Mods → Minecraft, open a pack someone sent you. It becomes a mix with **their picture**, ready to install in the Minecraft Launcher. The file also opens in the Modrinth App and Prism Launcher.
- **What's in a modpack**: the details window lists its mods (name and version from each mod itself, which ones only run in the game, which are turned off), **checks Modrinth for newer versions** (nothing is changed), and shows its single-player worlds.
- **Host a world you played alone**: pick a world (in the details window, or New server → Installed modpack → World). The server gets a copy, with its game mode and difficulty, and with your Minecraft name **your inventory, place and levels from single-player become yours on the server**. The original world isn't touched.
- The Minecraft Launcher's own folder (.minecraft) is listed too, with its worlds and mods.
- Fix: **NeoForge (and Forge) mixes** couldn't be saved as a .mrpack when none of their packs named a loader build ("OmniHub can't look up neoforge versions"). OmniHub now finds the newest NeoForge or Forge build for the chosen Minecraft version.
- Fix: mods added from an instance folder or an installed modpack could get the download of another file from the same Modrinth version (often the Forge jar of a mod that ships both), which failed its checksum when downloaded. Each mod now gets the download of its own jar.

Also in 0.12: your installed modpacks from Prism Launcher, CurseForge and the Modrinth App; Minecraft servers 24/7 on your PC (also from your phone).

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
