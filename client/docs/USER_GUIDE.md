# Vesper Client — user guide

## Requirements

| | |
|---|---|
| Minecraft | Java Edition 1.8.9 |
| Loader | Forge 1.8.9-11.15.1.2318 (not Fabric, Legacy Fabric or NeoForge) |
| Java | 8 (supplied by the Minecraft Launcher) |
| Required mods | none |
| Optional | PolyPatcher (FPS); OptiFine 1.8.9 HD U M5 untested |

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

## New in 0.2.0

- **Funny kill effects:** Bonk! (a cartoon mallet and dizzy stars), Party Popper (confetti and a rainbow "GG!"), Ghost Out (a little ghost waves and floats away) and KA-POW! (a comic burst). Pick one in Kill effects, or Random. Try them in the Showroom.
- **World sky:** three more skies (Galaxy Core, Sunset, Abyss). Options: daylight that follows the game's clock, a moon, the Milky Way, shooting stars (how many per minute), hidden clouds, fireflies / snow / embers around you, brightness and speed.
- **Menu blur:** in-game menus blur the world behind them, fading in; set the strength, fade and tint. **Motion blur** (off by default): smooth trails when you turn; the HUD stays sharp.
- **FOV & zoom:** your own FOV up to 150°, a steady FOV that doesn't jump when you sprint, and hold **C** to zoom (scroll to zoom more). The hand stays normal.
- **Sword animations:** swing styles (1.8, 1.7 classic, Smooth, Spin, Stab, Chop, Swirl, Flick), block-hit styles (1.7 swing while blocking, Push, Tilted), swing speed (look only), item size and position, and no lowering when switching items.
- **Hit lines:** impact lines and a slash arc where you hit, gold on critical hits, and optional speed lines at the screen's edges while sprinting.
- **World lyrics:** with OmniHub running on the same PC, the song you're playing shows its synced lyrics in the world: floating in front of you, huge in the sky, above your head or on the HUD. The sung part lights up; set colours, size, distance and a timing offset.

## New in 0.3.0

- **World colour:** General → World colour (neon) sets the colour of jump marks, world glyphs, the block outline and car underglow (or turn on Rainbow).
- **Jump marks:** every jump leaves a neon mark on the floor: a six-point star, rune circle, ring, triangle or diamond, with sparks; landing from high up sends a shockwave.
- **World glyphs:** neon symbols floating around you.
- **Rain & wet ground:** rain that stops at roofs and splashes, with sound, and puddles that mirror the sky and ripple. Always, only when it really rains, or off.
- **Hit sounds:** Pop, Ding, Thud, Click, Bass, Chime (Vesper's own) or Minecraft sounds, a different one on critical hits, volume and pitch.
- **Emotes:** press **B** for the wheel: Flex, Wave, Dance, Salute, Cheer and Car flex (a car drops in next to you with neon underglow; Supercar, Muscle car or Hypercar, in any colour). Moving or attacking stops an emote; the camera turns to face you.
- **Wraith head:** your head becomes a ghostly flame with spiked horns (colours and sizes adjustable).
- **More:** Speed blur while sprinting, a neon Block outline, a Time changer (your own time of day), and items in your hand can now be as small as 0.2×.

## New in 0.4.0

- **Resource packs inside the game:** Client → Resource packs, or the *Get packs (Vesper)* button on Minecraft's Resource Packs screen. Search Modrinth's 1.8.9 packs and press Download; the pack is checked against Modrinth's checksum, saved in your resourcepacks folder and listed right away (turn it on in Resource Packs).
- **FPS boost:** Performance → *FPS boost game settings* switches to fast settings (fast graphics, no clouds, minimal particles, no entity shadows, smooth lighting off, VBOs on, vsync off, render distance capped) and saves yours; turn it off and yours come back exactly.

## New in 0.4.2

- **Install resource packs in one click:** on Minecraft's Resource Packs screen press *Get packs (Vesper)* (or Client → Resource packs), search, and press **Install**. The pack is downloaded, checked against Modrinth's checksum, turned on at the top of your packs and loaded straight away; it stays on next time. Click it again (*Turn off*) to switch it off. Downloads retry by themselves if the connection drops.

## New in 0.4.1

- **Wraith head, redesigned:** the horns now curl round in a tall "C" either side of the flame, with dark bands at the base, thorns along the outside and a blue fade to the tips. New **Flame style**: soft smoke or crystal shards.
- **Cars, redesigned:** smoother bodies with side air intakes, a rear wing on the Supercar and Hypercar, black five-spoke rims and coloured brake callipers. The default is glossy black with a red underglow; the underglow colour (or your world colour) and calliper colour are adjustable.

## Performance

- **General → Quality:** Low-end, Competitive, Balanced or Cinematic sets how many particles and how much detail Vesper's own effects use.
- **Performance** module: skips drawing far-away mobs and armour stands (players are always drawn), and lowers the frame rate in menus and when the game is in the background.
- For the biggest FPS gains in 1.8.9 also use PolyPatcher (OmniHub installs it with the client).

## Uninstall

Delete the jar from `mods` (or uninstall the OmniHub Client from OmniHub). Settings are in `.minecraft/config/vesper/` (or the profile's own folder).
