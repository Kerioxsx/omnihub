## New in this version

**Host Minecraft servers on your PC, 24/7** — a new **Servers** tab, and the same on your phone.

- **Make a server in one go**: vanilla, Paper (plugins), Fabric, Quilt, Forge or NeoForge, any Minecraft version they support. OmniHub downloads the server and the right Java for it and checks every file. Or make it **from a Modrinth modpack, a .mrpack file or one of your mixes**: only the mods a server needs are installed (client-only mods like shaders and minimaps are left out).
- **24/7**: servers start with OmniHub (even with the window closed), restart if they crash, keep the PC awake while they run, back up the world on a schedule, and stop cleanly (world saved) when the PC shuts down.
- **Players**: who's on and where (X, Y, Z and the world), their **inventory** (give anything, take a stack away), health and food, game mode, teleport (to a spot, spawn or another player), heal, feed, a private message, kick and ban. The **whitelist**, operators and bans, and the **player limit**.
- **Who can join**: paid Minecraft accounts only, or **everyone, also players without a paid account** (free launchers). Minecraft can't check names then, so OmniHub guards your operators: someone with an operator's name is only an operator when they join from a place (address) you allowed, and an attempt from somewhere new is kicked and waits for your OK, with a notification on the PC and the phone. Switching moves everyone's whitelist, operator and ban entries and their saved things to the new kind of account, so nobody loses anything.
- **Live map**: the world from above, drawn by OmniHub from the server's own files (every server type, nothing uploaded), with players moving on it. Follow a player, zoom, Overworld, Nether and the End.
- **Mods, plugins and data packs** from Modrinth (with what they need), from a file, on/off/remove. **Claude makes data packs and Paper plugins** for your server with your Anthropic key (plugins are compiled only when you allow it).
- **Friends over the internet**: the address to share, a one-click firewall rule, the router port opened for you (UPnP), and a plain warning when your internet provider shares one address between homes (then a port can't be opened).
- **Console** with commands, every setting of server.properties, backups you can restore, memory and Java.
- **From your phone** (More → Minecraft servers): start, stop and restart, players with their location and inventory (give, heal, teleport, kick…), the whitelist, the player limit, who can join, approve an operator's new place, the live map (pinch to zoom) and the console. It can be turned off in Phone → Permissions.

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
