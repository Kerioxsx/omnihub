## New in this version

**Fortnite: a Grass switch, for the clean potato look.**

- New **Grass** choice (Games → Optimize → Fortnite): Off removes the grass on the ground (Fortnite's own Show Grass setting), On shows it, Auto turns it off with lowest quality. OmniHub now writes it even when Fortnite's settings file has no line for it yet.
- The **Potato** and **Clean & bright** presets turn grass off, Pretty turns it on, and Balanced leaves it on Auto.
- For the flat, untextured walls and floors from the clips, combine the Potato preset with **Potato textures (NVIDIA)** at Strong or Extreme. On AMD or Intel there is no safe equivalent: the driver setting is NVIDIA's, and removing game files gets you kicked by Fortnite's anti-cheat.

**Kicked out of Fortnite matches? OmniHub now tells you why, and texture pack removal is gone.**

- **Kicks explained**: Fortnite writes the reason whenever it removes you from a match (connection timeout, anti-cheat, mismatched game files, being idle). OmniHub reads its newest logs and shows the last one in Games → Optimize → Fortnite, next to crashes, with what to try.
- **"Remove & play" is removed.** Fortnite's anti-cheat checks the game's files during a match, and playing with the optional texture packs taken out can get you removed from it. Any packs still out go back as soon as Fortnite is closed (or with **Put back now**). For flat textures use Potato textures (NVIDIA) or the Potato preset. On AMD or Intel, the safe way is to uninstall High Resolution Textures in the Epic Games Launcher (Fortnite → Options).

**Fortnite crash help, potato levels that each look different, and capped presets.**

- **Last crash, explained**: if Fortnite crashed in the past week, Games → Optimize → Fortnite now shows when, Fortnite's own error message (from its crash report) and the likely cause: graphics driver stopped responding, out of graphics or system memory, unreadable game files, or a crash while the texture packs were out. Each comes with what to try.
- **Presets no longer run uncapped**: Potato, Clean & bright and Competitive used to set the frame rate to Unlimited. At these settings that can mean several hundred FPS, a PC running flat out and a crash after a while. They now cap just above your monitor's refresh rate. Unlimited is still there under Frame rate limit.
- **Potato textures (NVIDIA)** now has five steps that each look different: Off, Light (+1), Medium (+3), Strong (+5) and Extreme (+8). The old Max (+15) looked the same as +8, because every texture is already at its smallest size by then, so it's gone.
- NVIDIA's driver doesn't apply this setting to **DirectX 12**, which is why it seemed to do nothing with the Balanced or Pretty presets. The row now says so when Fortnite is on DirectX 12. It works in Performance mode (the Potato preset) and DirectX 11.
- **Play without texture packs** is marked experimental. Parts of the game may still ask for the packs and crash. If Fortnite crashes while they're out, the crash notice says so.

**Fortnite without its texture packs, on any graphics card, and every file goes back.**

- The other half of the potato apps' trick: on AMD and Intel they "remove texture packs". OmniHub now does it the careful way: Games → Optimize → Fortnite → **Play without texture packs → Remove & play**. Fortnite's *optional* packs (the files in `FortniteGame/Content/Paks` with "optional" in their name: High Resolution Textures and the like) are **moved, never deleted**, into a folder beside the game on the same drive, and Fortnite starts.
- They **go back by themselves as soon as Fortnite closes**. If OmniHub or the PC closes mid-game, they go back the next time OmniHub starts. If Fortnite never starts, they go back after 20 minutes. **Put back now** returns them at any time (with Fortnite closed).
- Nothing moves while Fortnite runs, a half-finished move is undone, and a pack the Epic launcher downloaded again in the meantime is kept. Don't run Verify in the Epic launcher while the packs are out: it would download them again.
- If Fortnite has no optional packs installed, OmniHub says so: there is nothing to remove.

**Four more Windows tweaks (Games → Optimize → This PC), each with Undo:**

- **Background apps off**: Store apps stop running and refreshing in the background.
- **Transparency effects off**: no see-through blur in the taskbar, Start and windows.
- **Power throttling off** (administrator, restart): Windows stops slowing programs it thinks are in the background.
- **Windows telemetry service off** (administrator): the DiagTrack service is stopped and disabled; Undo puts back how it started before.
- EMTweaks' TCP/Nagle "ping" tweaks were left out on purpose: Fortnite's gameplay traffic is UDP, so they don't change in-game ping.

**Potato textures for Fortnite, the way the potato apps do it.**

- The flat, untextured "potato" look from the clips (and from apps like EMTweaks, whose Potato button works "through the driver") is an NVIDIA driver setting: **Texture filtering – LOD Bias** on Fortnite's profile, with driver-controlled LOD bias off. OmniHub now sets it itself, with no extra tools: Games → Optimize → Fortnite → **Potato textures (NVIDIA)**: Off, Light (+2), Strong (+8) or Max (+15). Windows asks for administrator rights; restart Fortnite to see it. Off gives both settings back to the driver.
- Nothing in Fortnite's files changes: it is the same driver profile the NVIDIA Control Panel writes. On AMD or Intel there is no such driver setting; the closest is removing Fortnite's High Resolution Textures in the Epic Games Launcher (Fortnite → Options).
- Combine it with the Clean & bright or Potato preset for the full look.


**Fortnite "Clean & bright": the sharp, flat pastel look from the clips.**

- New **Clean & bright** preset (Games → Optimize → Fortnite): Performance mode, everything at its lowest, near view distance, a sharp 100% 3D resolution and Brightness up to 140%.
- New **Brightness** choice (Keep, 100–160%), saved as Fortnite's DisplayGamma (2.2 = 100%). OmniHub only changes it when the file already has that line. How Fortnite's slider maps to the value isn't documented anywhere we could find, so if it looks off, move Fortnite's own Brightness slider.
- For even more colour, like many of the clips: NVIDIA Control Panel → Adjust desktop colour settings → Digital vibrance, or AMD Software → Display → Saturation. That's a graphics-driver setting OmniHub doesn't change.


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
