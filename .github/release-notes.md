## New in this version

- **Fixed "Error writing to file … omnihub.exe" when installing or updating** (Windows Installer error 1310). The file was still in use: OmniHub only hid in the tray when the installer asked it to close, and Brave, Chrome or Edge kept a hidden OmniHub helper running for the browser extension. Now:
  - both installers close every running OmniHub, including that helper, before they copy anything;
  - OmniHub closes when Windows or an installer asks;
  - the browser extension's helper runs from a copy in OmniHub's data folder, so it never locks the installed program.
- If OmniHub is in *Program Files*, the setup asks for administrator approval once instead of failing (new in 0.2.5).

Updating from 0.2.1 or later: OmniHub installs this by itself (or Settings → About → Install now), or download the setup below and run it. If an install from an older version still stops with that error: quit OmniHub from the tray, close your browser, end any `omnihub.exe` in Task Manager → Details, and run it again. The .msi always updates OmniHub in the folder it is already in.

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
