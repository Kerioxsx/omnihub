## New in this version

**Karaoke Studio: sing along and learn songs in any language.**

- **A new tab in Music** (Music → Karaoke Studio), with its own settings and history. The music player works exactly as before.
- **Pronunciation written for you**: the first time, pick your first language and accent from a searchable list (Spanish (Mexico), English (UK), Portuguese (Brazil), Hindi (India), Japanese and many more). Under every word you see how to say it in your own spelling, its syllables, and tips for sounds your language doesn't have. Russian and Ukrainian, Japanese kana, Korean and Greek are re-spelled sound by sound; Spanish gets syllables with the stressed one marked. OmniHub says when it can't do more instead of guessing.
- **Words light up as they're sung**, with the pronunciation following the same timing. Lyrics that only time whole lines are marked as approximate, and "Lyrics earlier / later" corrects them.
- **Any song**: what your PC is playing (Spotify, Apple Music, YouTube…) or a song file from this PC. Lyrics come from an .lrc file beside the song, your lyrics folder, LRCLIB or NetEase, or an .lrc file you pick.
- **Follow the song**: the music keeps its time and the microphone shows your note, how close you are to it, and a summary at the end (steadiness of held notes, entrances against the lyrics, your comfortable range). Only what was really measured is shown.
- **Follow my voice** (song files from this PC): the song waits for you before each line, eases off and pauses when you stop, and goes on from the word you stopped on. A short breath changes nothing. Slower practice speeds keep the key. Headphones work best.
- **Loop a line, practise the lines worth another go**, and see your history. The microphone is analysed on your PC and never recorded; only the summary numbers are saved, and you can delete them.

Also in 0.30.2: ZANGETSU v2 themes, the 3D button studio, quick sites, and Bluetooth and sound output from your phone.

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
