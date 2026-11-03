## New in this version

**A Steam Deck–style game library, emulators, and counters with events, goals and a stream overlay.**

- **Game library** (Games → Library): every game as its real cover art, like the Steam Deck. Steam games get their covers, background and logo from Steam (or the art you set in Steam); other games get a coloured card until you pick art. Filter by favourites, playing, backlog and finished, search, and move around with a **controller** (D-pad, A to open, B to go back).
- **Full-screen game page**: the background art and logo, Play, playing now on Steam, your hours and last played, then **achievements**, **screenshots** you took (Steam's and the app's own), the **soundtrack** (plays a Steam soundtrack you own, with links to find it on YouTube and Spotify), **mods**, **save backups** and **art & limits**.
- **Session recap**: after you close a game, how long you played, its achievements and screenshots from that session.
- **Save backups**: OmniHub suggests where a game keeps its saves; back them up with one click or after every session (the newest 10 kept), and put one back.
- **Daily time limits** per game: a warning near the end, then the game is closed.
- **Steam extras** with your own Steam Web API key (kept encrypted on this PC): achievements, friends online and what they play, and your **wishlist with sale alerts**. SteamGridDB art with your own key.
- **Emulators** (Games → Emulators): finds RetroArch, mGBA, melonDS, Dolphin, Cemu, Ryujinx, DuckStation, PCSX2, PPSSPP, RPCS3 and more, scans your ROM folders with box art, and starts any game in the right emulator.
- **Counter events**: give a counter events with their own keys (Wrong shiny, Fainted, Phase…). Each one is logged with the count, and **phases** start the "this phase" count again. **Undo** takes back the last change.
- **Goals**: get told when a counter reaches a number, plus odds alerts (1×, 2×, 3× past the odds) for shiny hunts. A **celebration** when you find a shiny.
- **Hunt stats**: shinies found, average encounters, hunting time, luckiest and longest hunts, finds by month.
- **Stream overlay**: show counters in OBS as a Browser source (`http://127.0.0.1:47823/counter/<id>`), see-through, with size, colour, picture, timer and odds options. Only this PC can open it.
- **Controller buttons as counter keys**: press a button on an Xbox (or XInput) controller to count.
- **Counters on your phone** (More → Counters): count, log events and start the timer.
- Fixed: a counter key could get stuck and stop counting after a game grabbed the keyboard; the key watcher now repairs itself.

Also in 0.24.0: counters with keys from anywhere, shiny hunting, the auto clicker, the Pokémon save editor and the glass look.

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
