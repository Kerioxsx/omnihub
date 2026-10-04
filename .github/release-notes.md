## New in this version

- **The AirPlay receiver starts the way it was tested.** It now uses the video and sound outputs the add-on is tested with on Windows, instead of whatever GStreamer picks on your PC. OmniHub's check that iPhones can find the PC no longer connects to the receiver itself. If the receiver still closes, the iPhone → PC card says how it ended and offers **Start in safe mode** (software video, which avoids graphics-driver crashes).
- **Updates don't close OmniHub on you.** Automatic updates now wait until OmniHub is in the tray and nothing is running (AirPlay included). After an update, OmniHub says "Updated to …" so a restart isn't mistaken for a crash.
- **If OmniHub closes unexpectedly**, the next start says so and offers **Copy details**: the end of OmniHub's log and Windows' own crash summaries. Nothing is sent anywhere.
- A page that hits an error now shows a message with **Try again** instead of a blank window.

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
