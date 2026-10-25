## New in this version

**A much lighter OmniHub, more Fortnite frames, Roblox shaders, every Roblox flag, and Govee floor lamps in full colour.**

- **Lighter on your PC**: a window you can't see — in the tray, minimised, or behind a full-screen game or video — stops drawing, drops to WebView2's low memory level and is suspended like a sleeping browser tab, so it uses next to no processor and gives memory back while you play. It catches up the moment you look again. Pages load only when you open them (the glow and case-screen windows no longer load the whole app), fast streams only go to windows that show them, and "is it running?" checks take a quick name lookup instead of a full process sample.
- **Fortnite Max FPS setup**: one click sets no frame cap, Performance mode, the lowest quality everywhere, near view distance, fullscreen, NVIDIA Reflex on + boost and the FPS counter. New **Rendering mode** choice: Performance, DirectX 12, DirectX 11 or keep yours.
- **Roblox opens from OmniHub again**: Play now opens the Roblox app (through Roblox's own link, so it updates first) instead of starting the player with nothing to do. If a game is slow to open, OmniHub says what to check (the Epic Games Launcher may be updating Fortnite or asking you to sign in) and **Start again** launches it again without ending the boost.
- **Free memory before the game**: just before the game starts, background programs hand the memory they aren't using back to Windows, so the game starts with the most free RAM (on in both boost modes).
- **Clean Fortnite caches**: empties Fortnite's logs, crash reports and web cache and the graphics driver's shader caches (NVIDIA, AMD, Intel, DirectX), and says how much it freed. Only while Fortnite is closed; the first match after rebuilds shaders.
- **Roblox shaders**: in a Roblox game profile, turn on *Shaders* and pick a look: **Realistic**, **Vivid**, **Cinematic**, **Soft**, **Sharp** (lightest on frame rate) or **Custom**, with a strength slider. OmniHub downloads the official ReShade (the build Roblox allows) and effects from SweetFX and qUINT, and puts them back after each Roblox update when you start Roblox from OmniHub. In the game, **Home** opens ReShade's menu and **End** turns the effects off and on.
- **Every Roblox flag**: *All flags* lists every flag Roblox's client has, with the value Roblox uses now, searchable, the ones Roblox still reads marked. Add any flag, edit its value, and switch each one **on or off** without deleting it.
- **Govee floor lamps on Auto**: lights that ignore streaming but have segments (like the H6076, known as 14 segments) get a colour per segment by themselves.

How many frames you get depends on your processor and graphics card: OmniHub removes every limit it can and gets everything else out of the way.

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
