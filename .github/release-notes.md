## New in this version

**Hide any file under your vault password, and lock any note.**

- **Hidden files**: Vault → **Hidden files**. Drop any file on the window (or **Hide files…**): a zip, videos, documents, however big. OmniHub encrypts it with your vault key and removes it from where it was, so it's gone from Explorer and from search. With the vault unlocked: **Open** a temporary copy (deleted when the vault locks), **Save a copy** somewhere and keep it hidden, **Unhide** it back to its folder (or another), or delete it for good. Big files show their progress and can be stopped. Files on another drive stay on that drive, so a 50 GB zip on D: doesn't fill C:.
- **Locked notes**: the lock button at the top of a note locks it with the same vault password (or Windows Hello). Its text is encrypted; the title stays visible in the list. When the vault locks, the note's text goes away until you unlock it again. Locked notes aren't sent to Claude or shown on the phone.
- How it's protected: every hidden file has its own key, wrapped by your vault key; the contents are encrypted 1 MB at a time (AES-256-GCM), and even file names are encrypted. Changing your master password keeps everything readable. Folders need zipping first; deleting a file the normal way can leave traces on a hard drive, and that applies here too.

Also in 0.17.0: search everything (Ctrl+K), app updates, pin a screenshot on top, Wi-Fi QR codes, imports from other password managers.

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
