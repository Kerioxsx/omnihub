## New in this version

**Game covers show again, and the library has a full-screen Big Picture mode.**

- **Covers, backgrounds and logos load again.** Three causes, all fixed: Windows didn't load OmniHub's own image addresses for art Steam keeps on your PC; Steam moved its web art to new addresses that OmniHub's security rules blocked; and newer Steam versions keep that art in different folders.
- **Steam's current art addresses**: OmniHub asks Steam's store which art each game has (no key needed) and keeps the answer for a week, so newer games get their covers too.
- **Covers for games from other stores**: Epic, Xbox, EA and other games use the cover of the same game on Steam, when Steam sells it under the same name.
- **Big Picture** (Library → Big Picture, or the View button on a controller): your library fills the screen. Rows for recently played, favourites, playing and all games. The focused game's art fills the background and drifts slowly, covers glide and grow as you move, with soft sounds. D-pad or stick to move, A to play, B to leave (keyboard: arrows, Enter, Esc).
- A game's name no longer shows twice on a coloured card when you hover it.

Also in 0.29.0: quiet mode, with the screens off and the fans at their lowest, from your phone.

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
