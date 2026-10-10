## New in this version

**0.6.1 — smoother lyrics.** In Aurora's big word-by-word lyrics, words that had been sung were meant to fade away and be removed, but while someone kept singing they stayed on the page, invisible and blurred — a dozen or more per line. That cost the graphics card more with every word, and on a slower PC the fading words could pile up on top of each other. Now each word fades on its own and is gone a third of a second later, with at most three fading at once.

**From 0.6.0 — Aurora goes cinema.** The song now fills the screen, behind short lyrics in the cover's own colours:

- **The music video as the background.** When the artist has an official music video, Aurora plays it full screen behind the lyrics — muted (your music app is still the sound), in YouTube's own player, **up to 4K** when the artist uploaded it in 4K, and kept **in step with the song** (it seeks when the song jumps and eases its speed for small drifts). Only the artist's own uploads count (their official channel or VEVO), not lyric videos or fan uploads. Videos with a story before the song can be moved with **Video timing**, remembered per video.
- **Else the animated cover**, when the artist has a visualizer for the song.
- **Else the whole cover, full size.** OmniHub now fetches the original cover from Apple's catalogue (usually 3000×3000), so it stays sharp on any monitor — even 4K — and it drifts slowly so the screen never feels frozen. Wide screens crop around the cover's detail, never its middle at random.
- **Else the light show** (the Visual styles and effects from 0.5).
- **Lyrics in the cover's colours.** A light tone of the cover's own colour, with a matching glow; on bright covers or videos Aurora adds just enough shadow and darkening to keep every word readable.
- **Lyrics where the picture is calm.** Aurora finds the quietest band of the cover (not a face, not the title) and puts the lyrics there; or pick top, middle or bottom.
- **3D emojis, used sparingly.** A glossy emoji appears beside a word you can picture or feel — a heart, the moon, fire, tears — never on every line, and never twice in a row. *Now and then*, *More often* or *Off*; 3D or Windows style.
- **Corner light** in the cover's colours, breathing with the bass.
- **Settings** under *Background*: what plays behind the lyrics (video else cover, video, cover or light show), music videos on or off, fill the screen or whole picture, what to keep in view when cropped, video quality (best, 1080p, 720p), cover motion, darkening and corner light. The panel says honestly what is on screen ("The music video from … · playing in 4K (2160p)").

**What goes to YouTube:** only the song's title and artist, to find the video, and the video itself plays from youtube-nocookie.com. Turn off *Music videos from YouTube* to stop both. The 3D emojis are Microsoft's Fluent Emoji (MIT licence), built into OmniHub — nothing is fetched for them.

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
