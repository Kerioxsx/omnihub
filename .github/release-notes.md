## New in this version

**Search everything, app updates, Wi-Fi QR codes, and imports from other password managers.**

- **Search everything (Ctrl+K)**: type in the command palette and it finds files (from your latest scan of each drive), notes and ideas, apps, screenshots (by their text too) and, while the vault is open, vault entries by title. Enter opens it: a file in Explorer, an app starts, a vault entry opens in the vault.
- **App updates**: Apps → **Updates** lists every installed app with a newer version (through winget, Windows' own package manager), with **Update** per app and **Update all**.
- **Pin a screenshot on top**: in a screenshot, **Pin on top** keeps it in a small window above everything else. Drag it, resize it from its edges, scroll to fade it, double-click or Esc to close.
- **Wi-Fi QR code**: a Wi-Fi entry in the vault has **Show Wi-Fi QR code**: guests point their phone's camera at it and join without typing the password.
- **Import from another password manager**: Vault → settings → **From another password manager…** reads Bitwarden (CSV or unencrypted JSON), Chrome, Edge, Brave, Opera, Firefox, 1Password, KeePass, LastPass or any CSV with name, URL, username and password columns. Logins already in the vault are skipped, two-factor secrets come along, and OmniHub offers to delete the export file afterwards (it has your passwords in plain text).
- **Phone**: *Get the PC's clipboard* on Home copies the PC's clipboard text onto the phone (never a password copied from the vault). On Android, with OmniHub added to the home screen, **Share → OmniHub** from any app sends a link or text to the PC: open it there, copy it there, or save it as an idea.

Also in 0.16.1: a Capri switch at the top of Chats.

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
