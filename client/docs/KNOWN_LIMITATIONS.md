# Known limitations — Vesper Client 0.3.0

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
- **World lyrics need OmniHub** on the same PC (0.45 or later) with lyrics found for the song; without it nothing shows. They were tested against a stand-in server, not OmniHub itself.
- **Sword animations** replace Minecraft's item renderer at run time (ADR-8). Another mod that does the same (some animation mods, possibly OptiFine features) may conflict; turn Sword animations off if hands look wrong. The Spin style turns around the grip, so the blade sweeps out of view.
- **Motion blur** copies the screen every frame; on slow GPUs it costs some FPS. Menu blur draws 16 offset copies while a menu is open (8 on Low-end).
- **FOV above 110** is applied only while a frame is drawn; Minecraft's own setting is never changed. It isn't applied while the Options or Video Settings screens are open.
- **Emotes and the Wraith head** replace the player model at run time (ADR-11); another mod doing the same may conflict. Other players don't see your emotes, car or head.
- **Rain and puddles** are drawn on top of the world; they are not real water and don't reflect blocks or players, only the sky's colour. Rain can show through glass roofs that let rain through in Minecraft too.
- **Hit sounds and the rain sound** weren't heard in testing (the test machine has no audio device).
- **Time changer** sets the time on your screen only; daylight and mob behaviour on the server don't change.
