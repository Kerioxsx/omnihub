## New in this version

**Forgot your vault password? Now there's a way back in. Plus a Minecraft theme and six more animated themes.**

- **Forgot your master password?** (on the vault's lock screen, and “Forgot it?” in Change master password)
  - **With Windows Hello turned on for the vault:** confirm with your face, fingerprint or PIN and choose a new password. Everything stays: logins, hidden files, locked notes. If the vault is open but Hello isn't on yet, it can be turned on right there first.
  - **Without Windows Hello:** nothing can open the vault without its password (that's what keeps it safe). You can **set it aside and start a new vault**: the old one is kept on your PC, not deleted, and **Put back** (on the Vault page or in Vault settings) returns it with everything in it if you remember the password later.
  - If you approve on your phone, setting a new password asks your phone too.
- **Minecraft theme**: blocky hills and clouds, a sky that follows your PC's clock (day, sunset, night with stars and a square moon), the Minecraft font on headings, and **your own item pack** floating and spinning like dropped items (Settings → Appearance → Item pack: a zip of item pictures, kept on your PC). Without a pack, XP orbs float up instead.
- **Six more animated themes**: Matrix (code rain), Rain (with far-off lightning), Snowfall, Fireflies, Lava lamp, and Clouds (a light theme).

Also in 0.22.1: change your master password, and the Aurora, Synthwave, Ocean, Galaxy, Sakura and Ember themes.

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
