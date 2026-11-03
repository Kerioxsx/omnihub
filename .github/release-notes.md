## New in this version

**Fortnite presets and checks, OBS on the deck, a Discord status, a network monitor, backups, controller mode, and screenshots you can search by their text.**

Fortnite (Games → Boost profiles → Fortnite):
- **Presets**: Competitive (the most frames), Balanced (DirectX 12, medium quality) and Pretty (DirectX 12, epic quality). There are now quality levels besides "lowest", and your own settings stay backed up.
- **FPS by settings**: each FPS-meter session remembers the video settings it was played with, and the FPS card compares average FPS and 1% lows for each set, so you can see what's really faster on your PC.
- **Replays**: see how much space they take, and send old ones to the Recycle Bin (the 5 newest and the last week are kept).
- **Wi-Fi power saving off** (it causes ping spikes), with a note when you're on Wi-Fi instead of a cable.
- **Graphics driver**: its version and age, with a link to update it. After a driver update it suggests cleaning the shader caches.
- The ping test's best region now skips regions that drop packets, and says where to pick it in Fortnite.

Everything else:
- **OBS buttons** for the deck and the wheel: switch scenes, start or stop streaming or recording, save the replay buffer, mute a source. Uses OBS's own WebSocket server; the password is kept encrypted.
- **Discord status** (Settings → General): the game you're playing and your shiny hunt, under your name in Discord. Discord needs an app for this, so you make a free one in its Developer Portal and paste its ID.
- **Network monitor** (Tasks): download and upload speed now, data used per day for the last 30 days, and which programs have connections open and to where. Windows only tells administrators how much data each program uses, so that isn't shown.
- **Backups** (Tools → Backups): your folders copied to another drive daily, weekly, or whenever it's plugged in. Only new and changed files are copied, and nothing in the backup is ever deleted.
- **Controller mode** (Settings → General): use the whole app with a controller. D-pad or stick to move, A to press, B to go back, LB/RB to switch pages, Start to search.
- **Search screenshots by their text**: OmniHub reads the text in your screenshots in the background (with Windows' own text recognition, on this PC, never while you play), so searching finds them by what they show.
- **Wake and play from bed**: starting a game from your phone turns the screen on, and the Phone page shows what a Wake-on-LAN app needs to wake the PC from sleep, with how to allow it.

Also in 0.27.0: updates that always finish, and the Bleach themes.

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
