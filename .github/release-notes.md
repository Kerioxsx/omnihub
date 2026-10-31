## New in this version

**Aurora becomes a music hub: search every music service from the player. Plus a Tools page, a panic button and voice commands for your PC.**

- **Search in Aurora** (the bar above the controls, or press **/**): songs, albums and artists. Pick one and it plays in the service you choose: **YouTube Music** plays the song itself; **Apple Music** opens it in the Apple Music app (or the web player); **Spotify**, **Spotify Web Player**, **SoundCloud** and **Amazon Music** open on it (press play: they only let their own signed-in apps start a song). **Automatic** uses the app that is playing, else your music app.
- **Artist cards**: picture, a short bio, **Play** and **Shuffle**, top songs, and their albums in a carousel. Albums open to their songs.
- **Up next** beside the song: the rest of its album and more by the artist. One click plays any of them.
- **A big clock** at the top of Aurora (12- or 24-hour). All of it can be switched off in Aurora's settings → Music hub.
- **Tools page**: a **checksum checker** (paste the SHA-256 from a website, pick the file), a **secure shredder** (overwrite, then delete), **bulk rename** with a live preview ({name}, {n:3}, {date}, find and replace, letter case), **recent files**, **ports** (“what is using port 3000?”, with End), and **window layouts** (save where your windows are, put them back with one click, starting missing apps).
- **Command palette answers**: type **12*4**, **15% of 80** as **15% * 80**, **5 km in miles**, **100 f to c**, **uuid**, **base64 hello**, **sha256 text**, a Unix timestamp, or paste JSON to tidy it. Enter copies.
- **Privacy & security**: a **panic button** (Settings, the tray menu, or your phone) locks the vault, cuts every phone off, stops screen sharing and locks the PC; a **notification when a paired phone connects**; and a **privacy dashboard** showing which apps used your microphone, camera or location, from Windows' own record.
- **Voice**: **“Capri, lock my PC”**, **“Capri, open Spotify”** (any app in your Start menu), **“Capri, pause the music”**, **“next song”**, **“mute”**, **“Capri, panic”**.
- **Phone**: take a **screenshot of the PC** and see it on the phone, **print** a file on the PC's printer, a **panic** button, and home-screen shortcuts (long-press the app icon: Lock PC, Screenshot, Mute PC).
- **Settings backup** now includes your window layouts.

Notes: the music search uses Deezer's public catalogue and Wikipedia for bios; only what you type (and the playing song's title and artist, for Up next) is sent. The shredder can't reach copies an SSD keeps internally; BitLocker is what protects deleted files there.

Also in 0.19.0: instant replay, “Capri, clip that”, Capri reacting to your games, and autofill in any browser.

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
