## New in this version

**Quick actions (a macro deck for your PC and phone, a quick-launch wheel, clipboard history), focus sounds, batteries, a PC health score and Tidy Downloads.**

- **Macro deck** (Quick actions → Deck): big buttons for media keys, the microphone, apps, files, websites, games, key presses (a Discord mute key, OBS scenes, F13–F24), typed text, counters, screenshots and locking the PC. Use them on the PC or on your **phone** (More → Deck), like a stream deck. The phone only sees each button's name, icon and colour.
- **Quick-launch wheel**: press **Alt+Shift+Q** anywhere and a wheel of up to 8 actions opens at your mouse. Point, click or press 1–8; Esc closes it.
- **Clipboard history**: press **Alt+Shift+V** anywhere to see what you copied (text and pictures), search it, and paste any of it. Pin items to keep them after a restart (encrypted). Passwords from the vault and from password managers are never kept, and you can add apps to skip or pause it.
- **Focus sounds** (Focus → Sounds): rain, waves, wind, a fireplace, and brown, pink and white noise, made on the fly and mixed at their own volumes. They keep playing while you use the rest of the app.
- **Batteries** (My PC → Batteries): the PC's, Xbox controllers' and Bluetooth headsets', mice' and controllers' batteries, with today's history, time left, and a notification at 20% and 10%.
- **PC health score** (My PC → Health): drive space, memory, restarts and pending updates, startup apps, temporary files, the Recycle Bin and Downloads, each with a button to fix it.
- **Tidy Downloads** (Tools → Folder rules): one click sorts pictures, videos, music, documents, archives and installers into their own folders inside Downloads, and can send installers older than 30 days to the Recycle Bin.

Also in 0.25.0: the Steam Deck–style game library, emulators, and counter events, goals and the stream overlay.

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
