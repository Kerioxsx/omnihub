## New in this version

- **Phones connect again on Windows.** The phone companion only listened for IPv6 on Windows, so phones opening `192.168.x.x` saw "site can't be reached". It now takes IPv4 and IPv6, and Phone → Connection check tests the address from the PC itself.
- **Updates from inside the app.** OmniHub checks for new versions and installs them for you (Settings → About). Your settings, pairings and data stay. This is the last time you need to download an installer yourself.
- **Approve fast scans once.** Settings → Storage → "Approve fast scans once" stops the administrator prompt before every fast scan.

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
