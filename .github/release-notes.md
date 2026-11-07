## New in this version

**Timers and bedtime, free game alerts, PC checks, a game mover, a soundboard, text shortcuts, a colour picker, a screen translator, elimination clips, and more.**

- **Timers, alarms and a bedtime lock** (Tools → Timers): they ring on the PC and on your phone. Bedtime warns you, then locks the PC at the time you set (snooze if you must).
- **Shut down or sleep when downloads finish** (Tools → Downloads): OmniHub waits until the internet has been quiet for five minutes.
- **Slow down an app's internet** (Tasks → Online programs): cap how fast a program uploads, like a cloud backup or a torrent, so your game or call stays smooth. Windows only lets this cap uploads, not downloads; it asks for administrator approval.
- **Soundboard** (Quick actions → Soundboard): your sound files on buttons and keys that work inside games, to your speakers or into your microphone through a virtual audio cable.
- **Text shortcuts** (Quick actions → Text shortcuts): press a spare key (F13–F24, Pause, Scroll Lock, the numpad) and OmniHub types your saved text into the app in front.
- **Pin any window on top** and **Wi-Fi for guests** (Tools → PC checks): keep a video or guide above everything; show a QR code that joins your Wi-Fi.
- **Colour picker and ruler** (Tools → Images): freeze the screen, click any pixel to copy its colour, drag to measure in pixels.
- **Screen translator** (Tools → Images): drag over text in a game or app; Windows' own text recognition reads it and the AI model on your PC (Ollama) translates it. Nothing is sent anywhere.
- **Clip my eliminations in any game** (Games → Clips & Capri): with instant replay on, OmniHub reads the middle of the screen and saves a clip when it sees "Eliminated" or your own words. Works best in borderless or windowed mode.
- **Free game and patch notes alerts** (Games → Extras): free games on Epic and elsewhere, and new patch notes for your Steam games.
- **PC checks** (Tools → PC checks): drive health from each drive's own report, a temperature stress test, and a "ready to play?" check before a game.
- **Move a game to another drive** (a game's page → Move): copied, checked file by file, and a link left behind so every launcher still finds it.
- **Music per game**: a scene can start a song or playlist, and scenes can run when a game starts.
- **Big Picture keyboard**: search the library with a controller using an on-screen keyboard.
- **Aim trainer** (Games → Extras): a flick drill and a reaction test to warm up, with your bests kept.

Also in 0.32.0: timers and alarms on your phone (More → Timers).

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
