# MASTER PROMPT — Claude Code: Build a Minecraft 1.8.9 PvP Client with Subagents

You are the **lead software architect, lead developer and integration owner** for a serious long-form Minecraft 1.8.9 PvP-client project. You must build a **real, installable Minecraft Forge 1.8.9 mod** with a polished custom in-game title/menu system that behaves like its own PvP launcher. Avoid a fake prototype, HTML mockup, stock GUI, or superficial screenshot recreation.

## Objective and reference

Closely study the two videos under `references/video/`, their curated stills under `references/frames/`, and `references/ANALYSIS.md`. They show **Larp Client** design and showcase: dark/monochrome entrance + menu, ornate white logo treatment, small vertical navigation, wavy and star decorations, player preview, vivid celestial skies, red/blue/purple glowing wings and comets, orbiting glyphs, purple rift, particles, cosmetic rings, top HUD, NPC target view, showcase models, cosmic black-hole/halo, flame, and an optional 3D vehicle prop. Make the style **as similar as possible in mood, animation quality, composition and visible features, while improving usability, customization and performance**.

**Use fresh original branding, original models and original audio/textures.** Do not rip the video, reproduce Larp's protected logo/brand, or assume every effect is real-time kill behavior. Design and name our own original client. Include an original simple geometric logo in the spirit of a sharp monoline sigil without copying the reference symbol.

Our product targets Minecraft **Java Edition 1.8.9**, **Forge 1.8.9**, **Java 8**, installation through a legitimate existing Minecraft launcher profile. It must work as a Java mod, not depend on an external executable. Keep separate desktop launcher support as a later optional milestone.

## Read these files first

- `CLAUDE.md`
- `references/ANALYSIS.md`
- `docs/FEATURE_MATRIX.md`
- `docs/ARCHITECTURE.md`
- `docs/TEST_PLAN.md`
- Both MP4 videos and the supplied frame images.

Before writing a complex mod module, create a tracked `TASKS.md` with a task inventory, dependencies, ownership, implementation status, and per-task acceptance checks. Record an architecture decision log and keep it updated.

## Mandatory specialized subagents

Discover and use the existing project-scoped `.claude/agents/` files. Delegate duties appropriately to:

1. `visual-director` — analyze reference frames and define high-fidelity art direction/UI/motion requirements. Must give a *reference timestamp* and clear recreation specs per feature.
2. `build-engineer` — verify Forge 1.8.9 MDK, Java 8, Gradle, launcher-profile installation, base mod lifecycle, artifact packaging.
3. `menu-ui` — implement full custom title screen, animated menu, settings shell, navigation, HUD editor visual hierarchy.
4. `vfx-engineer` — implement reusable particle/render/animation timeline, safe world-space VFX and minimum five true in-game kill effect presets.
5. `cosmetics-animator` — implement original 3D/2D model pipeline and celestial wings, comets, orbiting sigils, cosmetics, showcase stage.
6. `hud-pvp` — implement user-customizable HUD and *honest* client-side combat trigger pipeline. Do not spoof kill confirmations.
7. `performance-engineer` — benchmark/profile, quality presets, culling, state/resource cleanup, graceful low-fidelity fallbacks.
8. `compatibility-security` — audit game/mod integration, privacy, server compliance, conflict risks and no unfair PvP mechanics.
9. `qa-playtest` — build/install/in-game testing, acceptance matrix, screenshot comparisons, regression report.

Use actual Claude Code subagent delegation, not fake claims. Subagents get narrow, concrete tasks; avoid concurrent edits of shared bootstrap/config APIs. If spawning is unavailable, perform tasks sequentially and label the limitation. **As lead integrator, you are responsible for merging and verifying every change.** Read-only review subagents should not silently rewrite the implementation.

## Development plan and execution gates

### Gate 0 — environment, legal constraints, feasibility

Verify the up-to-date archived Forge 1.8.9 MDK (official Forge page lists 11.15.1.2318), Java 8/ForgeGradle compatibility and dependencies. Run a first minimal mod launch. Investigate when an old repository is broken; avoid downloading arbitrary precompiled clients. Document mod limitations, server rules and what a client-only effect can do. No Microsoft account/token handling is needed for the v1 in-game mod. Use the official launcher for authentication. Do not ship game binaries.

**Pass if:** the project builds, Minecraft 1.8.9 with Forge launches, `@Mod` is loaded, custom config loads, user can return to normal menus.

### Gate 1 — actual custom launcher-like title/menu

Create a custom first-boot/first-title entrance inspired by both clips: near-black backdrop, fine original monoline logo, centered/left-biased title, small `Press to Enter` text, graceful transitions. On enter, show Singleplayer / Multiplayer / Minecraft Settings / Client / Quit Game. Provide decorative curved layers and faint moving stars toward right side and optional live character preview. Add settings and a modular in-client config overlay. Make it function via proper Minecraft/Forge GUI classes, not as a detached simulation.

Add easing, input handling, focus support, scaling to 1080p and 4K, shader fallback, toggle for low motion. Screen transitions must not interfere with multiplayer connection or official account login.

**Pass if:** all menu items are functional from a real 1.8.9 session, title recreation is visibly close in composition, and no infinite GUI event recursion occurs.

### Gate 2 — settings/modules/HUD foundation

Implement a typed module registry with enable/disable, persistence, search, category tabs, per-module keys, sliders, color pickers, defaults/reset and profile switching. Implement draggable/scalable HUD elements. Use authentic Minecraft-rendered FPS, ping, armor/status/keystrokes etc, not placeholders. Include a small top status line like the video with original symbols and no fake domain/user labels.

**Pass if:** users can change settings, restart Minecraft, and get the same layout; all data shown is accurate or explicitly labeled demo.

### Gate 3 — render/animation framework + preview stage

Implement a reusable timeline with interpolation, easing, particles, gradient/textured trails, orbit transforms, animated symbols, ring meshes, optional GLSL-compatible backgrounds and fallbacks. Include a safe local self-contained character/NPC cosmetic preview stage so the art can be validated without a multiplayer match. Provide live frame budget settings. All GL states restored when drawing ends.

**Pass if:** visual preview runs in Minecraft in the actual mod, animated objects align to a player and reset on world unload/resource reload.

### Gate 4 — reproduce cosmic cosmetics shown in videos

Create original, selectable and configurable cosmetics:

- Celestial wings (red theme from A; vivid blue from B; warm red from B; purple/pink from B).
- Shooting-star/comet trails and wing tips with adjustable glow, length, spawn rates and motion.
- Orbiting sigils, circular symbols, floating icons and halo around the player; make original geometric icon art, not copyrighted logos.
- Starfield, dark aurora, crimson inferno-style sky, violet rift and colored nebula backdrops for menu/showroom; optional multiplayer-safe visual mode with appropriate culling.
- Floor rings, glints/sparkles, constellation particles, sparkling white fragments and violet wire-frame planes.
- A local-only optional novelty prop in the showroom (e.g., an original compact hover-cart with a cosmic ring/fire display), clearly a 3D cosmetic and not a drivable addition.

Build at least 3 preset palettes that a user can switch instantly (blue/teal, red/amber, violet/magenta). Tune animation to be sophisticated but do not use camera motion in active fights.

**Pass if:** screenshot captures of all three colorways visually align with the energy/motion/style of the uploaded frames.

### Gate 5 — genuine kill-animation system (without pretending all servers expose a kill event)

Develop 5 selectable, entirely original effects: **Astral Collapse, Violet Rift, Solar Bloom, Frost Constellation, Comet Ascension**. Each is an actual programmed in-game sequence with independent enable/disable, colors, particle intensity, sound option, performance caps, skip/reduced-motion and local test button.

Create a `CombatVisualEvent` pipeline with `HIT_CONFIRMED`, `ENTITY_DEATH`, `PLAYER_KILL_CONFIRMED`, and `DEMO`, each carrying a source and confidence. Use trustworthy local/singleplayer events and optional cooperating-server confirmations where available. Where supported, an opt-in server-specific message parser may label the source as *best effort* and must not fake a universal confirmed kill. Merely seeing an entity removed is not proof that *you* killed it. No automated attack, gameplay modification, illegal packet modification or anti-cheat evasion.

If a character performs a client-side fall/dissolve, use a temporary local VFX proxy at the location: **never alter other players' real entities, collision or server-authoritative health**. Other players will not see it unless they have compatible rendering/mod support.

**Pass if:** every preset works via local demo and legitimate events; incorrect triggers/unloads are guarded; combat mechanics stay unchanged.

### Gate 6 — advanced polish and PvP client quality

Add tasteful optional first-person cosmetic swing/hold animations (no hitbox changes), color customization, saved profiles, input overlays, GPU-friendly particles, resource pack changes, hover/focus sounds, customizable preview model, 3 quality presets, toggle-all-effects and diagnostics. Use configs for safe defaults on public servers. Avoid unapproved multiplayer information overlays or gameplay automation.

**Pass if:** client is comfortable for normal PvP, effects never block clicking/fighting, input latency/frame pacing does not degrade unexpectedly, and all animations can be disabled instantly.

### Gate 7 — test and release

Have `qa-playtest` verify behavior, compatibility, actual average FPS and frame-time impact, settings persistence, memory/resource leaks, shader fallback, launch/exit/return-to-menu, public-server-safe default, and all 5 effects using screenshot/short capture evidence. `compatibility-security` performs final audit. Create `docs/TEST_RESULTS.md` and `docs/KNOWN_LIMITATIONS.md` based on actual testing only.

Deliver a **real release jar**, sources, assets, user guide, installation instructions, test results and limitations. If some optional feature is not possible or not complete, keep the working build, disable the partial feature and list it honestly.

## Non-negotiable quality bar

- This must be **real Java mod code**, and compiled and installed, not a plan-only simulation, HTML/CSS demo, pseudocode or mockup.
- No single agent can call the job finished without a successful integration build.
- Prioritize **fidelity to the two reference videos** with original assets over throwing in unrelated generic client modules.
- Make the custom menu elegant: restrained shapes, animated curvature, carefully spaced navigation, accessible controls, and polished hover transitions; no cheap neon clutter.
- Provide visible, working animations for wings, orbitals, trails, kill effect presets and sky scenes.
- Verify actual behavior in-game. Never invent FPS results or claim universal server compatibility.
- Follow Minecraft legal distribution guidelines and each server's rules; be clear that client-only cosmetic visuals may be invisible to others.
- Make sensible decisions and implement iteratively. Do not ask me to review every tiny change.

## Immediate first response and task execution

1. Summarize the precise visual references you extracted from videos A/B, citing timestamps and identifying uncertain pieces.
2. Inspect workspace and verify the toolchain and official Forge 1.8.9 setup.
3. Produce `TASKS.md` and an executable vertical-slice plan with agents assigned and explicit file ownership.
4. Start implementing Gate 0 and Gate 1 immediately. Do not stop at ideation; build/test as each slice becomes available.
5. Continue in dependency order, using subagents for specialist tasks and running integration checks regularly.
6. Conclude each milestone with what actually works, exact test commands/results, screenshot paths and known limitations.

**Start now.**
