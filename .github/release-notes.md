## New in this version

- **Aurora — a new way to see your music.** Open Music (or full screen) and the song itself becomes the show:
  - **the cover art is the visual**, big and vivid on black: it swells with the bass, glitches, splits its colours and jumps on the beats, and changes with every song. Styles: **Visual**, **Fisheye**, **Fisheye Visual**, **Minimal** (mostly dark) and **Ambient** (colour fields);
  - **a thin neon light around the screen** in the cover's colours: a crisp edge, an inner highlight, a soft glow and a wider bloom. Bass widens it, beats lift it, quiet parts soften it; it never flickers;
  - **huge floating lyrics**: the word being sung in the middle, the one before above, the next one in a pill below. Word by word when the lyrics time each word; otherwise a whole line at a time (no guessing). Instrumental breaks show breathing dots, and the next line appears just before it is sung;
  - pausing freezes everything where it is (then shows the cover as a card); seeking and skipping update at once and never show the last song's lyrics.
- **The Apple Music–style Lyrics view is still there.** Switch between **Aurora** and **Lyrics** at the top left (or press V); OmniHub remembers your choice.
- **Settings for everything** in a compact glass panel (Settings in the player's dock): gradient and colours (from the cover, or your own two, several or one), animation (music sync, idle, none), thickness and glow, visual style, intensity and speed, lyric font, size, weight, highlight, position and more, sensitivity and bass response, reduced motion and no flashes. Changes show at once and are saved. **Reset to defaults** is one click. **L** shows or hides the lyrics.
- **Glow around your screen** (optional): the same light around your monitor, over every app, on one display or all. Clicks go straight through it, and it steps aside while a game or video is full screen.
- **Your own lyrics files**: choose a folder of `.lrc` files and they are used first (matched by the file's tags or "Artist - Title.lrc" name, and only when the length fits the song).

**How Aurora hears the music:** it reads the sound Windows is already playing (the mix you hear, called loopback), only while Aurora is on screen. It never uses the microphone, never records and never sends sound anywhere. Turn off *Follow the music* to stop it; the light then moves on its own.

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
