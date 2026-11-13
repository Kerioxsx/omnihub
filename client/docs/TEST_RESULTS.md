# Test results — Vesper Client 0.1.0

Everything below was run for real. Nothing was tested on a physical GPU or with the official Minecraft Launcher yet; see "Not tested".

## Environment

- Linux container, 4 vCPU (Intel Xeon), **no GPU**: Mesa 25.2.8 **llvmpipe** software OpenGL (4.5 compatibility profile) on an Xvfb virtual display at 1600×900.
- Minecraft 1.8.9, Forge 11.15.1.2318, LWJGL 2.9.4, Java 8 (Temurin 8u512) for the game; Gradle 8.14.3 on JDK 21 with gg.essential.loom 1.15.52 to build.
- Launched with `gradle runClient` (Forge development environment, offline dev account `VesperDev`). Input driven with `xdotool`, screenshots with ImageMagick `import`.

## Build

| Check | Command | Result |
|---|---|---|
| Compile + remap | `cd client && gradle build` | Pass. `build/libs/vesper-client-0.1.0.jar`, 261 KB, 149 entries |
| Bytecode level | `javap -v` on a class | major version 52 (Java 8) |
| Remapped names | `javap -c` on `Effects`/`KillFx` classes (kill-effects mod built the same way) | Minecraft members are SRG names (`func_…`, `field_…`) as Forge needs at run time |
| Models inside jar | `unzip -l` | `assets/vesper/models/{wing,crown,skiff}.obj` present |

## Gate 0 — launch

| Check | Result |
|---|---|
| Game starts with Forge and the mod | Pass. Log: `Vesper settings loaded from …/config/vesper/config.json`, `Vesper Client 0.1.0 ready`; F3 shows "Powered by Forge 11.15.1.2318, 4 mods loaded" |
| Custom title replaces vanilla's, no `GuiOpenEvent` loop | Pass (title shown once, no stack overflow or repeated opens in the log) |

## Gate 1 — title and menus

| Check | Result |
|---|---|
| Splash: sigil draw-on, "VESPER", pulsing "Press to enter" | Pass (screenshot `g1-splash`) |
| Any key or click enters; logo moves into the menu header | Pass (`g1-transition`, `g1-menu`) |
| Hover/focus marker follows the mouse | Pass (`g1-menu2`) |
| Multiplayer opens the real server list; Cancel returns to our title | Pass (`g1-multiplayer`, `g1-back-from-mp`) |
| Singleplayer opens world select; Esc returns to our title | Pass (`g1-singleplayer`, `g1-back-from-sp`) |
| Singleplayer → Create New World → world loads | Pass (world created through the menus with xdotool) |
| Player preview with the user's skin (default skin offline) and cosmetics | Pass (`g3-title-preview`) |

## Gate 2 — settings, modules, HUD

| Check | Result |
|---|---|
| Client screen: categories, every module, toggles, sliders, choices, keys | Pass (`g2-client2`, later build shows 16+ modules) |
| A change is written to `config/vesper/config.json` about 1 s later | Pass: toggling "Reduced motion" wrote `reducedMotion: true` |
| Settings come back after a restart | Pass: palette chosen in the showroom (Nebula) was still active after relaunching and joining a world |
| HUD status bar with real data (Singleplayer, FPS, clock) | Pass (`w-ingame`, `w-focus`) |
| Armour/held-item durability element | Pass (diamond sword 1561, `w-thirdperson`) |
| HUD editor (drag/scale/hide) | Built; **not yet exercised with simulated drags** |

## Gate 3/4 — showroom, skies, cosmetics

| Check | Result |
|---|---|
| Showroom opens from Client without a world | Pass (`g5-showroom`) |
| Sky domes Nocturne, Bloodmoon, Aurora Veil render | Pass (`g6-tidal`, `g6-ember`, `g6-nebula`) |
| Palettes switch wings, comet, crown, floor rings and sky instantly | Pass (Tidal / Ember / Nebula) |
| Wings (Blender mesh) attached to the back with glowing tips, flapping | Pass |
| Comet plume + thorn crown (Blender mesh), halo, floor ring, glints | Pass |
| Cosmetics on your player in the world, third person | Pass (`w-thirdperson`) |

## Gate 5 — kill effects

| Check | Result |
|---|---|
| All five play from the showroom test buttons | Pass (`fx1…fx5` a/b/c frames: Astral Collapse, Violet Rift, Solar Bloom, Frost Constellation, Comet Ascension) |
| A real singleplayer kill plays the selected effect | Pass: zombie killed with a sword → Astral Collapse at the body (`k1`) |
| A death that is not yours does **not** play a kill effect | Pass: `/kill @e[type=Zombie]` killed 10 zombies → only vanilla death smoke (`neg1`) |
| `/vesper` client command (demo, toggle, diag) | Pass (`[Vesper] Performance off.` etc. in chat) |

## Performance (software rendering only — not representative of a gaming PC)

- In-world FPS on llvmpipe at 1600×900: **11–19 FPS** with wings + halo + HUD on, at night on a platform. This machine has no GPU, so these numbers say nothing about a real PC; they only show the client runs.
- The background-FPS cap engaged (HUD read 15 FPS while the window was unfocused) and turned off with the module.

## Not tested (yet)

- The **official Minecraft Launcher** with the remapped release jar (all runs used the Forge dev environment). The kill-effects mod built the same way has the same risk.
- Real multiplayer servers (Hypixel etc.), the opt-in chat kill parser against live chat, OptiFine and other mods alongside, 4K screens, and real GPU frame times.
- HUD editor dragging, profile switching, key binds.

Screenshots referenced above are in `docs/screenshots/` locally (kept out of git).

## 0.2.0 (same environment)

| Check | Result |
|---|---|
| Build | Pass: `vesper-client-0.2.0.jar`, 320 KB |
| Bonk!, Party Popper, Ghost Out, KA-POW! from `/vesper demo` in a world | Pass (fx-bonk, fx-party_popper, fx-ghost_out, fx-cmp) |
| A real kill plays the selected effect (Bonk!) | Pass: a pig killed with a sword played Bonk! |
| The new effects from the showroom buttons, 2-column layout | Pass (v2-show-bonk, v2-panel) |
| Galaxy Core sky with Milky Way, shooting stars on, fireflies | Pass (sky1); daylight mode and the moon not screenshotted |
| Menu blur behind the pause menu | Pass (blur) |
| Zoom key: camera zooms, hand unchanged | Pass (zoomcmp) |
| Sword at rest identical to vanilla with the module on/off | Pass (swcmp) |
| Spin swing | Pass (swspin) |
| Hit lines: burst on the first frame, slash arc after | Pass (hits) |
| World lyrics in front, karaoke highlight, next line | Pass against a stand-in for OmniHub's endpoint (v2-world) |
| Not tested | Motion blur, FOV above 110, block-hit styles, HUD/sky/above lyric modes, daylight following the clock, with OmniHub itself, on a real GPU |

## 0.3.0 (same environment)

| Check | Result |
|---|---|
| Build | Pass: `vesper-client-0.3.0.jar`, 406 KB, with models, sounds and sounds.json |
| Rain with splashes, world glyphs (night) | Pass (v3-world) |
| Puddles with ripples (day) | Pass (v3-day, jumps) |
| Jump mark at the feet | Pass (jumps, first frame) |
| Wraith head: head hidden, flame, horns, third person | Pass (wraith) |
| Emote wheel | Pass (wheel) |
| Car flex (car drops in, underglow, arms crossed) and Flex | Pass (emotes) |
| Not tested | Hit and rain sounds (no audio device), speed blur, block outline, time changer, wave/dance/salute/cheer, landing shockwave, other jump shapes, real GPU |

## 0.4.0 (same environment)

| Check | Result |
|---|---|
| Pack browser lists Modrinth's 1.8.9 packs | Pass: 30 packs shown |
| Download a pack | Pass: `LowOnFire_1.8.9.zip` saved to resourcepacks after the SHA-1 check, button shows Added |
| Not tested | FPS boost on a real GPU (the settings switch and restore were not exercised in a run), the button on Minecraft's Resource Packs screen |

## 0.4.1 (same environment)

| Check | Result |
|---|---|
| New horns (curling "C" shape, bands, thorns, blue gradient) with the crystal flame, daylight | Pass (h2) |
| Rebuilt car (smooth profile, side intake, wing, black rims, callipers) in the car flex | Pass (c2) |
| Model previews rendered in Blender (Cycles) before going in game | Pass |
