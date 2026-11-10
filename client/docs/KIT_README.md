# Minecraft 1.8.9 PvP Client — Claude Code Build Kit

This is a **development plan and Claude Code orchestration kit**, not a finished mod. It targets a legal, client-side Forge 1.8.9 `.jar` with an integrated launcher-like menu and polished cosmetics, PvP visuals, configurable HUD, and performance options. The video references show **Larp Client**; use them for close aesthetic reference but create original implementation, branding and assets.

## How to use

1. Extract this ZIP into a new, empty local folder.
2. Open that folder in **Claude Code** (not plain Claude chat if you want agent files to be invoked automatically).
3. Read `MASTER_PROMPT.md`, then paste its contents into the Claude Code session. Alternatively tell Claude Code: `Read MASTER_PROMPT.md and execute it with the project subagents.`
4. Keep `CLAUDE.md` and `.claude/agents/*.md` in this directory. If the subagents are not visible in an existing Claude session, restart Claude Code in this directory.
5. The `references/video/` directory includes the two uploaded reference videos, and `references/frames/` includes frame captures. `references/ANALYSIS.md` explains where to look.
6. Have a legitimate Minecraft: Java Edition installation and a supported Java 8 JDK ready; Claude should verify versions, download the **official 1.8.9 MDK** from Minecraft Forge and build a mod from scratch.

## Deliverable milestones

- **Milestone A:** A Forge 1.8.9 mod starts, custom title GUI works, settings save and reload.
- **Milestone B:** HUD editing and a local preview scene work; animation engine and particles function.
- **Milestone C:** Multiple original kill effects and cosmetics work, with honest kill-attribution handling.
- **Milestone D:** Performance tuning, packaging, client-only safety checks, compatibility testing, and documented release `.jar`.

## Expected handoff files

`build/libs/<original-client-name>-<version>.jar`, `README.md` for installation, `docs/USER_GUIDE.md`, `docs/TEST_RESULTS.md`, `docs/KNOWN_LIMITATIONS.md`, editable sources and assets.

## External references

- Minecraft Forge 1.8.9 downloads: https://files.minecraftforge.net/net/minecraftforge/forge/index_1.8.9.html
- Minecraft Usage Guidelines: https://www.minecraft.net/en-us/usage-guidelines
- Minecraft EULA: https://www.minecraft.net/en-us/eula
- Hypixel allowed modifications: https://support.hypixel.net/hc/en-us/articles/6472550754962-Hypixel-Allowed-Modifications
- Claude Code subagents: https://code.claude.com/docs/en/sub-agents

The video content is an **inspiration/reference**; we cannot promise its effects are actual vanilla-compatible gameplay features or that every animation will be visible to other multiplayer players.
