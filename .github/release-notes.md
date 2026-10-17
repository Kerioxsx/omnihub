## New in this version

More speed for Fortnite and Roblox, and a check for the most common FPS loss on a fast PC. Everything from 0.8.0 stays as it was.

- **Fortnite: NVIDIA Reflex set for you.** Optimize (and every Competitive launch) now sets Fortnite's own *NVIDIA Reflex Low Latency* to **On + Boost** (or On, or Off: your choice), and turns off its *GPU crash debugging*, *latency markers* and *latency flash*, testing aids that cost frames while on.
- **Fortnite: DirectX 12's heaviest features off with Lowest quality.** Nanite geometry, Lumen global illumination and reflections, temporal anti-aliasing / super resolution and landscape detail go to their lowest, for when you play in DirectX 12 rather than Performance mode.
- **Roblox: the graphics slider at its lowest.** Roblox's *Automatic* graphics picks near the top on a fast PC; with *Lowest graphics quality* on, OmniHub sets the slider to 1 in Roblox's own settings file.
- **Roblox: one more Fast Flag from Roblox's allowlist.** *Lighting worked out once* (part of the Max FPS preset) stops Roblox re-lighting the world as things move.
- **Memory at the speed it is sold at.** Optimize PC now says when your memory runs slower than its kit is sold at, for example DDR5-6000 running at 4800 because EXPO or XMP is off in the BIOS. That is often the biggest free gain in processor-bound games like Fortnite and Roblox. OmniHub only reads the speed; the switch is in your BIOS, and the page shows the way there.

Each of these changes only what the game already saved in its settings file; a key a game update renames is left alone. "Put back mine" still restores your own file. Windows security settings (Memory Integrity, Defender) stay untouched.

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
