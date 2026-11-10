# Technical architecture and implementation guidance

## Minimum supported stack

- Game: Minecraft Java Edition **1.8.9**.
- Forge MDK: official current archived Forge 1.8.9 page identifies **11.15.1.2318** for latest/recommended (check live before starting).
- Build/JDK: Java **8** and compatible legacy Gradle/ForgeGradle; pin versions reproducibly. Legacy Gradle repositories may be unavailable and require repository fixes; log decisions and avoid untrusted mirror scripts.
- Runtime: LWJGL **2** (Minecraft 1.8.9), legacy OpenGL pipeline. Verify shader compatibility with this era of Minecraft instead of assuming modern GL3+ APIs. Avoid OptiFine internal API coupling.
- Default installation: normal user-owned Minecraft launcher + 1.8.9 Forge profile; mod `.jar` in correct `mods` folder.

## Component boundaries

```
project/
  build.gradle / gradle.properties / gradlew ...
  src/main/java/<package>/
    ClientMod.java                 // entry, lifecycle, registration
    event/                        // Forge event adapters, main-thread dispatch
    gui/TitleScreen.java          // splash/title/menu
    gui/ClientConfigScreen.java   // browsable settings with components
    gui/HudEditorScreen.java
    module/                       // Module interface, registry, categories, flags
    setting/                      // config serialization, migrations, profiles
    render/                       // OpenGL state guards, batching, fonts, shaders
    fx/                           // time-based animation/particles/effect definitions
    combat/                       // signals & attribution, no gameplay changes
    cosmetic/                     // original self-only models, transforms
    hud/                          // draggable HUD widgets, telemetry
    preview/                      // local showcase and scripted safe events
    diagnostics/                  // logging, performance overlay
  src/main/resources/
    assets/<modid>/textures/...   // original effect and gui assets
    assets/<modid>/models/...     // original assets, rigs
    assets/<modid>/shaders/...    // GL2-compatible shaders + fallback
    assets/<modid>/sounds/...     // original or properly licensed audio
  docs/ ...
```

## Render pipeline

- Render with interpolation `partialTicks`; all animations based on monotonic elapsed time, not on fixed frame assumptions.
- Use Minecraft texture manager/resource locations; pre-create GL resources and rebuild on resource reload.
- Save and restore GL blend, depth, lighting, alpha, culling, line width, shader/program, active texture, projection/model view states (where required). Rendering should not corrupt sky, hitboxes, world GUI or OptiFine.
- 2D: reusable UI components with hover/focus, crisp text, scissor clipping, DPI-aware scaling, alpha transitions.
- 3D: a bounded scene graph and a particle pool with budget by quality level; world-space/camera-facing particle options, depth-tested by default, optional additive glow.
- Skybox modes: safer in custom menu/showroom; in live multiplayer never occlude players/world geometry or provide hidden visibility. No world-light hacks.
- Shader capability detection: GLSL 1.20-compatible path if available + non-shader fallback. Bloom is optional, not baseline.

## Combat effect event contract

```
VisualEvent {
  type: HIT_CONFIRMED | ENTITY_DEATH | PLAYER_KILL_CONFIRMED | DEMO,
  actorUuid?, targetUuid?, worldPosition, timestamp,
  confidence: CONFIRMED | BEST_EFFORT | DEMO,
  source: SINGLEPLAYER | COOPERATING_SERVER | SERVER_MESSAGE | MANUAL_PREVIEW
}
```

- **Do not equate a client's local observation of a target disappearing with a confirmed kill.** An entity can leave range, teleport, unload or despawn.
- Singleplayer/local-test (or a cooperating server) supplies the trustworthy path. A server-specific parser of kill messages/scoreboards must be opt-in, locale-aware, limited, and labeled best effort with false-positive tests; do not intercept or modify outgoing combat packets.
- On unknown public servers, default to harmless hit animations and demo kill preview; only play confirmed-kill effects when clear evidence exists.
- Debounce duplicate notifications and clear pending callbacks on world unload/server switch.
- Apply world-position effects locally without changing server entity states. Enemy models are not actually mutated. Any ragdoll/dissolve must use a temporary *local visual proxy* rather than modifying a remote player's real hitbox or network behavior.

## Config example (design guide, not fake implemented output)

```json
{
  "schemaVersion": 1,
  "profile": "Balanced",
  "modules": {
    "customTitle": true,
    "astralWings": true,
    "killEffects": true,
    "targetHudPublic": false,
    "particles": true
  },
  "effects": {
    "selectedKillEffect": "astral_collapse",
    "quality": "MEDIUM",
    "particlesMax": 250,
    "intensity": 0.65,
    "reducedFlashes": true
  }
}
```

Keep user settings in a private, local config folder. Do not send telemetry or store credentials.

## Agent file ownership

- `build-engineer`: build, environment, primary mod bootstrap (with lead integrator).
- `menu-ui`: title GUI, menus, layout and motion.
- `vfx-engineer`: rendering/effects framework and kill presets.
- `cosmetics-animator`: wings, sigils, models, transforms and showroom assets.
- `hud-pvp`: HUD widgets, combat signal detection and safe gating.
- `performance-engineer`: profiling, optimization, crash resilience and quality modes.
- `compatibility-security`: Minecraft policy review, mod conflicts, permissions, forbidden mechanics checks (review-only by default).
- `qa-playtest`: test matrix, regression tests, human-verifiable screenshot evidence (review/test-only).
- `visual-director`: frame-by-frame references, image comparison, art direction and acceptance review (review-only).

## Integration order

After architecture decisions: build bootstrap -> settings/registry -> blank replacement menu -> animation and rendering primitives -> video-inspired main menu -> preview stage -> cosmetics -> kill effects -> HUD/combat signals -> profiles -> final optimization/testing. Have the lead integrate each vertical slice on the main branch; only independent files should be edited concurrently.
