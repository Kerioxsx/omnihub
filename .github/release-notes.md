## New in this version

**A colour per segment on Govee lights that ignore streaming.**

- **Per segment, any light**: a new choice on each light (Lights → the light → Live colours). Lights like the H6076 floor lamp ignore Govee's fast streaming, so until now they could only show one colour in screen, music and effect modes. This choice sends Govee's own segment commands instead (what its Bluetooth app sends), about 8 times a second, only for the segments that changed: up to 15 segments, each with its own colour. Set the number of segments your light has under *Placement, segments*.
- *Auto* stays as it was: streaming for the lights that take it, one colour for the rest.

Also in 0.13.4: calmer brightness boost, SteelSeries GG spotted, Save as .mrpack.

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
