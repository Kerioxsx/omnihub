## New in this version

**Vesper Client: our own PvP client for Minecraft 1.8.9.**

- **Download:** `Vesper-Client-1.8.9.jar` below. It needs Minecraft 1.8.9 with **Forge 1.8.9-11.15.1.2318**. Or let OmniHub install everything: Mods → Minecraft → OmniHub Client (Vesper is ticked by default).
- **Title screen and menus:** a "Press to enter" splash with Vesper's own sigil, then a dark menu with satin-like curtains, twinkling stars and your player in a 3D preview wearing your cosmetics. Keyboard and mouse.
- **Cosmetics** (only you see them): celestial wings modelled in Blender, a comet plume with a thorn crown, orbiting glyph tokens, a halo, a floor ring and glints. Three palettes switch everything at once: Tidal, Ember and Nebula.
- **Five original kill effects:** Astral Collapse, Violet Rift, Solar Bloom, Frost Constellation and Comet Ascension. They play on kills the game confirms (singleplayer/LAN), or best-effort from chat if you turn that on. Never on a target that just disappears.
- **Showroom:** a 3D stage with five skies, palettes, cosmetic toggles, kill-effect previews on a test figure, and a hover-cart prop.
- **HUD:** status bar, FPS, ping, CPS, keystrokes, armour, potions, coordinates, session time and target info (singleplayer only by default). Drag, resize and hide them in the HUD editor.
- **Performance:** skips far mobs and armour stands (players are always drawn), low FPS in menus and in the background, and quality presets for every effect.
- Not yet tested with the official launcher using the release jar, on real servers, or with OptiFine. The test results and known limitations are in the repository under `client/docs/`.

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
