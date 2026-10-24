## New in this version

**Govee lights: real colours on every light, and every light keeps moving.**

- **Colours instead of white** on lights with few LEDs: a light that covers a big part of the screen used to mix all its colours into a pale white. Now, when a part of the screen is colourful, the light shows its main colour. The brightness boost no longer pushes pale colours to white.
- **Every light updates**: *Auto* now gives a colour per segment only to lights Govee's account (your API key) or OmniHub knows can take it, and **one colour** to every other light, which all Govee lights take. Lights on one colour get it worked out from their whole part of the screen.
- **Lights that stopped**: a light that leaves streaming (restarted, or touched in the Govee app) is switched back to it by itself within seconds.
- Each light shows *Live* while OmniHub drives it, whether per segment or one colour.

Also in 0.13.1: lights found behind routers that hide them, a brightness boost, smoother music, an installer fix for a drive that's gone.

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
