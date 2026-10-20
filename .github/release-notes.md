## New in this version

**Mods: a mod manager for every game in your library, a Minecraft modpack mixer, and Claude making mods for you.** Everything else stays as it was.

- **Every game, and how it's modded.** The new **Mods** page lists your Steam games (and Epic games, and any game folder you add) and works out how each one takes mods: Unity games use BepInEx or MelonLoader, Balatro uses Lovely and Steamodded, GTA V uses Script Hook V, Unreal games use .pak files and UE4SS, and so on.
- **Install with one click.** For the 300+ games on **Thunderstore** (Lethal Company, R.E.P.O., Balatro, Valheim, Risk of Rain 2, Content Warning, PEAK, Schedule I…), search and install mods right there. OmniHub installs what each mod needs and the game's mod loader with it, in the places the loader expects.
- **Or add a mod you downloaded.** A zip, a 7z, a folder or a single file, from Nexus Mods, GitHub or anywhere. OmniHub puts the files where that game's loader looks for them.
- **Switch mods off and on, update them, remove them.** OmniHub keeps a copy of every game file a mod replaces, so switching a mod off or removing it puts the game back as it was. *Switch all off* gives you the plain game in one click (for example to play GTA Online).
- **Steam Workshop** items each game has downloaded, with their names and pictures.
- **Balatro:** Lovely and Steamodded install with the first mod that needs them, and mods go into `%AppData%\Balatro\Mods`.
- **GTA V Enhanced:** OmniHub installs Script Hook V .NET for you. Script Hook V itself has to come from its author's page (his licence doesn't let others hand it out): add its zip and OmniHub puts it in place. Mods are for Story Mode with BattlEye off; the page explains how, and how to go back to GTA Online.
- **Minecraft: mix modpacks.** Add two or more packs (from Modrinth, a .mrpack file, or an instance you already play in Prism Launcher, CurseForge or Modrinth App), choose a Minecraft version and a loader, and OmniHub makes one pack. Every mod appears once, at the version made for that Minecraft. Mods are updated to fit, anything a mod needs is added, and mods that refuse to run together are sorted out (the first pack wins). Save the mix as a .mrpack, or install a Fabric or Quilt mix straight into the Minecraft Launcher as a new profile with its own folder.
- **Claude makes mods for you.** Add your own Anthropic API key, pick a game and describe the mod: *"a joker that gives +4 mult and doubles it on lucky cards"*, *"press F7 for slow motion"*. Claude reads the game's files, looks up how its mod loader works, writes the mod, and you press **Install**. Then tell it what happened in the game and it fixes things, and **Update** installs the new version. It makes Steamodded mods for Balatro, Script Hook V .NET scripts for GTA V, UE4SS Lua mods for Unreal games, BepInEx and MelonLoader plugins for Unity games (built with the free .NET SDK, only when you allow each build), and config or data changes for other games. It shows about what each mod has cost on your key.

**Safe by design:** Claude only writes inside the mod's own folder and only reads the game's files. It can't write programs or scripts that Windows runs, and its builds refuse anything except plain code. It makes single-player and offline mods, never cheats for online games. Online competitive games (Counter-Strike 2, VALORANT, Fortnite, Apex and others) get no mods at all, because changing their files can get accounts banned. Games with anti-cheat get a plain warning. Your Anthropic API key is kept encrypted for your Windows account, never shown again, and only ever sent to Anthropic. Downloads are checked against the checksums Modrinth and GitHub publish.

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
