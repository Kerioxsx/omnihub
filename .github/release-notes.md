## New in this version

**Any emoji pack.** Aurora's *Your font & emoji pack* now takes any pack, not only game textures:

- **A zip of pictures** — PNG, GIF (animated ones move), WebP, JPG or SVG. Twemoji, Noto, OpenMoji, Apple or Samsung emoji dumps, Discord or Twitch emotes, a game's item textures… Each emoji the lyrics earn gets the pack's matching picture, however the pack names its files: by code point (`1f525.png`, `emoji_u1f525.png`, `U+1F525.svg`), by the emoji itself (`🔥.png`), by its name (`fire.png`, `red_heart.webp`, `Fire_3d.png`, `face_with_tears_of_joy.svg`), by its short code (`joy.gif`, `heart.png`), or by the word in the song ("apple" → `apple.png`). Pixel-art pictures stay crisp; smooth ones stay smooth.
- **Or an emoji font** (.ttf, .otf, .ttc, .woff): every emoji is drawn in it.
- Where the pack has nothing for an emoji, the plain emoji shows. Your files stay on your PC; nothing is uploaded or shipped with OmniHub.

**Also in 0.7.1** (last update): your own lyrics font, an emoji at least every two lines, the whole cover again, clearer reasons when a music video can't play, and faster releases.

**Spotify's animated covers (Canvas):** Spotify doesn't let other apps read them (there is no public way), so OmniHub can't show them. Animated covers come from Apple Music when the album has one there.

**What is sent where:** to find a video, only the song's title and artist go to YouTube; for animated covers, to Apple Music; for covers, to Apple and Deezer; for lyrics, to LRCLIB and NetEase. Every one can be turned off.

**How Aurora hears the music:** it reads the sound Windows is already playing (the mix you hear, called loopback), only while Aurora is on screen. It never uses the microphone, never records and never sends sound anywhere.

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

The FPS meter uses Intel PresentMon {{PRESENTMON}} (`OmniHub-PresentMon-x64.exe`, MIT license in
`OmniHub-PresentMon-LICENSE.txt`). You don't need to download it either: OmniHub fetches and checks
it when you turn on the FPS meter in Games.

`SHA256SUMS.txt` lists the SHA-256 checksum of each file.
