## New in this version

- **Volume & calls on your phone.** Every app's volume and mute (like Windows' volume mixer), the PC's volume, and a button that mutes your microphone in every app. When Discord, WhatsApp, Nyxen, Teams or a browser is using the microphone, Home shows the call with **Mute mic** and **Deafen**.
- **Close any app from your phone** (More → Open apps): **Close** works like clicking ×, **Quit** ends apps that only hide in the tray.
- **Music sideways.** Turn the iPhone and the Music screen and full-screen lyrics switch to a side-by-side layout. Full-screen lyrics now have play/pause, skip and volume too.
- The vault's "primary account" warning only shows for the provider's own login (not for any entry with a Gmail address).

Updating from 0.2.1: OmniHub installs this by itself (or Settings → About → Install now).

## Install

1. Download **`OmniHub_{{VERSION}}_x64-setup.exe`** below.
2. Run it. It installs for your Windows user only, so no administrator rights
   are needed. Windows 10 or 11; WebView2 is installed automatically if it is
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

`SHA256SUMS.txt` lists the SHA-256 checksum of each file.
