## New in this version

**Capri plays along: instant replay, “Capri, clip that”, and reactions to your games. And autofill in any browser.**

- **Instant replay** (Games → **Clips & Capri**): while a game runs, OmniHub keeps the last minutes of your screen and game sound (1 to 20 minutes), encoded on your graphics card (NVIDIA, AMD or Intel) so it barely costs FPS. Save with **Clip last 30 s**, **Start recording** / **Stop recording**, or your voice. Clips go to Videos\OmniHub Clips. The recorder (FFmpeg, about 130 MB) downloads the first time you turn it on.
- **Voice commands**, with Windows’ own speech recognition, on your PC: **“Capri, clip that”**, **“Capri, record the last ten minutes”** (1–20 minutes, or 15–90 seconds), **“Capri, start recording”**, **“Capri, stop recording”**, **“Capri, call me …”** (boss, king, daddy, legend… or names you add), **“Capri, what’s my FPS”**, **“Capri, take a screenshot”**. It only listens while a game runs, unless you change that.
- **Capri reacts to your games**: when they start and end, when you clip something, when your FPS drops. In **Counter-Strike 2** (Valve’s official game-state link) she follows rounds, multi-kills, MVPs, the bomb and the result; in **League of Legends** (Riot’s live game data) kills, multikills, dragons, barons and the result. Triple kills and better are clipped by themselves. Other games don’t share what happens in a match, so there she reacts to starting, stopping, clips and FPS.
- **Autofill in any browser**: Vault → Browser autofill lists your browsers (Chrome, Edge, Brave, Opera, Opera GX, Vivaldi, Chromium, Firefox); switch on the ones you want, one or several, and **Open its extensions page** takes you there.
- Everything new is off until you switch it on, and runs only while a game does.

Notes: Capri needs “follow your coding assistant” on in her menu to hear OmniHub. Counter-Strike 2 picks up OmniHub’s link the next time it starts. Regular Firefox keeps an add-on Mozilla hasn’t signed only until it restarts (load it again from about:debugging, or use Firefox Developer Edition).

Also in 0.18.0: hidden files (any size, under your vault password) and locked notes.

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
