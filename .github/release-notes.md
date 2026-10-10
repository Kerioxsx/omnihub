## New in this version

**A quick update to Aurora:**

- **Your own font and emoji pictures.** Under *Your font & textures* in Aurora's settings, choose a font file (.ttf, .otf or .woff) for the lyrics and a zip of item textures for the emojis, for a Minecraft look: words get the matching picture ("apple" → the apple, "diamonds" → a diamond, "blue" → the heart of the sea, love → a heart), drawn pixel-sharp. Your files are copied into OmniHub's data folder and stay on your PC; nothing is uploaded or shipped with OmniHub.
- **An emoji at least every two lines.** Lines with a word to picture get theirs as before; when two lines in a row have none, the second gets one anyway (a sparkle, a note, a star…). "Bye", "goodbye" and "hello" get 👋.
- **The whole cover again.** Everyone is switched back to the whole cover once (nothing cut off at the top or bottom); "Fill" is still there and now says that it cuts.
- **Music videos: clearer and more reliable.** If the app missed the moment a video was found, it now asks again until it knows, and when a video can't play, Aurora says why (the artist doesn't allow it in apps, YouTube blocked its player, the stream failed…) instead of quietly showing the cover.
- **Faster releases.** Updates now build and publish in a fraction of the time.

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
