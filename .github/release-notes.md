## New in this version

**Potato graphics for Fortnite, and a redesigned OmniHub.**

- **Potato preset** (Games → Optimize → Fortnite): the blurry, low-poly look from the clips, for weak PCs. Performance mode, every quality setting at its lowest, near view distance, and a 3D resolution allowed below the menu's 25% minimum (down to 10%). Fortnite may raise a value it doesn't accept.
- **Lock the settings file:** makes GameUserSettings.ini read-only after Apply, so Fortnite can't put its own values back (what the potato guides do). While it's locked, Fortnite's menu can't change video settings; untick and Apply, or Restore, to unlock. A "File locked" badge shows when it is.
- **A new look:** top bar with icon section tabs and a wider search; a new side rail with icon tiles and a sliding accent marker; every page header now shows the page's icon, with a divider that appears as you scroll; deeper cards that lift on hover; solid primary buttons.


**Vesper 0.6.0: Right Shift menu, toggle sprint, 54 new functions and 59 new cosmetics and visuals.**

- **Right Shift** opens Vesper's mods menu in game and **Right Ctrl** the HUD editor.
- **Toggle sprint** (and optional toggle sneak).
- **39 HUD readouts** and **15 helpers**: fullbright, no fog, chat timestamps, chat cleanup, nick hider, auto GG, durability and hunger warnings, death coordinates, no pumpkin overlay, hide boss bar, more crit and sharpness particles.
- **46 cosmetics** in the new wardrobe (10 hats, 10 capes, 8 auras, 8 trails, 6 pets, 4 back items) and **13 visuals** (screen filters, vignette, ambient fireflies/leaves/snow/embers, item glow, kill streak banner, death marker, chunk borders, custom crosshair, cinema bars, kill flash).


**Vesper 0.5.0: a launcher-style Client menu, a POV camera, damage numbers and more motion.**

- **New Client menu** (in Minecraft): you, live in 3D with your cosmetics, beside the categories; every module as a tile with its switch; options slide in from the right. The panel opens and closes smoothly, tiles come in one after another, the category marker slides, tiles lift and press, and a short note confirms each switch.
- **POV & camera:** third-person distance, over-the-shoulder offset and height, a smooth F5 glide, F5 without the front view, and an optional freelook key (check each server's rules).
- **Damage numbers**, a **smooth hotbar** highlight, a **low-health pulse** and a **crosshair hit pulse**.
- **Wraith horns** now grow from the top of the head and curl up.


**Forge installs that keep going, and resource packs that install in one click inside Minecraft.**

- **Fix: "Couldn't load versions … peer closed connection"** on the Loaders tab. Forge's version list now retries (through Windows' proxy, then directly), then uses a mirror of it, and if that fails too, works the list out from Forge's own Maven. NeoForge, Fabric and Minecraft's version lists fall back the same way.

- **Forge doesn't give up:** when Forge's site is slow, down or blocked, OmniHub retries, then gets the same files from mirrors (BMCLAPI and Maven's second address), first through the proxy Windows is set to and then directly. Every file is still checked against Forge's own checksums, so a mirror can't change what's installed. Applies to OmniHub Client 1.8.9, Forge and NeoForge for any version, and Forge/NeoForge servers.
- If Forge's own installer stops halfway (a dropped connection), it runs again by itself and picks up where it stopped. Damaged library files left from an earlier failed install are fetched again.
- **Resource packs in one click (Vesper 0.4.2):** Minecraft's Resource Packs screen → *Get packs (Vesper)* → **Install**. The pack downloads, turns on at the top of your packs and loads straight away. Click again to turn it off.
- Packs added from OmniHub's Client tab are now turned on for the next game start too.
- Honest note: no download can be guaranteed when there is no internet at all, but it now takes every source being unreachable at once for an install to fail, and the message then lists what was tried. The mirror path was checked here (the mirrors serve byte-identical Forge 1.8.9 files); a full install with Forge's site blocked hasn't been run on a real Windows PC.

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
