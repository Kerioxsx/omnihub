## New in this version

**Scenes, every game store, night light, local AI, Ask your PC, a speedrun timer, a crosshair, desktop widgets, a live wallpaper, and more.**

- **Scenes** (Quick actions → Scenes, the deck and your phone): one tap sets everything: lights, fan profile, notifications, Discord status, where the sound comes out, the microphone, the screen resolution and HDR, a game, keys. Four ready-made ones: Gaming, Movie, Sleep and Back to normal.
- **Display and audio per game**: a scene can run by itself when a game starts. When the game closes, the screen and HDR go back as they were, and an "after" scene can run.
- **Every game store in the library**: GOG, Ubisoft Connect, EA, Battle.net and Xbox / Game Pass games now appear next to Steam and Epic.
- **Night light** (My PC → Monitors): warmer, optionally dimmer screens from evening to morning, fading in and out, with your monitors' brightness for the night. You can turn it on or off for now.
- **Local AI in Chats**: chat with models on your own PC through Ollama, free and offline, on the PC and on your phone.
- **Ask your PC** (Home): ask "why is my PC slow?" or "what's using my internet?" and get an answer from what OmniHub measures right now. A local model can explain it in a sentence if you like.
- **Speedrun timer** (Counters): splits, personal bests and best segments in a small window over the game, with keys that work inside games.
- **Crosshair overlay** (Games → Extras): your own crosshair over any game; clicks go straight through it. Some games with strict anti-cheat dislike overlays, so it's off until you turn it on.
- **Desktop widgets and a live wallpaper** (Quick actions → Desktop): temperatures, now playing, a counter, batteries, network speed or a clock on your desktop, and your OmniHub theme animated behind your icons.
- **Creators going live and a daily recap** (Games → Extras): an alert when Twitch or YouTube creators you follow start streaming, and an evening summary on your phone.
- **Similar photos** (Storage → Duplicates): finds look-alike photos and screenshots and keeps the best one.
- **Family and guest phones** (Phone → Limit): limit a paired phone to a few areas, like only the deck and the music.
- **Synced folder** on your phone (More → Synced folder): everything new in a PC folder downloads in one tap, and files you pick go straight into it.

Also in 0.30.3: Karaoke Studio.

Updating from 0.2.1 or later: OmniHub installs this by itself (or Settings → About → Install now), or download the setup below and run it.

## Install

1. Download **`OmniHub_{{VERSION}}_x64-setup.exe`** below.
2. Run it. It installs for your Windows user only, so no administrator rights
   are needed (unless an older OmniHub is in Program Files: then it asks once). Windows 10 or 11; WebView2 is installed automatically if it is
   missing.
3. The installer is not code-signed yet, so Windows SmartScreen will warn
   you: click **More info → Run anyway**.

Prefer an MSI (for example for deployment tools)? Use
`OmniHub_{{VERSION}}_x64_en-US.msi` instead.

The phone app needs no install: in OmniHub open **Phone → Pair a phone** and
scan the QR code with your phone (same Wi-Fi network).

iPhone mirroring uses the AirPlay add-on (`OmniHub-AirPlay-addon-x64.zip`,
UxPlay with its GStreamer runtime). You don't need to download it: OmniHub
fetches and checks it when you click **Install AirPlay receiver** on the
Screen share page. Its source is in `OmniHub-AirPlay-addon-sources.zip`.

Instant replay uses FFmpeg {{FFMPEG}} (`OmniHub-ffmpeg-x64.exe`, LGPL; license and source links in
`OmniHub-ffmpeg-LICENSE.txt`). OmniHub fetches and checks it when you turn on instant replay.

The FPS meter uses Intel PresentMon {{PRESENTMON}} (`OmniHub-PresentMon-x64.exe`, MIT license in
`OmniHub-PresentMon-LICENSE.txt`). You don't need to download it either: OmniHub fetches and checks
it when you turn on the FPS meter in Games.

`SHA256SUMS.txt` lists the SHA-256 checksum of each file.
