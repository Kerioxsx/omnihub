## New in this version

**Counters with keys that work from anywhere, shiny hunting, an auto clicker, a Pokémon save editor, and a glass look.**

- **Counters** (new page): count anything (deaths, resets, push-ups) with a click or a **key that works from anywhere**, even with a game or emulator in front (the game still gets the key; holding it counts once). Add your own picture, a step, a key to take away, and a **timer**. Keep as many counters as you like; everything stays on your PC.
- **Pop-out**: any counter in its own small window that stays on top of your game, with the count, the picture and the timer. It updates as you press your key.
- **Shiny hunting for any Pokémon** (all 1025): pick the Pokémon, the game and the method; the odds are worked out for you (Masuda method, Shiny Charm, or your method's own odds), with the chance you'd have found it by now, how far past the odds you are, and encounters per hour. **Found it!** keeps the find in your history (count, time, odds) and starts again.
- **Auto clicker** (Tools → Auto clicker): any button, single or double clicks, every 0.01 ms up to an hour, a random part so it looks less robotic, at the mouse or at a point you pick, stop after a number of clicks or seconds. F6 (or your key) starts and stops it from anywhere; **Esc** or pushing the mouse into the top-left corner always stops it. Windows can't really click every 0.01 ms: it goes as fast as your PC allows and shows the real speed. Some online games ban auto clickers.
- **Pokémon save editor** (Tools → Pokémon saves) for the Game Boy Advance games: Ruby, Sapphire, Emerald, FireRed and LeafGreen. Change the trainer (name, IDs, money, play time) and any Pokémon in the party or the 14 PC boxes: species, nickname, level, nature, shiny, ability, item, ball, friendship, moves and PP Ups, IVs and EVs, and its original trainer. Add Pokémon to empty places, delete them, and bring in or save single Pokémon as .pk3 files. Checksums are fixed on save, and a copy of the old save is kept next to it each time. Close the game in your emulator first.
- **Glass** (Settings → Appearance): frosted, see-through cards and sidebar over any theme.

Also in 0.23.0: getting back into a vault whose password you forgot, the Minecraft theme and six more animated themes.

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
