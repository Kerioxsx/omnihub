## New in this version

**Vesper Client 0.3.0: jump marks, rain and puddles, emotes with cars, a flame head with horns, hit sounds and more.**

- **World colour:** pick one neon colour (or rainbow) for your world; the new effects follow it.
- **Jump marks:** every jump leaves a neon mark on the floor: six-point star, rune circle, ring, triangle or diamond, with sparks and a shockwave when you land from high up.
- **World glyphs:** neon symbols floating around you.
- **Rain & wet ground:** rain that stops at roofs and splashes, with sound, and puddles on the ground that mirror the sky and ripple, always or only when it rains.
- **Hit sounds:** six sounds of Vesper's own (made from scratch) or Minecraft's, a different one on critical hits.
- **Emotes (press B):** Flex, Wave, Dance, Salute, Cheer and Car flex: a car drops in next to you with neon underglow (Supercar, Muscle car or Hypercar; original designs, any colour).
- **Wraith head:** your head becomes a ghostly flame with spiked, glowing horns.
- **Also:** speed blur while sprinting, a neon block outline, a time changer, and even smaller items in your hand.
- Only you see all of this. Tested in Minecraft with software rendering; the sounds weren't heard there (no audio device), and some options weren't screenshotted. Details are in `client/docs/TEST_RESULTS.md`.

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
