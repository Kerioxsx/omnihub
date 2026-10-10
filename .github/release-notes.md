## New in this version

**Aurora, take two** — after the first week with it:

- **Music videos play.** YouTube refuses official music videos to players inside apps (that's why the video never showed), so OmniHub now plays them through a small page on its own website, the way any site embeds a video. When an artist allows their video *only* on YouTube itself, OmniHub plays its stream directly with **yt-dlp** (downloaded once, about 60 MB, checked against its published checksum; turn it off under More settings). Listening to the normal song still shows the official video, and when there are several official videos, the **most watched** one plays. Spanish titles like "Video Oficial" count too.
- **Animated album covers.** When the album has a moving cover on Apple Music (up to 4K), it plays behind the lyrics — and the **light show** now runs its glitch, halftone and colour effects over the moving cover, like the Spider-Verse edits.
- **The whole cover, sharp.** The cover now shows **all of it** (nothing cut off at the top or bottom), as tall as the screen, its sides blending into a soft glow of itself; "Fill the screen" is still there. Remixes, live and acoustic versions now get *their* cover, not the original's, and Deezer is a second source when Apple's catalogue has nothing (up to 1900×1900).
- **Lyrics for more songs.** NetEase Cloud Music is a second source of time-synced lyrics (Latin and K-pop included) when LRCLIB has nothing, and songs that had no lyrics are looked up again every few days instead of never.
- **More emojis, in Spanish too, in the lyrics' colour.** Emojis now understand Spanish ("corazón roto" 💔, "te extraño" 🥺, "esta noche" 🌙, "lágrimas" 😢…), show up on any line with a word you can picture, and by default are **plain, one colour — the same as the lyrics**, like the reference edits (3D is one click away).
- **Simpler settings.** A short *Quick* section — what's behind the lyrics (Video, Cover, Light show), animated covers, the cover's fit, lyrics and emojis — with everything else under *More settings*. A line always says what is on screen and why.

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
