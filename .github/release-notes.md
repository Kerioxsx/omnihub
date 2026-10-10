## New in this version

Aurora, after the first real-world try:

- **Sharper covers.** Music players give Windows a small cover (often 300×300), which looked soft stretched across the screen. OmniHub now finds the same cover at **1200×1200** in Apple's iTunes catalogue — only when the song and the picture match — and draws every cover with sharper scaling. Only the song's title, artist and album are sent; turn it off under *Lyrics and covers*.
- **More effects, each on its own slider.** Beat zoom, glow on bright parts, echo, ripples, twist, shake, glitch, colour split, pixelate on beats, fisheye, kaleidoscope, halftone dots, scanlines and duotone, with **Calm**, **Default**, **Wild** and **Shuffle** presets. Glitch is subtle by default now.
- **Lyrics word by word for every song.** Most lyrics online only time each line; Aurora now spreads a line's words over it by their syllables, so the highlight moves word by word (close, not exact — songs whose lyrics time each word still use those exact timings). Turn *Estimate words* off for whole lines.
- **Only the word being sung is highlighted.** The next word below is now quiet instead of sitting in a coloured pill.
- **Lyrics that move with the music.** Words pop in, bounce on beats and sway gently. A new **Movement** slider sets how much (Reduced motion keeps them still).

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
