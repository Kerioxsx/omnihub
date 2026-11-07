## New in this version

**Big Picture works with every kind of controller.**

- **Every controller is recognised**: Xbox (including Elite), PlayStation 4, PlayStation 5 and DualSense Edge, Switch Pro, Joy-Con (single or a pair), Switch 2 Pro and Joy-Con 2, GameCube and Switch Online controllers, Steam controllers, and any other gamepad Windows sees (8BitDo, PowerA, HORI, Logitech and more), even ones without a standard layout.
- **The right button symbols**: Big Picture shows A/B/Y on Xbox controllers, ✕/○/△ on PlayStation ones and the Nintendo letters on Nintendo ones, for whichever controller you last pressed.
- **Nintendo layout**: on Nintendo controllers, A (the right button) confirms and B goes back, like on a Switch (can be turned off).
- **Vibration**: a light buzz when you move and a stronger one when you pick a game, at the strength you choose (off, light, medium, strong), on controllers that report vibration to Windows apps.
- **PlayStation light bar colour**: DualShock 4, DualSense and DualSense Edge light up in your theme's colour or one you pick. Over a cable by default; Bluetooth is optional, because over Bluetooth it puts the controller in the mode Steam uses and some older games then don't see it until it's turned off and on.
- **Controllers panel** (Y / △ / X, View, or the button at the top of Big Picture): the controllers connected with a live button test, a vibration test, stick dead zone, vibration strength, light colour and navigation sounds, all usable with the controller.
- The shoulder buttons jump five games at a time. A button held while a screen opens no longer presses something on the new screen.
- Honest limits: Switch 2 controllers' HD rumble has no public way in for Windows apps, and Elite paddles work as whatever buttons the Xbox Accessories app gives them.

Also in 0.30.0: Capri Desktop, nightly quiet mode, overheat alerts, a gaming dashboard on your phone, a party queue, screen-time limits and more.

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

Instant replay uses FFmpeg {{FFMPEG}} (`OmniHub-ffmpeg-x64.exe`, LGPL; license and source links in
`OmniHub-ffmpeg-LICENSE.txt`). OmniHub fetches and checks it when you turn on instant replay.

The FPS meter uses Intel PresentMon {{PRESENTMON}} (`OmniHub-PresentMon-x64.exe`, MIT license in
`OmniHub-PresentMon-LICENSE.txt`). You don't need to download it either: OmniHub fetches and checks
it when you turn on the FPS meter in Games.

`SHA256SUMS.txt` lists the SHA-256 checksum of each file.
