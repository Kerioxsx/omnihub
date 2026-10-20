## New in this version

**Lights: control every Govee light OmniHub finds.** Everything else stays as it was.

- **Every light on your network.** Govee lights with *LAN Control* on (Govee Home app → the light → settings) are found by themselves, with no account and no cloud. Switch them on and off, set the brightness, a colour or a white from warm to cool, for all of them at once or one by one, and give each its own name. Tap the eye to make a light blink so you know which one it is.
- **They follow your screen, live.** Each light takes its colours from its place around the screen (all around for a TV or monitor backlight, a side, the top, across) up to 30 times a second, eased so changes glide instead of flicker. Instant for games, very smooth for films. Black bars don't dim them, and a vividness slider makes colours pop. The graphics card shrinks the screen first, so it costs almost nothing, and nothing is saved.
- **They dance to your music.** Whatever this PC plays, in any app: the lights take the album cover's colours and pulse on the beats (*Pulse*), show the sound from bass to treble along the light (*Spectrum*), or flow at the song's tempo with flashes on the beat (*Flow*), with a sensitivity slider.
- **Effects and presets.** Rainbow wave, spectrum, flow and breathing in your colours at your speed, plus one-click presets: Movie, Gaming, Party, Spectrum, Flow, Rainbow, Chill, Ocean, Focus, Reading and Night light. Save your own too.
- **Timers.** Wake up to the lights fading in, have them fade out at bedtime, or switch to a preset or a brightness on chosen days. There's also a sleep timer (15–90 minutes, fading out over the last minute).
- **With your Govee API key (optional):** each light's own scenes, DIY scenes and snapshots, its own microphone music modes, and its own settings such as gradient, night light and, on lights that have a speaker, the volume. OmniHub shows whatever each light reports. Paste the key under Lights → Setup (get it in the Govee Home app: Profile → Settings → Apply for API Key). It's kept encrypted for your Windows account and only ever sent to Govee.

**Privacy:** live colours go straight from this PC to the lights on your network (UDP 4001–4003). The Govee cloud is only used with your key, and only for the light's own scenes and settings. If Windows Firewall blocks the lights' answers, Lights → Setup shows it and can add the rule (it asks for administrator approval once).

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
