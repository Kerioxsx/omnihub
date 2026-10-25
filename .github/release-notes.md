## New in this version

**Roblox shaders like RoShade and Bloxshade, every Roblox flag at hand, and Govee floor lamps in full colour by themselves.**

- **Roblox shaders**: in a Roblox game profile, turn on *Shaders* and pick a look: **Realistic** (shadows in corners, soft bloom), **Vivid**, **Cinematic**, **Soft**, **Sharp** (lightest on frame rate) or **Custom** (what you tune in the game is kept), with a strength slider. OmniHub downloads the official ReShade (the build Roblox allows) and effects from SweetFX and qUINT, puts them in Roblox, and puts them back after each Roblox update when you start Roblox from OmniHub. In the game, **Home** opens ReShade's menu and **End** turns the effects off and on. Another shader tool already in Roblox is left alone.
- **Every Roblox flag**: *All flags* lists every flag Roblox's client has, with the value Roblox uses now, searchable, the ones Roblox still reads from your PC marked. Add any flag, edit its value, and switch each one **on or off** without deleting it. Pasting a JSON block still works.
- **Govee floor lamps on Auto**: lights that ignore streaming but have segments (like the H6076 floor lamp, now known as 14 segments, or any light with its segments set) get a colour per segment through Govee's own segment commands by themselves.

Also in 0.13.5: *Per segment, any light* for Govee lights that ignore streaming.

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
