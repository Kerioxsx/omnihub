## New in this version

**Govee lights: all of them found, all of them moving, brighter and smoother.**

- **Lights that weren't found**: besides the usual call on the network, OmniHub now also asks every address on your Wi-Fi directly (routers often drop the first kind), and asks twice. Still missing one? Setup lists what to check: LAN Control on for *each* light, the same network (not a guest Wi-Fi), or its address.
- **Lights that stayed still** in music or screen mode (or kept the identify blink): not every Govee light takes a colour per segment. The new **Auto** choice (the default) gives a colour per segment only to lights Govee's account says can take it, and one colour to the rest. If a light still stays on one colour, pick **One colour** for it.
- **Screen colours brighter and truer**: a new **Brightness boost** (on by default) lifts dim colours to light the room without changing them, while black stays black, and colourful parts of the picture count more than grey ones, so a red car on a grey road shows red instead of a muddy brown.
- **Smoother music**, especially on lamps with many LEDs (like the Govee x JBL table lamp speaker): colours glide from one beat's colour to the next and fade more gently. A **Smoothness** slider in Music sets how snappy it is. You can also pick up to 84 segments for a light.
- **Installer fix**: setup no longer stops with *"could not write to D:\OmniHub"* or *"The volume D:\ is currently unavailable"* when OmniHub was once installed on a drive that's gone. The .exe installs to your user folder instead (and offers it if a folder can't be written); the MSI goes back to Program Files.

Also in 0.13: share mixes and modpacks with friends (with the cover and your server), a friend's pack, modpack details, servers from single-player worlds.

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
