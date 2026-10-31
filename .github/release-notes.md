## New in this version

**Let anyone send you files with a QR code, folder rules that tidy up by themselves, a focus timer with a time tracker, and tools for developers.**

- **Guest uploads** (Phone → Let someone send you files): show a QR code; anyone on your Wi-Fi scans it and sends you photos, videos or files from their browser, without pairing or installing anything. It only receives (they see nothing on your PC), works for 15 minutes to 4 hours and up to the size you pick, and you can end it any time. Files land in Incoming\Guests, and you get a notification for each one.
- **Folder rules** (Tools → Folder rules): “PDFs in Downloads go to Documents\PDFs”, “copy photos to Pictures”, “installers older than 30 days go to the Recycle Bin”. A file is only touched once it has finished downloading, subfolders are left alone, nothing is overwritten, and deletions go to the Recycle Bin.
- **Focus** (new page): a focus timer (25/5, 50/10 or 90/15) that closes the apps you choose (Discord, Steam…) if they open during a round, holds OmniHub's notifications until the break, and tells you when to rest. Plus a **time tracker** (off until you turn it on): how long each program was in front today and this week, only while you're at the PC; program names and minutes only, never window titles, kept on your PC.
- **Projects** (Tools): every Git project in the folders you add, with its branch, uncommitted changes and commits to push or pull; open it in VS Code, a terminal or Explorer.
- **PATH & variables** (Tools): edit your PATH (reorder, add, remove; missing folders crossed out) and your environment variables, the same ones System Properties edits.
- **Dev servers on your phone** (Tools → Ports): the phone button next to a listening port shows a QR code that opens it on your phone, and says so when the server only listens on this PC.

Also in 0.20.0: Aurora's music hub (search every music service from the player), the Tools page, the panic button, and more voice commands.

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
