## New in this version

**Aurora: sharp covers again, songs that start straight away, and more.**

- **Sharper covers, fixed.** When the full-size cover couldn't be matched once, OmniHub remembered that for good and kept the player's small thumbnail. Now every song is looked up again (and again after two days if nothing was found), and the match accepts covers that the player cropped or re-compressed a little. Covers come from Apple Music (usually 3000×3000) or Deezer (1900×1900).
- **Drawn sharper:** the full-size cover is drawn at the size it moves to, so the slow zoom doesn't soften it. A small cover is never stretched across the screen any more: it shows whole, with a shadow, over its own light.
- **See the cover's quality:** Aurora settings → Sharper covers shows the cover's size and where it came from, with **Look again**.
- **Spotify songs start right away.** Connect Spotify once (Aurora settings → Music hub → Play Spotify songs right away): songs, albums and artists you pick in Aurora start playing in Spotify in the background, without switching to Spotify. A song plays in its album, so the album continues after it. Spotify starts minimized if it's closed. This needs your own free Spotify developer app and Spotify Premium (Spotify's rule); the steps are in the settings. Without it, Spotify opens on the song as before.
- **Play next:** with Spotify connected, every song row in Aurora has a Play next button that adds it to Spotify's queue.
- Party mode and the Music quick action also start songs straight away when Spotify is connected.
- Not tested on a real Windows PC with Spotify yet (built and checked here). If connecting fails, check that the Redirect URI in your Spotify app is exactly `http://127.0.0.1:43827/callback`.

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
