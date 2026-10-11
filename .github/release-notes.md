## New in this version

- **Music videos that start later are matched to the song.** Many official videos open with a story, so the song starts later than on the album and the video ran early. OmniHub now downloads the video's soundtrack (a few MB, not kept), listens to your song as the PC plays it, finds where the song is in the video, and moves the video so the two line up. It's remembered for each video. (Settings → More → *Match videos to the song by ear*.)
- **Spotify's Canvas and videos, mirrored.** Spotify doesn't let other apps read its animated art or videos, so OmniHub watches Spotify's window like a screen recorder (Apple Music's too), finds the part that moves, and shows it behind the lyrics, in step. Nothing is saved or sent. If Spotify is completely covered or minimised it may stop drawing; then the cover shows, and OmniHub says so.
- **Closer to the edits:** the word being sung splits into pink and cyan copies and jolts on hard beats; a new word arrives with an echo trail; and in the light show the colours swing round on drops. Each has its own setting.
- **Lyrics in time with the voice.** For lyrics that only time whole lines, the words were spread too fast (the highlight ran ahead of the singer); they now take the line's whole time.
- **Never stuck in full screen.** OmniHub no longer comes back in full screen after closing or updating during full-screen Aurora; F11 and Esc leave full screen anywhere.
- **Clearer videos.** A small note beside *Aurora / Lyrics* says what the music video is doing (looking, loading, playing, or why it can't play), and the blurred cover around a "whole picture" video only shows while the video does.

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
