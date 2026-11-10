# Architecture decision log

## ADR-1 — Build with Gradle 8 + essential-loom instead of ForgeGradle 2.1 (2026-11-10)

**Context.** The official Forge 1.8.9 MDK (11.15.1.2318) uses ForgeGradle 2.1, which needs Gradle 2.7–2.14 and old Maven repositories; it does not run on modern JDKs and is fragile today.
**Decision.** Use `gg.essential.loom` 1.15.52 (Essential's maintained fork of architectury-loom with legacy Forge support) on Gradle 8.14. It consumes the **official** Forge `1.8.9-11.15.1.2318-1.8.9` artifacts and MCP `stable_22` mappings, compiles with a Java 8 toolchain and remaps to SRG names exactly like ForgeGradle's reobf.
**Verified.** `gradle build` produced a jar whose classes are Java 8 (major 52) with SRG member names (`func_…`/`field_…`); the same setup already ships OmniHub's kill-effects mod.
**Consequences.** Gradle runs on JDK 21; the JDK 8 toolchain is fetched by the foojay resolver. `gradle runClient` works for development (offline dev account).

## ADR-2 — Forge events and GuiScreen only; no ASM/Mixin (for now)

Everything so far is doable with Forge events (`GuiOpenEvent`, `RenderWorldLastEvent`, `RenderPlayerEvent`, `RenderGameOverlayEvent`, `ClientTickEvent`, `LivingDeathEvent` in singleplayer). No core mods or bytecode transforms, which keeps compatibility with OptiFine and other mods high. Revisit only with a demonstrated need.

## ADR-3 — Kill attribution tiers

`CONFIRMED` only from the integrated server in singleplayer/LAN (the server-side `LivingDeathEvent` with the local player as the damage source). `BEST_EFFORT` from an opt-in, per-server chat pattern (off by default, labelled). Nothing else counts as a kill: a target that disappears, teleports, unloads or disconnects never triggers a kill effect. Hit effects use the client's own attack (`AttackEntityEvent`), labelled `HIT_CONFIRMED` only in the sense "you swung at it", not that the server accepted the hit.

## ADR-4 — Original branding

Name **Vesper Client**; an original monoline sigil (no hexagram or other religious symbol, nothing from the reference client). Orbit tokens use original geometric glyphs, not cryptocurrency or other brand logos. No textures, models or audio are taken from the reference videos; all visuals are procedural (GL lines, quads, particles) or original files in `assets/vesper/`. Sounds use Minecraft's own sound events (nothing redistributed).

## ADR-5 — Where it lives and how it's installed

Source in the OmniHub repository (`client/`). Installation through OmniHub's existing 1.8.9 Forge profile setup (official launcher, Microsoft sign-in); the reference media stay out of the repository.
