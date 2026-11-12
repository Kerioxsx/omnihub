# Vesper Client — task inventory

Status: `todo` · `doing` · `done` (built **and** checked in a running game) · `partial` (works, with stated gaps) · `blocked`.
Owner = the agent role responsible (the lead integrator merges and verifies everything).
Evidence lives in `docs/TEST_RESULTS.md` and `docs/screenshots/`.

| ID | Gate | Task | Owner | Depends on | Status | Acceptance check |
|---|---|---|---|---|---|---|
| T0.1 | 0 | Toolchain: Forge 1.8.9-11.15.1.2318, Java 8 bytecode, Gradle 8.14 + essential-loom (ADR-1) | build-engineer | — | done | `gradle build` makes `build/libs/vesper-client-<v>.jar`, classes remapped to SRG names, major version 52 |
| T0.2 | 0 | Mod bootstrap (`@Mod`), log line on load | build-engineer | T0.1 | done | Forge log shows "Vesper Client … ready" in `gradle runClient` |
| T0.3 | 0 | Versioned JSON settings, corrupt-file recovery, atomic save | lead | T0.2 | done | Broken file is set aside, defaults load, game starts |
| T0.4 | 0 | Headless launch harness (Xvfb + Mesa software GL) with screenshots | qa-playtest | T0.2 | done | Title screen screenshot captured from the running game |
| T1.1 | 1 | Splash "Press to enter" with original sigil, entrance animation | menu-ui | T0.2, VISUAL_SPEC | done | Shown once per launch, any key/click enters, Esc skips, never loops |
| T1.2 | 1 | Title menu: Singleplayer / Multiplayer / Minecraft Settings / Client / Quit | menu-ui | T1.1 | done | Each opens the real screen and Back returns to our title |
| T1.3 | 1 | Curtain arcs + drifting stars, player preview, hover/focus motion, keyboard nav, reduced motion | menu-ui | T1.2 | partial | Arrow keys/Enter work; reduced motion stops movement; 1080p/4K scale |
| T1.4 | 1 | Original monoline sigil logo (GL lines, no hexagram/religious symbol) | visual-director → menu-ui | — | done | Drawn crisp at any scale |
| T2.1 | 2 | Module registry + typed settings (bool/slider/colour/choice/key) | lead | T0.3 | done | Settings persist across restart |
| T2.2 | 2 | Client screen: category tabs, search, toggles, sliders, colour picker, key binds, reset, profiles | menu-ui | T2.1 | done | Change → restart → same values |
| T2.3 | 2 | HUD elements (real data): status bar, FPS, ping, CPS, keystrokes, armour, potions, coords, clock | hud-pvp | T2.1 | partial | Values match F3 / tab list |
| T2.4 | 2 | HUD editor: drag, scale, hide, snap, reset | menu-ui + hud-pvp | T2.3 | partial | Layout survives restart and resolution change |
| T3.1 | 3 | GL state guard, easing, timeline, particle pool with budget | vfx-engineer | T0.2 | done | No GL state leaks (sky/hand/GUI unchanged after effects) |
| T3.2 | 3 | Showroom (local preview stage) with avatar turntable + test NPC | cosmetics-animator | T3.1 | done | Opens from Client menu without a world or server |
| T3.3 | 3 | Procedural skies: starfield, aurora, crimson storm, inferno, violet rift | vfx-engineer | T3.1 | done | Menu/showroom backdrops; world sky only when opted in |
| T4.1 | 4 | Celestial wings (4 colourways), flap animation | cosmetics-animator | T3.1 | done | Aligned to body in 1st/3rd person, own player only |
| T4.2 | 4 | Comet plume + thorn crown, orbiting glyph tokens, halo, floor ring | cosmetics-animator | T3.1 | done | Follows player, culled when far / in first person |
| T4.3 | 4 | Three palettes (Tidal blue/teal, Ember red/amber, Nebula violet/magenta) | cosmetics-animator | T4.1 | done | Instant switch; screenshots per palette |
| T4.4 | 4 | Showroom-only hover-cart prop with accretion ring + flame | cosmetics-animator | T3.2 | done | Only in the showroom; not an entity |
| T5.1 | 5 | CombatVisualEvent pipeline (HIT_CONFIRMED, ENTITY_DEATH, PLAYER_KILL_CONFIRMED, DEMO; source + confidence) | hud-pvp | T0.2 | done | Unit-level checks: unload/teleport/disconnect never produce a kill |
| T5.2 | 5 | Five kill effects: Astral Collapse, Violet Rift, Solar Bloom, Frost Constellation, Comet Ascension | vfx-engineer | T3.1, T5.1 | done | Each plays from the showroom test button and from a singleplayer kill |
| T5.3 | 5 | Opt-in server kill-message parser (best effort, labelled) | hud-pvp | T5.1 | partial | Off by default; false-positive tests |
| T6.1 | 6 | Quality presets (Low-end, Competitive, Balanced, Cinematic), toggle-all-effects key, diagnostics overlay | performance-engineer | T3–T5 | partial | Effects off instantly; diagnostics show frame time |
| T6.2 | 6 | Hover/focus sounds (vanilla sound events only), accessibility (scale, reduced flashes) | menu-ui | T1 | partial | — |
| T7.1 | 7 | Measured performance (software GL here; real GPU numbers need the user's PC) | performance-engineer + qa | all | partial | Numbers recorded with method and hardware |
| T7.2 | 7 | Compatibility / security audit | compatibility-security | all | todo | `docs/AUDIT.md` |
| T7.3 | 7 | TEST_RESULTS, KNOWN_LIMITATIONS, USER_GUIDE, install via OmniHub | qa-playtest + lead | all | partial | Docs based on real runs only |

## Notes on partial items (0.1.0)

- T1.3: keyboard navigation, reduced motion and scaling work; 4K not tested.
- T2.3/T2.4: HUD elements and editor are built; the editor's drag/scale wasn't driven in automated tests yet.
- T5.3: the chat parser exists and is off by default; not tested against a live server.
- T6.1: quality presets, the all-effects key and the Performance module exist; no diagnostics overlay beyond `/vesper diag`.
- T6.2: menu sounds use vanilla events; no extra accessibility options beyond reduced motion/flashes and text size.
- T7.1: only software-rendering numbers so far (see TEST_RESULTS).
- T7.2: the compatibility/security review agent couldn't run (usage limit); the lead's own review is in KNOWN_LIMITATIONS.
