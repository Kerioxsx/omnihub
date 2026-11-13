## New in this version

**Vesper Client 0.2.0: funny kill effects, full skies, blur, FOV, sword animations, hit lines and live lyrics in your world.**

- **Four funny kill effects:** Bonk! (a cartoon mallet, dizzy stars), Party Popper (confetti, streamers, a rainbow "GG!"), Ghost Out (a little ghost waves goodbye and floats away) and KA-POW! (a comic-book burst and smoke). Plus the five from 0.42, or Random.
- **Full skies:** three new skies (Galaxy Core, Sunset, Abyss), daylight that follows the game's clock, a moon, the Milky Way, shooting stars, hidden clouds, and fireflies, snow sparkles or embers drifting around you. Brightness and speed are adjustable.
- **Smooth blur:** in-game menus blur the world behind them, fading in, with your own strength and tint. Optional motion blur for smooth trails when you turn (the HUD stays sharp).
- **More FOV:** your own FOV up to 150°, a steady FOV that doesn't jump when you sprint, and a smooth zoom key (C) with scroll.
- **Sword animations:** 8 swing styles (1.8, 1.7 classic, Smooth, Spin, Stab, Chop, Swirl, Flick), 4 block-hit styles (including the 1.7 swing-while-blocking), swing speed, item size and position, and no lowering when switching items.
- **Hit lines:** anime-style impact lines and a slash arc where you hit, gold on critical hits, and speed lines while you sprint.
- **Live lyrics in your world:** OmniHub now shares the playing song's synced lyrics with Vesper (on this PC only). They float in front of you, huge in the sky, above your head or on the HUD, with the sung part lighting up. Turn it on in Vesper → Visuals → World lyrics.
- Everything is original and can be adjusted in Vesper's Client menu. Tested here in Minecraft with software rendering; not yet on a real GPU, and the world lyrics were tested against a stand-in, not a full OmniHub.

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
