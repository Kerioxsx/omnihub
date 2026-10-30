## New in this version

**Capri, on and off with one switch.**

- **Capri** ([mochiiwuff/capri-desktop](https://github.com/mochiiwuff/capri-desktop)), the desktop companion who narrates what Claude Code, Codex or Antigravity are doing: a **Capri** switch at the top of **Chats** (and in the tray menu). The first time you switch her on, OmniHub downloads her official release from her GitHub page, checks it against GitHub's checksum and installs it for your user (about 100 MB, no administrator); then she starts, and starts with OmniHub until you switch her off. Off closes her and takes her out of Windows' startup list. When her page has a newer version, an update button shows next to the switch. Capri is mochiiwuff's free, non-commercial project; OmniHub doesn't include her, it fetches her for you.

Also in 0.16.0: your monitors' brightness from your phone (More → Volume & brightness) and on the PC (My PC → Monitors).

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
