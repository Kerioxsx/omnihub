# Verification, performance and quality gates

No release is complete on the basis of code compilation alone.

## Build and setup

- [ ] Official Forge 1.8.9 MDK used; Java 8 verified; Gradle wrapper setup is reproducible and dependencies locked/documented.
- [ ] `gradlew clean build` passes; output jar exists and correct mod metadata is present.
- [ ] No bundled Minecraft client files, Larp brand assets, proprietary models, hardcoded tokens or external credential collection.
- [ ] Client starts with Forge, reaches custom menu, can open Singleplayer/Multiplayer, Settings and Quit, and returns to menu correctly.

## Functional tests

- [ ] All enabled menus have actual keyboard navigation and correct mouse hit areas on both windowed/fullscreen.
- [ ] Splash displays once and can be skipped, does not infinite-loop via GuiOpenEvent.
- [ ] Settings persists cleanly, handles corrupted JSON, reset, migrations, readonly file.
- [ ] Animated effects have fixed lifetimes, cleanup on disconnect/resource reload, and no resource leaks.
- [ ] Test NPC/showcase can trigger all kill FX via an explicit local `DEMO` action.
- [ ] All 5 initial presets function in-game, not just in Photoshop/HTML.
- [ ] Safe kill-attribution paths tested; `DISCONNECT`, entity unload and false death chat do not generate fake confirmed kills.
- [ ] Wings/trails orbit and align to player in first/third person without obscuring gameplay unnecessarily.
- [ ] HUD widgets work with real values; target HUD does not access disallowed hidden enemy data by default.
- [ ] Config profiles are reproducible and effects can be disabled instantly.

## Visual tests

Capture paired screenshot evidence from game at menu splash, menu, preview avatar, blue/red/purple wings, aurora/rift/solar skybox, each kill effect, HUD editing and per-profile settings. Compare composition (not copyrighted textures) with `references/frames/`.

## Performance tests

- 1080p, 1440p and 4K; windowed/fullscreen; weak/typical/high-end hardware when actually available.
- 60/120/240 Hz modes where supported; realistic combat stress with entities/players in active view.
- Baseline vs P1 cosmetic effects at LOW/MEDIUM/HIGH quality; publish avg FPS, 1% lows, p95 or p99 frame time, RAM and GPU use where measurable.
- Test many effects triggered in rapid succession for memory growth, hitching and saturation; cap simultaneous instances.
- Explicitly record hardware, driver, settings and methodology. **No promise of 1400 FPS based on the reference clip.**

## Multiplayer and safety

- [ ] No automated gameplay, combat/movement advantages, unauthorized packet rewriting or anti-cheat bypass.
- [ ] No private account tokens in logs or config.
- [ ] Features designed as client-only do not require other players to install the mod; other players are not promised visibility.
- [ ] Competitive safe mode disables problematic informational overlays, perspective modifications and distracting graphics.
- [ ] Test on private/local server before any public deployment. Public server policies are independently reviewed.

## Release artifact contents

- Runnable and source-reviewable `.jar`, checksums, install steps, one-click reset, uninstall steps.
- `docs/TEST_RESULTS.md` with actual commands, captured screenshots and environment info.
- `docs/KNOWN_LIMITATIONS.md` stating what is approximate, server-dependent, and not supported.
- Proper attributions/licenses for every distributed model, texture, font, sound and dependency.
