## New in this version

- **Play and Boost only work again.** A boost waited for Windows' administrator prompt before starting the game, and that prompt often only blinked in the taskbar, so nothing seemed to happen. The prompt now opens in front of OmniHub, and the game starts within 25 seconds even if it isn't answered. Game links (Epic, Steam…) open more reliably, a missing launcher is named, and a boost that hits an error puts everything back instead of leaving the buttons stuck.
- **More for Fortnite (and every game in Competitive mode): the game's NVIDIA settings.** OmniHub sets the game's profile in the NVIDIA driver, the same place as the NVIDIA Control Panel's *Program settings*: prefer maximum performance, low latency mode on, high-performance texture filtering, VSync not forced, and no driver frame cap. A global *Max Frame Rate* there is a common, forgotten FPS ceiling. Anything you set for the game yourself stays. It needs administrator approval once and can be turned off (the settings go back to NVIDIA's defaults).
- **My PC: your PC in 3D.** A new page draws your PC from what Windows reports: processor, graphics card, each memory stick, motherboard size, cooler and fan hubs. Nothing is installed to read them. You fill in what it can't see (case colour, fans, glass, radiator), turn it with the mouse, and the lights, fans and screens in it move live.
- **Lighting.** Colours and effects for the motherboard, memory, graphics card, cooler, fans and strips, through the free OpenRGB (ASUS, MSI, Gigabyte, Corsair, NZXT, Lian Li, Razer and more). Effects: static, breathing, spectrum, rainbow wave, flow, music (pulses with what plays) and temperature, plus your own profiles. Keyboard, mouse and headset are left to their own apps unless you include them.
- **Fan profiles: Silent, Default (Auto) and Fast.** Motherboard and graphics-card fans go through LibreHardwareMonitor, and liquid coolers and fan hubs through liquidctl. No fan goes under 30% (pumps 50%), everything runs flat out from 90 °C, and when OmniHub closes every fan goes back to the hardware's own control.
- **Screens inside the PC.** A case screen that Windows sees as a monitor shows your picture, GIF or video, live PC stats, or the song playing. An NZXT Kraken's LCD shows a picture, a GIF or the liquid temperature.
- **Lighter.** In the tray, OmniHub asks its web view to drop caches and pauses the window's timers. It checks the music player less often when nothing plays. Status dots pulse a few times instead of forever, and the 3D view draws at full refresh rate only while you look at it.

**What OmniHub talks to for My PC:** only programs on this PC. OpenRGB on its local port, LibreHardwareMonitor on 127.0.0.1 (OmniHub refuses any other address), and liquidctl, run as a program. OmniHub loads no driver of its own and never touches Windows' security settings (Memory Integrity, Defender and the like stay as they are).

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
