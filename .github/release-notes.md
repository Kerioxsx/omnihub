## New in this version

**Install NeoForge and Forge mixes in one click, Govee colours that pop, and your keyboard in sync.**

- **Install any mix**: NeoForge and Forge mixes now have a working **Install in the Minecraft Launcher** button. OmniHub runs NeoForge's or Forge's own installer for you (about a minute, with the Java it needs, downloaded once), then adds the mix as its own profile with its picture.
- **Govee lamp blinking off**: a lamp that kept switching off for a moment and back on now stays steady.
- **More saturated colours**: pale screen colours are made vivid before they reach the lights, so lights no longer look white. In music, lights never dim so far between beats that they look switched off.
- **What each light gets**: each light's badge says *Live · per segment* or *Live · one colour*, so you can tell at once which setting a light needs.
- **Your keyboard and PC lights follow the Govee lights**: in My PC → Lighting, pick **Like the Govee lights** and your keyboard, PC and strips show what the Govee lights show (the screen, the music or their effect).
- **No more keyboard lag or flickering colours**: keyboards, mice and headsets get colours at a pace they keep up with, and OmniHub tells you when another lighting program (SignalRGB, iCUE, G HUB, Razer Synapse, Armoury Crate…) is fighting over the same lights.

Also in 0.13.2: real colours on lights with few LEDs, every light updating, lights found behind routers that hide them.

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
