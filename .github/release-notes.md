## New in this version

**Your phone becomes a scanner, a barcode reader and a whiteboard, it can ask your phone before sensitive things happen, and new tools for images, videos and downloads.**

- **Scan** (phone: More → Scan): take a photo of a page, receipt or business card; the corners are found (drag them if they're off), the page is straightened and cleaned up, and several pages become one PDF in Documents\OmniHub Scans with the text read by Windows' own text recognition (nothing goes online). Receipts are read into shop, date, total and tax; business cards into name, phone, email and website, plus a contact file (.vcf). Or save it all as a note.
- **Barcode to PC** (phone: More): scan a barcode or QR code and type it where the PC's cursor is (with Enter, for inventory sheets), copy it, or open its link. Needs “let phones use the mouse and keyboard”, and a browser that reads barcodes (Chrome on Android); you can also type or paste any text to type on the PC.
- **Whiteboard** (phone: More; PC: Tools → Whiteboard): draw on the phone and each stroke shows up on the PC, where you can save it as a picture.
- **Approve on your phone** (Settings → Privacy & security, off until you turn it on): before the PC shows or copies a vault password, restarts OmniHub as administrator, shreds files or changes PATH, your paired phone asks and the PC waits up to a minute for Allow. Turning it off needs the phone too. It slows down someone at your PC; it isn't a lock against someone with your Windows account and time.
- **Download on the PC** (phone: More, or Share → OmniHub; PC: Tools → Downloads): send a link and the PC downloads it into Downloads\OmniHub: files as they are, and the video (or only the sound) from YouTube, TikTok, X or Instagram pages through yt-dlp.
- **Images** (Tools): take location, camera and date details out of photos before sharing (they stay upright), resize, compress, and turn iPhone HEIC photos into JPG. New files go next to the originals.
- **Video** (Tools; and on the phone for any video in Files): fit a clip in 10 MB for Discord (it works out the quality, and a smaller picture if it must), compress with your graphics card's encoder, convert to MP4 (MKV and MOV from OBS are just repackaged), WebM or GIF, cut a part out without re-encoding, or keep only the sound as MP3, M4A or WAV. Uses the FFmpeg instant replay downloads.

Also in 0.21.0: guest uploads by QR code, folder rules, the focus timer and time tracker, and developer tools.

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
