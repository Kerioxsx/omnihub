## New in this version

- **iPhone → PC mirroring finds the PC.** On PCs with a VPN, a second network card or virtual adapters, the AirPlay receiver announced itself on the wrong one and iPhones never saw it. It now announces the PC's Wi-Fi/Ethernet address, and the iPhone → PC card checks that iPhones can find it (with what to check if not). OmniHub offers to update the AirPlay receiver once — click **Update receiver**.
- **No more false "in a call".** Calls are now the calling apps (Discord, WhatsApp, Nyxen, Teams, browsers…) recording from your microphone right now. Games with voice chat show as "also using your mic" instead of a call.
- **Watching the PC on an iPhone:** the viewer covers the whole screen (the tab bar no longer sits on top of its buttons), the full-screen button works on iPhone (it hides every control; turn the phone sideways for the biggest picture), and the stream stats appear only when you tap the Live pill.

Updating from 0.2.1 or later: OmniHub installs this by itself (or Settings → About → Install now).

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
