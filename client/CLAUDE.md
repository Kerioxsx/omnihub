# Project instructions for Claude Code

Goal: implement a **real, buildable, visually polished Minecraft 1.8.9 Forge PvP client mod**, not an HTML mockup, screenshot, deceptive placeholder, standalone web app or proprietary game client redistribution.

Start by reading `MASTER_PROMPT.md`, `references/ANALYSIS.md`, `docs/FEATURE_MATRIX.md`, `docs/ARCHITECTURE.md` and `docs/TEST_PLAN.md`. Review video/frame references yourself. Track implementation in `TASKS.md` and preserve a clear difference between working, partial and planned features.

## Engineering rules

1. **Working game mod first**: ensure Java 8, Minecraft 1.8.9 and verified supported Forge MDK build, tests and a reproducible Gradle build before complex effects.
2. Prefer Forge events and `GuiScreen` APIs; use ASM/Mixin only for a demonstrated need, after compatibility analysis and a fallback is considered. Document all transforms.
3. Keep all PvP effects **visual only** and client-side, with no aim assistance, combat automation, modified reach, attack rate, movement, packets, hit registration, knockback or server-state interference.
4. A PvP kill cannot be inferred accurately on arbitrary servers solely from client rendering. Use a tiered confirmation strategy and say exactly which methods are reliable.
5. Preserve real Minecraft authentication via an official launcher profile in version 1. No cracked/offline impersonation, token collection or bundled game binaries.
6. Distribute only original mod source and assets, not Mojang game files. Create original assets; **do not copy the Larp Client logo, name, extracted textures, 3D models, audio or proprietary code**.
7. Use resource-backed asset loading; provide missing asset fallbacks. Never silently declare a feature done if it is only visual scaffolding.
8. Performance and accessibility: particle limits, quality presets, reduced-motion, effects toggle, frame-pacing awareness, memory/resource cleanup.
9. Server-specific compliance: multiplayer-safe defaults; warn that even purely cosmetic mods are used at the user's own risk and policies vary.
10. Every feature must have an owner, acceptance criteria, an integrated test and documentation. No placeholder download links or fake benchmarks.

## Project orchestration

You are the **lead integrator**. Use project subagents from `.claude/agents/` for scoped tasks. Agents should return changes and brief summaries with file names and tested commands. Delegate independent read-only or module-isolated work in parallel; serialize edits of shared lifecycle/bootstrap files. Review and integrate all changes before completion. If concurrency or tooling restrictions prevent parallel agents, run them sequentially without inventing work or claiming other agents ran.

## Output style

Report: what changed; build/test result and actual commands; screenshot/video evidence when feasible; known issues; next real milestone. Ask a question only if a genuinely blocking outside dependency exists. Otherwise choose sensible original defaults and proceed.
