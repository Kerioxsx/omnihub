# Security model

OmniHub touches your whole disk, your passwords and your PC's power button,
and lets a phone do some of that. This is what protects each part.

## Privileges

- The app runs as a **standard user**. Nothing in normal use needs
  administrator rights: power actions use the interactive user's shutdown
  privilege; screenshots, apps, notes and the vault are per-user.
- **Fast scan** (MFT) needs a raw volume handle. Each fast scan asks through
  UAC and runs a short-lived helper (`--omnihub-helper scan-volume`) that
  only reads the volume and writes a snapshot. Its arguments are validated
  (single drive letter, output folder must be OmniHub's scan cache, no
  junctions or symlinks on the path) so another program cannot use the
  helper to write elsewhere with admin rights. The snapshot holds file names
  and sizes only.
- "Restart as administrator" exists for people who prefer one prompt per
  session; drag-and-drop from Explorer does not work into an elevated app.

## Vault

- Entries are serialised and encrypted with **AES-256-GCM** under a random
  256-bit vault key. The vault key is wrapped with a key derived from the
  master password by **Argon2id** (64 MiB, 3 passes, 4 lanes). Both
  ciphertexts authenticate the format header.
- On Windows the whole file is additionally wrapped with **DPAPI** (current
  user), so a copied `vault.ohv` cannot even be attacked offline without the
  Windows account's credentials. Encrypted backups (export) omit DPAPI so
  they can be restored on another PC with the master password.
- Decrypted entries exist only in memory while unlocked; secret strings are
  zeroed on drop. The vault locks after idle time (default 5 minutes), when
  Windows locks, or by hand. Wrong passwords are throttled (exponential
  back-off after three failures).
- **Clipboard:** copied secrets are marked to stay out of Windows clipboard
  history and cloud clipboard, and cleared after 20 s if still on the
  clipboard.
- **Windows Hello** (optional): a Hello key pair signs a fixed random
  challenge; the SHA-256 of that deterministic signature wraps the vault key
  (`vault.hello`, also DPAPI-wrapped). Unwrapping needs a visible Hello
  prompt, unlike a plain DPAPI copy of the key, which any program running as
  you could decrypt silently. The master password always works.
- **Primary accounts:** the UI warns when an entry looks like a Google,
  Microsoft, Apple, Yahoo or Proton account and recommends passkeys, app
  passwords and 2-step verification instead of storing that password.

## Phone companion

- **Off by default.** When on, it listens on the LAN (or only on localhost)
  and **rejects every peer that is not a private address** (RFC 1918,
  link-local, loopback, IPv6 ULA; Tailscale's 100.64.0.0/10 only if you
  allow it), regardless of firewall settings.
- **DNS rebinding:** requests must carry an IP literal, `localhost` or this
  PC's own name in the `Host` header.
- **HTTPS** by default with a certificate generated on this PC (its SHA-256
  fingerprint is shown in the app so you can compare it with what the phone
  shows). Plain HTTP is a setting for networks you trust; vault access is
  refused over HTTP.
- **Pairing** needs someone at the PC: the desktop opens a 5-minute window
  with a QR code (128-bit secret) and a 6-digit PIN; one device per window,
  five wrong PINs close it, attempts are rate-limited per address. Pairing
  returns a 256-bit device token; only its SHA-256 is stored. Devices can be
  renamed and revoked; revocation also ends their sockets and vault
  sessions.
- **Least privilege for files:** by default the phone sees only Desktop,
  Documents, Downloads, Pictures, Videos, Music and the "From phone" folder.
  Every path is canonicalised and must stay inside a shared root (no `..`,
  no following junctions out). Download links and socket URLs carry
  short-lived tickets, never the token.
- **Power** actions that end the session need an explicit confirmation from
  the phone and run after a countdown (default 10 s) shown on the PC with a
  Cancel button.
- **Remote control** (mouse/keyboard) is a separate opt-in, off by default;
  the PC shows who is viewing and controlling and can stop it. Windows does
  not let a standard-user app inject input into elevated windows.
- **Vault on the phone** is a separate opt-in, HTTPS only, needs the master
  password on the phone each time, and gives that phone its own decrypted
  copy for 5 idle minutes (the PC's vault stays locked). Listing never
  includes secrets; every reveal is logged.
- **Audit log:** pairing, revocation, downloads, uploads, power requests and
  cancellations, app launches, screen sharing and control, vault unlocks and
  reveals, deletions from the desktop. Kept 180 days.
- Content-Security-Policy, `X-Frame-Options: DENY`, `nosniff` and
  `no-referrer` on every response.

## Deleting files

Cleanup only suggests. Deletion goes to the Recycle Bin unless you choose
"Delete permanently" and type a confirmation. Windows, Program Files,
ProgramData\Microsoft, drive roots, system files at the root and your user
folder itself are refused outright; Windows temp, update download cache,
crash dumps and delivery optimisation cache are the only allowed system
locations. Names that could not be decoded exactly are never passed back to
the OS.

## Not in v1

No cloud relay or internet access. No Windows service (a service would run
the phone server as SYSTEM; instead the app starts at login, minimised to
the tray). The installer is not code-signed yet, so SmartScreen will warn.
