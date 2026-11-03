## New in this version

**Capri Desktop, nightly quiet mode, overheat alerts, a gaming dashboard on your phone, a party queue, screen-time limits and more.**

- **Capri Desktop** (Games → Clips & Capri): installs the free companion from capriai.app straight from its developer's GitHub releases (checked against GitHub's checksum), opens and closes it, can start it with OmniHub, and gets Ollama ready for its local brain (downloads the model its site suggests). Open or close it from your phone's Deck page or a deck button.
- **Quiet mode on a schedule, and a sleep timer** (My PC → Fans, or the phone's Power page): quiet every night between two times, or in 15 minutes to 1½ hours.
- **Overheat alerts**: a notification on the PC and the phone when the processor or graphics card stays above your limit for 20 seconds.
- **Gaming dashboard on the phone** (More → Gaming dashboard): FPS, 1% lows, temperatures, load and ping, live, so the PC's screen stays clean.
- **Search the PC from the phone**: a search box on the Files page looks through every shared folder by name.
- **Apps update themselves at night** (Apps → Updates): winget installs the updates it finds, one app at a time, skipping the ones you list and never during a game boost.
- **Party queue** (Music → Party queue): friends scan a QR code and add songs from their phones, no app or pairing. Songs play one after another in your music app.
- **Screen-time limits** (Focus → Limits): minutes a day for an app or a website. A warning five minutes before; at the limit the app closes (a website's browser window is minimized).
- **Sensitivity converter** (Games → Extras): the same aim in CS2, VALORANT, Fortnite, Apex, Overwatch 2 and Call of Duty, with eDPI and centimetres per full turn.
- **Fortnite item-shop alerts** (Games → Extras): say what you're waiting for; you're told the day it's in the shop.
- **Import phone photos** (phone: More → Import photos): only photos not already on the PC are sent, into Pictures › Phone, a folder for each month.
- **PC parts price tracker** (Games → Extras): paste a product page; OmniHub checks the price every 6 hours and tells you when it drops or falls under your target.
- **Voice commands from the phone**: the microphone button on the phone's Home page ("start Fortnite", "quiet mode", "clip that", "open Spotify"). The microphone needs the secure (HTTPS) connection; otherwise you type the command.

Also in 0.29.1: game covers load again, and the library has a full-screen Big Picture mode.

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
