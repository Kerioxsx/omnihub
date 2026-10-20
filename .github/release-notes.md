## New in this version

**Chats: Claude, ChatGPT, Claude Code and Codex in one place, and on your phone.** Everything else stays as it was.

- **Every conversation together.** The new **Chats** page lists your chats with Claude and ChatGPT in OmniHub, and the conversations **Claude Code** and **Codex** keep on your PC, newest first, with search.
- **Keep texting any of them.** Open a conversation and send the next message; the answer streams in as it's written, with the steps it takes ("Ran npm test", "Edited main.rs"). Claude Code and Codex run in the conversation's own project folder with your own sign-in, and you choose how much they may do: plan only, ask first, edit files or auto for Claude Code; read only or edit the project for Codex. They never run with permissions skipped.
- **Start new ones.** A chat with Claude or ChatGPT (with your own Anthropic or OpenAI API key, kept encrypted for your Windows account), or Claude Code or Codex in any project folder.
- **From your phone.** A new **Chats** tab in the phone app: the same list, each conversation full screen like a messaging app, new chats, and Stop. You can turn chats, and running Claude Code and Codex from the phone, off on the PC; from the phone they only start in folders they already work in.
- **Ideas.** Save any message as an idea with one tap. Ideas are notes, so they also show in Notes (and in your Claude folder if ideas go there), and **Ask Claude** starts a chat about one.
- **Usage.** Claude Code: today, this week and this month in tokens and what the same use would cost on the API, the current 5-hour window, and when a limit you reached resets. Codex: how much of your 5-hour and weekly limits is used and when they reset. And what your API keys spent in OmniHub (chats and the mod maker). Links open your plans' own usage pages.

Chats in the ChatGPT and Claude apps live in your accounts online, not on your PC, so they aren't in OmniHub.

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
