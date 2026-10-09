## New in this version

- **Fortnite at max FPS.** Games → Fortnite now sets Fortnite itself for frames: unlimited frame rate (or a cap you pick), Performance rendering mode, every quality setting at its lowest, VSync and motion blur off, fullscreen, and Fortnite's own FPS counter so you can see what you get. It's written into Fortnite's settings file (only while Fortnite is closed), again before every launch if you like, and **Put back mine** restores your own settings.
- **Optimize PC** (Games → Optimize PC) checks what holds games back on this PC and fixes it in one click each — or all at once:
  - monitor at its full refresh rate (Windows often leaves 144/240 Hz screens at 60 Hz),
  - Ultimate Performance power plan,
  - background game recording (Game DVR) off,
  - Game Mode on,
  - optimizations for windowed games (Windows 11),
  - hardware-accelerated GPU scheduling (asks for administrator approval, takes effect after a restart),
  - mouse acceleration off.
  
  It also shows your processor, graphics card, memory and monitor, and links to Memory Integrity in Windows Security. Every change can be undone.
- **Finds your games.** Games installed through Epic, Steam, Riot, Roblox and Minecraft show up under "On this PC", ready to add with one click. Minecraft (Java) gets its own optimizer too: unlimited FPS, fast graphics, minimal particles, no clouds.
- **Full-screen lyrics, Apple Music style.** A new **Music** page, and a full-screen player from Home's Now Playing card or Ctrl K. The cover art is blurred into slowly drifting light, and big lyrics glide up line by line, filling word by word when the lyrics time words. Breaks show three breathing dots, and clicking a line jumps there. Space, the arrows and Esc work, and the controls fade away while you watch.

About frame rates: no app can promise a number like 540 FPS — it tops out where your processor and graphics card do. OmniHub takes away everything else in the way.

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

`SHA256SUMS.txt` lists the SHA-256 checksum of each file.
