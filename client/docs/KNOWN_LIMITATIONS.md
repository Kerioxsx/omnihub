# Known limitations — Vesper Client 0.1.0

- **Only you see it.** Cosmetics, skies and kill effects are drawn on your own screen. Other players see nothing unless they run Vesper too, and even then they don't see *your* cosmetics (there is no server or sync).
- **Kill effects on public servers.** The game only tells a client about a kill for sure in singleplayer or on a LAN world you host. On servers, kill effects play only if you turn on "Kill messages from chat" (best effort, English death messages, and only for someone you hit in the last 5 seconds). A target that just disappears never counts.
- **Target info** (name · health · distance) is off by default and only works in singleplayer unless you allow it; some servers forbid showing opponents' health.
- **Not yet tested with the official launcher** using the release jar, with OptiFine, or on real servers. See `TEST_RESULTS.md`.
- **Performance numbers** so far come from a machine without a GPU (software rendering). Real frame-time measurements need a gaming PC.
- **1.8.9 only.** It's a Forge mod for Minecraft 1.8.9. Other versions would need a separate port (Fabric for modern versions); none exists yet.
- **First-person**: wings and plume are drawn on your body, so in first person you won't see them (like any cosmetic).
- **Sounds** use Minecraft's own sound events; there are no custom sounds yet.
- **Showroom prop** (the Skiff hover-cart) is decoration in the showroom only; it isn't in the world and can't be driven.
- The reference videos showed another client's features; Vesper's look is original (its own logo, glyphs, models made in Blender from code in `models/vesper_models.py`, and procedural effects). Nothing was copied from that client.
