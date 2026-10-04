## New in this version

- **Fixed "Error writing to file … omnihub.exe" when installing.** If your OmniHub is in *Program Files* (or another folder only an administrator can change), the setup now explains that and asks for administrator approval once, instead of stopping with an error. **Install now** in OmniHub does the same; automatic updates leave those installs to your click, so no Windows prompt pops up unasked.
- Moving to a folder that never needs approval: uninstall OmniHub in Windows Settings → Apps (your notes, vault and settings are kept), then run the setup and keep the folder it suggests.

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

`SHA256SUMS.txt` lists the SHA-256 checksum of each file.
