# Vesper Client — user guide

## Install

**With OmniHub (easiest):** Mods → Minecraft → OmniHub Client. Leave "Vesper Client" ticked and press Install. It adds an "OmniHub Client 1.8.9" profile to the official Minecraft Launcher with Forge, Vesper and the curated PvP mods. Start Minecraft, pick that profile, press Play.

**By hand:**
1. Install Minecraft: Java Edition and run 1.8.9 once.
2. Install Forge **1.8.9-11.15.1.2318** (files.minecraftforge.net → 1.8.9 → Installer → Install client).
3. Download `Vesper-Client-1.8.9.jar` from the [latest OmniHub release](https://github.com/Kerioxsx/omnihub/releases/latest) and put it in `.minecraft/mods`.
4. Start the "forge" profile.

## Using it

- **Title screen:** press any key at "Press to enter". Arrow keys/Tab + Enter work everywhere in the menu.
- **Client** (on the title screen): every module with its options. Search with Ctrl+F. Changes save by themselves.
  - **Profiles** (bottom left): click the name to switch, **New** to copy the current one, **Reset** for defaults.
  - **Edit HUD layout:** drag elements, scroll to resize, right-click to hide, Shift to place freely.
  - **Showroom:** see your cosmetics on a 3D stage, switch palettes and skies, and preview the five kill effects on a test figure.
- **Keys:** set a toggle key for any module in its options, and an "All effects on/off" key in General.
- **`/vesper` command** (only on your client): `/vesper demo [effect]`, `/vesper effects on|off`, `/vesper toggle <module>`, `/vesper profile <name>`, `/vesper diag`.

## Performance

- **General → Quality:** Low-end, Competitive, Balanced or Cinematic sets how many particles and how much detail Vesper's own effects use.
- **Performance** module: skips drawing far-away mobs and armour stands (players are always drawn), and lowers the frame rate in menus and when the game is in the background.
- For the biggest FPS gains in 1.8.9 also use PolyPatcher (OmniHub installs it with the client).

## Uninstall

Delete the jar from `mods` (or uninstall the OmniHub Client from OmniHub). Settings are in `.minecraft/config/vesper/` (or the profile's own folder).
