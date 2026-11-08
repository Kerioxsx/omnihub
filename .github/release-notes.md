## New in this version

**A new Big Picture, made for the couch, with your shows in it.**

- **New look**: Home, Library, Watch and Settings tabs (switch with the bumpers), a big "pick up where you left off" card, landscape rows, a cover grid with filters and search, and four backgrounds (Nebula, OLED black, Aurora, Ember). Reduce motion and sounds in its Settings.
- **Continue watching**: the series and videos you're part way through on Netflix, Crunchyroll, Disney+, HBO Max, Prime Video, Paramount+, Apple TV and YouTube, from your browser's history; A resumes where you left off. It reads only those sites' watch pages, from a copy on your PC that is deleted straight after, and you can turn it off.
- **Pick your browser**: the first time you open a channel, Big Picture asks which browser to use (Edge, Chrome, Brave, Firefox, Vivaldi, Opera or your usual one). Change it in Big Picture's Settings or in Games → Sites. Channels open full screen with no bars.
- **The controller as the remote**: in a channel, the left stick moves the pointer, A clicks, the right stick scrolls, X plays and pauses, Y goes full screen, the D-pad seeks and changes volume, B is Esc, LB / RB move between buttons, LT goes back a page, and View closes the channel and brings you back. It works only while that browser is in front. Xbox controllers and others Windows sees as one (XInput); PlayStation controllers need Steam or DS4Windows for this.
- Everything in Big Picture works with a controller, the keyboard (arrows, Enter, Esc, [ and ]) or the mouse.

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
