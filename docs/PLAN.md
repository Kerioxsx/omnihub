# Plan: OmniHub — Windows PC Companion App (Storage, Remote, Vault, Screen Share)

**Thoroughness:** very thorough  
**Status:** Approved — implemented in this folder (see README.md for decisions on the open questions)  
**Date:** 2026-09-30  

## Goal

Build a modern Windows desktop companion application (with phone web/PWA access) that provides ultra-fast full-PC storage analysis (MFT-based, seconds on multi-TB NTFS), installed-app library + screenshots, secure local password/notes vault, file transfer and power control from phone, notes/ideas export to a Claude-watched folder, and best-effort low-latency bidirectional screen sharing (targeting high quality / low latency on LAN; true 0-delay is physically impossible).

## Requirements Summary

### Functional
- **Permissioned full-PC access**: Explicit elevation / consent for admin-level operations (MFT, shutdown, broad FS).
- **Storage management (highest priority)**: Scan large volumes (e.g. 4 TB) in seconds–tens of seconds (not hours). Exact file/folder sizes. Treemap + sorted lists + search + filters. Incremental updates. Safe cleanup suggestions (duplicates, large old files, temp, caches) with user confirmation.
- **App library**: Enumerate installed applications (registry + Start Menu + AppX), icons, install size (via storage scan), launch, screenshots folder integration.
- **Screenshots**: Capture + organized library (timestamped, optional tags).
- **Notes & Ideas**: Rich-text / Markdown notes. Dedicated “Ideas for Claude” that write/export directly into a user-chosen watched folder (plain .md or structured).
- **Password / credential vault**: Local encrypted storage for passwords, emails/usernames, notes. Master password + OS-backed protection (DPAPI). Strongly discourage storing primary Gmail passwords; support app passwords / secure notes.
- **Phone companion**:
  - File transfer (bidirectional, chunked, progress, resume).
  - Remote power: shutdown / restart / sleep / lock (authenticated).
  - Browse PC storage / select files for transfer.
  - View notes, vault items (with unlock), app library summary.
- **Screen share**:
  - PC → Phone and Phone → PC.
  - Target: 60 fps, up to 4K where hardware allows, lowest practical latency on LAN.
  - Acknowledge: true 0 ms is impossible (encode + network + decode). Goal is competitive with Sunshine/Moonlight / scrcpy-class latency (<50–100 ms glass-to-glass on good LAN).
- **Beautiful modern UI**: Dark/light, fluid animations, responsive (desktop + phone web), polished interactions.

### Non-functional
- Performance: MFT scan of large NTFS drives in seconds; UI remains responsive.
- Security: Least privilege by default; explicit elevation; local-only networking by default; strong auth for remote features; no cloud required.
- Reliability: Background service for remote features; graceful degradation when admin rights or GPU encode unavailable.
- Cross-device: Windows host primary; phone via modern browser (PWA) first; optional native later.
- Packaging: Single installable Windows app + system tray / optional Windows Service component.

## Research Findings

### Greenfield — no existing codebase
This is a new project.

### Storage scanning (critical differentiator)
- Traditional recursive `FindFirstFile` / `GetFileSize` is too slow for multi-TB / millions of files.
- Industry standard for speed: read NTFS Master File Table (MFT) directly (same technique as WizTree, Everything, modern WinDirStat 2.x, TreeSize MFT mode). Full inventory of a large NTFS volume typically completes in seconds on SSD, still far faster than recursive on HDD.
- Incremental / change detection: NTFS USN Journal (`FSCTL_READ_USN_JOURNAL`, `FSCTL_ENUM_USN_DATA`). Enables “scan only what changed” after initial baseline.
- Requires administrator privileges + raw volume handle (`\\.\C:` etc.).
- Open-source building blocks (Rust-friendly for Tauri):
  - `mft` crate (safe Rust MFT parser).
  - `ntfs` / `ntfs-core` / `ntfs-reader` crates (MFT + USN support).
  - C# / C++ alternatives exist (NtfsLib, MFTLib, etc.) if interop preferred.
- Visualization patterns: treemap (size-proportional), sorted file/folder lists, extension breakdown, duplicate detection (hash or size+name heuristics), age filters.
- Cache: Persist scan results (SQLite or similar) with volume serial + USN watermark for incremental refresh.

### Remote control & file transfer architecture
- Proven pattern: Windows host runs a lightweight local server (HTTP + WebSocket or gRPC) bound to LAN (or localhost + reverse tunnel). Phone opens a PWA or web page; QR code / mDNS / IP+PIN pairing.
- Examples of similar open projects: mobile-pc-control-server (Electron + Express + Socket.IO), RustDesk, various Flutter “turn off PC” apps, DeskStream.
- File transfer: chunked binary over WebSocket or WebRTC data channels; progress, pause/resume, integrity (hash).
- Power actions: authenticated command → `InitiateSystemShutdownEx`, `SetSuspendState`, lock workstation, etc. Require confirmation + short-lived token.
- Prefer local network only by default; optional authenticated internet access is out-of-scope for v1 or behind explicit advanced setting + strong auth.

### Screen sharing realism
- True zero-latency is impossible (physics + encoding + network).
- Best open/low-latency stacks:
  - PC → client: Sunshine (host) + Moonlight (client) class — DXGI Desktop Duplication + hardware encode (NVENC/AMF/QSV) + UDP/WebRTC. 4K60 possible with strong GPU + good link; latency often ~10–50 ms encode + network.
  - Android → PC: scrcpy (excellent, 35–70 ms typical, high fps, H.264/H.265).
  - iOS → PC: significantly harder (no public low-level equivalent to scrcpy; AirPlay / proprietary limited).
- Bidirectional simultaneous 4K60 is extremely demanding (two encode + two decode pipelines + bandwidth). Plan for high-quality unidirectional primary + lower-spec bidirectional secondary.
- Integration options for v1: embed/adapt open-source components or shell out to Sunshine/scrcpy where licensed appropriately; or implement DXGI + Media Foundation / FFmpeg path for PC→phone and document scrcpy integration for phone→PC (Android first).

### Security (passwords, vault, remote)
- Windows DPAPI (`CryptProtectData` / Electron `safeStorage` / Tauri equivalents) for at-rest protection keyed to the Windows user.
- Additional master password + Argon2id/PBKDF2 for vault encryption is recommended so data is not decryptable by every process under the same user.
- Never store primary Google account passwords if avoidable; educate user about app passwords / passkeys / OAuth.
- Remote features: PIN or short-lived token + optional Windows Hello; bind to local interface by default; rate-limit; audit log of power/file actions.
- Least privilege: normal mode without admin; elevate only for MFT/USN/raw volume and certain power actions.

### UI / stack recommendations
- **Preferred stack for Claude implementation**:
  - **Tauri 2** (Rust core + web frontend): small binary, excellent native access, good security model, web UI flexibility.
  - Frontend: React or Svelte + TypeScript + Tailwind + Framer Motion (or equivalent) for modern animated UI.
  - Data: SQLite (via `tauri-plugin-sql` or rusqlite) for scan cache, notes, vault metadata.
  - Background: Tauri tray + optional Windows Service for always-on remote listener.
- Alternative: Electron (faster ecosystem for some plugins, larger footprint).
- Phone: Progressive Web App first (works on Android & iOS without store). Optional later Flutter native client.
- Animations: micro-interactions, smooth page transitions, treemap zoom, progress rings — keep 60 fps on UI thread.

### Constraints & hard limits
- Admin rights required for fastest MFT path and some power APIs.
- GPU encode quality/latency highly hardware-dependent.
- iOS screen capture & control severely limited compared with Android.
- Storing real email passwords carries account-takeover risk; UI must warn.
- Network file transfer and screen share quality degrade outside high-quality LAN / Wi-Fi 6+.

## Proposed Approach

### High-level architecture
```
┌─────────────────────────────────────────────────────────────┐
│  OmniHub Desktop (Tauri 2)                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │ UI (WebView) │  │ Core (Rust)  │  │ Background       │  │
│  │ React/Svelte │←→│ Commands     │  │ Service / Tray   │  │
│  │ + animations │  │ MFT/USN eng. │  │ Local HTTP/WS    │  │
│  └──────────────┘  │ Vault        │  │ Screen capture   │  │
│                    │ Notes        │  │ Power / Files    │  │
│                    │ App enum     │  └──────────────────┘  │
│                    └──────────────┘                         │
└──────────────────────────┬──────────────────────────────────┘
                           │ LAN (mDNS / QR / PIN)
┌──────────────────────────▼──────────────────────────────────┐
│  Phone PWA / Browser                                        │
│  File browser · Transfer · Power · Notes · Vault (unlock)   │
│  Screen view / control (best-effort)                        │
└─────────────────────────────────────────────────────────────┘
```

### Alternative approaches considered
1. **Electron-first** — Faster prototype, larger RAM/disk. Rejected as primary because Tauri gives better native performance and smaller footprint for a system utility.
2. **Pure native (WinUI / WPF + C#)** — Excellent Windows integration but weaker cross-device web story and slower UI iteration for modern animated design. Possible for storage engine interop only.
3. **Full native mobile apps from day 1** — Higher cost; PWA covers 80 % of phone needs for v1.

**Chosen**: Tauri 2 + modern web frontend + PWA phone client. Storage engine in Rust (MFT/USN crates). Screen-share module isolated so it can start as “launch Sunshine / scrcpy” integration and evolve to deeper embedding.

### Subagent / parallel workstreams (for Claude implementation)
After approval, Claude (or parallel agents) should treat these as concurrent tracks where dependencies allow:

| Subagent / Track | Responsibility |
|------------------|----------------|
| **A. Scaffold & Core** | Tauri project, tray, permissions, SQLite, settings, elevation helper |
| **B. Storage Engine** | MFT parser, USN incremental, cache, treemap data model, cleanup heuristics |
| **C. App Library & Screenshots** | Installed apps enumeration, icons, size join from storage, screenshot capture + library |
| **D. Notes & Claude Folder** | Editor, Markdown, export watcher folder, tags |
| **E. Vault** | Master password, DPAPI + AES-GCM layer, CRUD UI, auto-lock |
| **F. Remote Protocol** | Local server, pairing (QR/PIN), auth tokens, file transfer, power commands |
| **G. Phone PWA** | Responsive UI, file browser, transfer progress, power, vault unlock view |
| **H. Screen Share** | PC→phone path (DXGI/hardware encode or Sunshine bridge), Android→PC (scrcpy integration), latency/quality targets, realistic docs |
| **I. UI/UX Polish** | Design system, animations, dark/light, accessibility, empty/error states |
| **J. Security & Packaging** | Capability audit, installer (NSIS/MSI), code signing notes, least-privilege modes |

## Implementation Steps

### Phase 0 — Foundations (Track A)
1. Initialize Tauri 2 + React (or Svelte) + TypeScript + Tailwind project.
2. Configure tray icon, single-instance, autostart option.
3. Add SQLite, settings store, logging.
4. Implement elevation helper (request admin only when needed for MFT/power).
5. Define capability/permission model (normal vs elevated modes).

### Phase 1 — Storage Engine (Track B) — highest priority
1. Implement raw volume open + MFT enumeration using chosen Rust crate(s) (`mft` / `ntfs-reader` etc.).
2. Build in-memory + SQLite-backed size tree (folder hierarchy + file list).
3. USN Journal baseline + incremental update path.
4. UI: drive selector, progress, treemap (canvas/WebGL or SVG), sortable table, search, filters (size, age, extension).
5. Duplicate / large-old / temp / cache detectors with “suggest only” + explicit delete confirmation.
6. Benchmark target: multi-TB NTFS SSD under 30 s for full inventory on mid-range hardware; document results.

### Phase 2 — App Library, Screenshots, Notes (Tracks C, D)
1. Enumerate installed apps (Registry Uninstall keys, AppX, Start Menu shortcuts); resolve icons and install paths.
2. Join with storage scan for “installed size”.
3. Screenshot capture (primary monitor or region) + library view with folders/tags.
4. Notes editor (Markdown + basic rich text). “Save to Claude Ideas” writes `.md` (or structured) into user-configured folder; optional file watcher / open-in-explorer.

### Phase 3 — Vault (Track E)
1. Master password setup (Argon2id).
2. Encrypt vault payload with key derived from master + DPAPI-protected salt/key material.
3. CRUD for entries (title, username, password, URL, notes, tags).
4. Auto-lock, clipboard clear timeout, Windows Hello optional unlock.
5. Clear UX warnings about primary account passwords.

### Phase 4 — Remote & Phone (Tracks F, G)
1. Local HTTP + WebSocket server (or Axum/Tokio) bound to LAN interfaces; mDNS optional.
2. Pairing: generate QR + short PIN; exchange session token.
3. Authenticated commands: list drives/folders (respect permissions), file transfer (chunked + hash), power actions with confirmation dialog on PC or double-confirm on phone.
4. Phone PWA: installable, responsive, offline shell, real-time transfer progress, power buttons, vault view (requires unlock token).
5. Security: local-only default, rate limits, action audit log, token expiry.

### Phase 5 — Screen Share (Track H)
1. Document realistic targets and hardware requirements.
2. PC → Phone: integrate or bridge Sunshine (preferred for quality) or implement DXGI Desktop Duplication + hardware encode + low-latency transport; serve to phone browser or companion viewer.
3. Phone → PC (Android first): integrate scrcpy (USB or TCP/IP) with UI controls inside OmniHub; document limitations.
4. iOS: best-effort / lower priority (document constraints).
5. Simultaneous bidirectional: lower resolution/fps or sequential modes to keep system usable.
6. UI: start/stop, quality presets, latency indicator, full-screen viewer.

### Phase 6 — Polish, Security, Packaging (Tracks I, J)
1. Design system, motion, dark/light, responsive breakpoints, accessibility (keyboard, contrast).
2. Empty states, error recovery, first-run onboarding (permissions explanation).
3. Security review: no unnecessary privileges, vault hardening, remote auth, no plaintext secrets.
4. Installer (NSIS or MSI), optional Windows Service for headless remote listener, uninstaller cleanliness.
5. End-to-end test plan execution and documentation for Claude (and user).

## File Impact

### New files / structure (high-level)
```
omnihub/
├── src-tauri/                 # Rust core
│   ├── src/
│   │   ├── main.rs
│   │   ├── storage/           # MFT, USN, cache
│   │   ├── vault/
│   │   ├── remote/
│   │   ├── apps/
│   │   ├── capture/           # screenshots + screen share
│   │   └── ...
│   └── Cargo.toml
├── src/                       # Frontend
│   ├── components/
│   ├── pages/                 # Storage, Apps, Notes, Vault, Remote, Settings
│   ├── hooks/
│   └── ...
├── pwa/                       # Phone companion (or shared frontend routes)
├── PLAN.md
└── README.md
```

- No existing project files to modify (greenfield).
- Leave system directories and user data outside app control except via explicit user actions.

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| MFT access fails without admin / on non-NTFS | Core feature degraded | Clear elevation UX; multi-threaded recursive fallback; support FAT/exFAT/network with slower path |
| “0 delay 4K60 both ways” expectation | User disappointment | Explicit docs + UI labels with realistic latency/quality targets; hardware requirements |
| Storing Gmail/primary passwords | Account compromise | Strong warnings, prefer app passwords, optional “secure note only” mode |
| Remote attack surface | Unauthorized shutdown / file access | Local-only default, PIN/token, short TTL, confirmation dialogs, audit log |
| Large scan memory usage | OOM on low-RAM PCs | Streaming parse, disk-backed cache, configurable depth / exclude lists |
| iOS screen share weakness | Incomplete feature | Android-first; document iOS limits; PWA view-only where possible |
| Licensing of third-party screen tools | Legal | Prefer MIT/Apache components or clean integration with user-installed Sunshine/scrcpy |
| Scope creep | Delayed delivery | Strict phase ordering; storage + vault + basic remote first; screen share as later phase |

## Test / Verification Strategy

1. **Storage**: Time full scan of 1–4 TB NTFS volumes; verify sizes against Explorer / WizTree on sample trees; incremental USN correctness after create/modify/delete.
2. **Vault**: Create entries, restart app, unlock, verify decryption; test wrong master password; clipboard timeout.
3. **Remote**: Pair phone on same LAN; transfer multi-GB file with progress/resume; execute shutdown (with cancel window); verify rejected unauthenticated requests.
4. **UI**: Keyboard navigation, dark/light, animation smoothness on mid-range hardware, responsive phone viewport.
5. **Screen share**: Measure glass-to-glass latency on wired/Wi-Fi; confirm 4K60 only when encoder + link support it; Android scrcpy path functional.
6. **Security**: Confirm no plaintext secrets on disk; elevated operations require consent; remote bound to expected interfaces.
7. **Packaging**: Clean install/uninstall; tray + optional service behavior; first-run permission flow.

## Open Questions

1. **Primary phone platform priority?** (Android-first is strongly recommended for screen-share quality; iOS will be limited.)
2. **Accept PWA for phone v1**, or require native apps from day one?
3. **Screen-share strategy preference**: (a) integrate/bridge existing tools (Sunshine + scrcpy) for best quality fastest, or (b) pure in-app implementation even if initially lower performance?
4. **Claude ideas folder**: plain Markdown files only, or also structured JSON / sidecar metadata?
5. **Branding / name**: “OmniHub” used as working title — confirm or replace.
6. Any hard requirement for internet/remote-over-WAN, or LAN-only is acceptable for v1?

---

**Plan ready.** Review `PLAN.md`. Reply **approve** (or describe changes) to proceed to implementation.  
While in plan mode no application source code will be created or modified.