# Development

## Prerequisites

- Rust through rustup (the version is pinned in `rust-toolchain.toml` and
  installed automatically), Node.js 22.
- Windows: Visual Studio Build Tools (C++), WebView2 runtime (preinstalled on Windows 11).
- Linux (core + phone server + UI with mocks only): `ntfs-3g` for the NTFS image tests.

## Everyday commands

```sh
npm ci
npx tauri dev                       # the real app (Windows)
npm run dev                         # desktop UI at http://127.0.0.1:1420 with mock data, any OS
npm run typecheck
npm run build                       # dist/ (desktop) and dist-mobile/ (phone)

cargo test -p omnihub-core          # unit tests + phone server end-to-end tests
cargo clippy --workspace --all-targets -- -D warnings
```

### Phone app against the real server

```sh
cargo run -p omnihub-core --bin omnihub-headless -- --http --pair --dry-run-power
# prints URLs with #pair=SECRET and a PIN
npm run dev:mobile                  # http://<your-ip>:1421, proxies /api to port 47800
```

Debug builds read the embedded phone app from `dist-mobile/` at runtime, so
`npm run build:mobile` is enough after UI changes. `--dry-run-power` logs
power actions instead of performing them. On Linux the screen stream is an
animated test pattern (`OMNIHUB_FAKE_SCREEN=1` forces it on Windows), and
`OMNIHUB_FAKE_MEDIA=1` plays two pretend songs with timed lyrics so the
Music tab works without Windows' media sessions.

`--home DIR` keeps all data in one folder (seed `DIR/data/games.json` to try
the phone's Games screen). The headless server reads commands on stdin:
`send <path>` / `text <words>` (offer to phones), `pair`, `vault-create|
unlock|lock|list <…>`, `vault-add <entry JSON>`, `browser on|off`,
`browser-allow|browser-deny` (answer an extension's pairing request) and
`screen-pause`.

### Browser extension

Load `browser-extension/` unpacked (`brave://extensions` → Developer mode →
Load unpacked); its key in `manifest.json` pins the extension ID the native
host allows. With the app (or the headless server with `browser on`)
running, the extension pairs and fills. The app's executable doubles as the
native-messaging host (`browser_host` is a thin standalone build of it).

### Games

`cargo test -p omnihub-core --test games` runs a whole boost with stand-in
programs (close an app, launch, follow, restore, reopen). The Windows-only
tweaks report "Windows only" elsewhere; on Windows the tests leave the power
plan and other system settings alone. Roblox detection and flag writing are
tested against a temporary folder (`GameHub::set_roblox_roots`).

### Storage engine against NTFS images

```sh
sudo scripts/make-ntfs-fixtures.sh target/ntfs-fixtures
OMNIHUB_NTFS_FIXTURES=$PWD/target/ntfs-fixtures cargo test -p omnihub-core
cargo run -p omnihub-core --example mftdump -- target/ntfs-fixtures/basic.img 0 5   # inspect records
```

The fixtures cover resident and non-resident files, sparse files, alternate
streams, hard links, Unicode names, large directories, a heavily fragmented
`$MFT` (hundreds of runs, attribute list in an extension record) and 512-byte
clusters where every record spans two clusters. The manifest next to each
image comes from walking the mounted file system, and the test compares
every entry.

### Windows-only code

`cargo check --target x86_64-pc-windows-gnu --workspace` type-checks the
Windows code from Linux (install `mingw-w64` for the C dependencies). The
live tests in `crates/omnihub-core/tests/windows_live.rs` scan the real system
drive and need an elevated terminal; CI runs them on `windows-latest`.

## Releasing

1. Bump the version in `package.json`, `Cargo.toml` (`[workspace.package]`)
   and `src-tauri/tauri.conf.json`, run `cargo check` so `Cargo.lock` follows,
   and push to `main`.
2. On GitHub open **Actions → Release → Run workflow** (on `main`). It tags
   that commit `v<version>`. Pushing the tag yourself works too:
   `git tag v0.2.0 && git push origin v0.2.0`.

The `Release` workflow builds the NSIS and MSI installers on Windows and
publishes them with a `SHA256SUMS.txt` on the repository's Releases page,
using `.github/release-notes.md` as the description. It refuses a tag that
does not match the version in `tauri.conf.json`, and a version whose tag
already points to another commit. Running it again for the released commit
rebuilds and replaces the files.

## Adding a command

1. Implement it in `omnihub-core` (with a test).
2. Add a `#[tauri::command]` in `src-tauri/src/commands.rs` and register it in
   `src-tauri/src/lib.rs`.
3. Add the typed wrapper in `src/desktop/api.ts`, any types in
   `src/shared/types.ts`, and a mock in `src/desktop/mock`.

## Events

Core services publish on an in-process bus (`events.rs`); the shell forwards
every event to the windows, the phone server forwards `transfer:*`,
`inbox:*`, `power:*`, `notes:*`, `media:*` and `games:*` to paired phones.
Topics:
`storage:progress|done|failed|deleted|dupes-done`, `screenshots:new|deleted|synced`,
`notes:changed|exported|folder-changed`, `vault:locked|unlocked`,
`remote:status|paired|devices`, `transfer:started|progress|done|cancelled`,
`inbox:new`, `power:pending|cancelled|executed`, `screen:viewers`,
`audit:new`, `settings:changed`, `media:state|lyrics`, `games:session`,
`browser:clients|pair-request|pair-done`, `notes:reminder`; the shell adds `app:navigate` (tray) and
`region:pending` (overlay).
