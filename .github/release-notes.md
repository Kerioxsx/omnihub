## New in this version

**Govee floor lamps follow effects and music, calmer brightness, SteelSeries keyboards, and Save as .mrpack.**

- **Govee H6076 floor lamp** (and other models that don't take a colour per segment): it changed with plain colours but stayed warm during effects, music and screen. *Auto* now streams per segment only to the DreamView kits; every other light gets one colour, which they all follow. A light can still be set to *Per segment* by hand.
- **Less glaring**: the screen's brightness boost starts at 40 instead of 65. If yours was on the old 65, it's brought down once; pick any value in Screen → Brightness boost.
- **SteelSeries keyboards**: OmniHub now spots SteelSeries GG (Prism) setting the keyboard at the same time and says how to hand it over (GG → Engine → your keyboard → turn off Prism), so the colours stop fighting.
- **Save as .mrpack**: mixes have their own *Save as .mrpack* button again, beside Share with friends.

Also in 0.13.3: NeoForge and Forge mixes install in the Minecraft Launcher, PC lighting can follow the Govee lights.

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
