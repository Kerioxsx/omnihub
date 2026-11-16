# Feature scope and acceptance map

Legend: P0 foundation, P1 core video fidelity, P2 improved client feel, P3 advanced/optional. Features must actually function and persist; a demo-only effect must be labeled as such.

| Priority | Module | Specification | Minimum definition of done |
|---|---|---|---|
| P0 | Forge integration | 1.8.9 Forge .jar, Java 8, official-profile installation, clean client lifecycle | Launch, title screen and multiplayer join without crash; release jar |
| P0 | Title replacement | `Press to Enter` splash, menu navigation, fade/slide transitions, original angular symbol | Keyboard/mouse accessible, no trapping title screen, no recursion |
| P0 | Settings | Versioned JSON settings, per-module enable/disable, defaults and reset | Changes survive restart and invalid configs recover |
| P0 | Preview world | Safe local cosmetic showroom/test harness with fake NPC and dummy signals | Effect can be seen without requiring PvP or a multiplayer server |
| P1 | Menu design | Black, charcoal, white, sparse typography; animated waves/stars; avatar right; settings + client tabs | Accurate layout inspired by references at 16:9 and 4K; scales |
| P1 | GUI editor | Search, tabs, sliders, hotkeys, color picker, favorites, presets, draggable HUD | User can toggle and customize modules in game |
| P1 | Kill FX engine | Client-only animation timeline, events, particle emitters, model animation, camera-safe VFX | ≥5 distinct selectable original effects with preview and real event path |
| P1 | Kill feedback | Accurate source tiers: local SP / supported server event / opt-in message parser; verify cause | Kill FX never falsely claims universal attribution |
| P1 | Cosmetics | Celestial wings, comet orbit, glowing sigils, red/blue/purple trails; visible on self | Proper transform, animation, clipping and resource cleanup |
| P1 | Backgrounds | Celestial/aurora, crimson cloud, violet storm, starfield, cosmic halo themes | Apply to menu/showroom; multiplayer-safe render switch |
| P1 | HUD | FPS, ping, coordinates optional, potion/armor status, CPS optional if allowed, hotbar, crosshair, keystrokes, input overlay, session timer | Correct values only and per-server disable when needed |
| P1 | Target HUD | Private preview NPC HUD; public-server variant only when server allows exposed info | Never expose hidden opponent data or imply all servers permit it |
| P1 | Hit feedback | Cosmetic strike spark, color, screen-safe feedback, audio cue and optional particles | Does not change attack/hit mechanics |
| P2 | Animation presets | Optional first-person item animation, blocking/swing visual only, third-person hand/idle, cosmetic pose | Animations do not alter hitbox, reach or server physics |
| P2 | Profiles | `Competitive`, `Cinematic`, `Low-end`, `Balanced` quality profiles | Switching updates settings and preserves user customization |
| P2 | Performance | Batching, caps, frustum/distance culling, render-state preservation, reduced-motion | Performance measured with overlays on/off and logged |
| P2 | Resource packs | Bundled original particle textures/models/sounds, optional user pack load under sandbox | Invalid assets fail safely; format documented |
| P2 | Showcase scene | Local third-person stage and avatar turntable; original cosmic vehicle prop optional | No fake vehicle gameplay / multiplayer injection |
| P2 | Accessibility | Scale, contrast, colorblind-friendly palette, reduced flashes/motion | Can use menus without effects and recover via reset |
| P2 | Diagnostics | In-client version/FPS/module times, crash-safe mode, debug visualizer | Troubleshooting output with no auth token leakage |
| P3 | Optional companion | Independent Windows launcher **later**, not needed for mod and never bundles MC proprietary files | Owns only mod installation/profile selection; legitimate auth |
| P3 | Server plugin | Documented optional plugin to confirm kill attribution for a cooperating server | Explicit opt-in and server-side verification; normal servers unaffected |
| P3 | Asset pipeline | Converter for original Blender/Blockbench animation assets into efficient pack files | Deterministic asset packaging and license metadata |

## Original kill animation candidates

Implement **all 5** in an initial complete preset library, using visual-only effects and a short duration limit. User can choose color, intensity, lifetime and audio independently.

1. **Astral Collapse:** orbiting points spiral inward; luminous ring contracts and dissolves into tiny stars.
2. **Violet Rift:** a violet geometric circle opens, luminous shards travel upward, purple haze fades.
3. **Solar Bloom:** warm gold/orange light burst with ember-like squares and a radial wave.
4. **Frost Constellation:** snow/crystal shapes form a ring, a cold twinkle, then a clean fade.
5. **Comet Ascension:** blue-white comets arc past the defeated model position, leaving motion trails.

Later presets: **Eclipse Halo** (pink/blue black-hole style), **Void Pixels** (non-graphic dissolve), **Phoenix Ember**, **Aurora Pulse**, **Celestial Crown**. Cinematic-style movement should never disrupt the player's actual camera during ranked gameplay unless opted into a safe local preview.

## Competitive compliance defaults

- All VFX client-side. `competitive_safe` profile defaults to simple effects and no potentially prohibited opponent HUD data.
- Optional health/distance in target HUD **disabled on public servers by default** unless server explicitly permits it.
- Do not implement autoclicker, aim assist, reach edits, knockback edits, auto-block, spoofed packets, automated gameplay or anti-cheat evasion.
- Explicit per-feature multiplayer caution; no claim of any guaranteed server approval.
