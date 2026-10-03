# Screen sharing: what to expect

**Zero delay does not exist.** Every frame is captured, encoded, sent,
decoded and displayed; each step costs milliseconds. What OmniHub aims for
is "feels immediate on a good local network" and honest numbers in the UI
(the viewer shows frames per second, round trip and encode time).

| Path | Needs | Typical glass-to-glass on good Wi-Fi/LAN | Max quality |
|---|---|---|---|
| **Built-in stream** (PC → phone browser) | nothing on the phone | 40–90 ms (round trip shown live) | up to 4K source, adaptive JPEG; 30–60 fps at 1080p on a mid-range CPU |
| **Sunshine + Moonlight** (PC → phone) | Sunshine on the PC, Moonlight app on the phone | 10–40 ms | 4K60 (or more) with NVENC/AMF/QuickSync, HDR |
| **scrcpy** (Android → PC) | scrcpy on the PC, USB debugging or wireless debugging on the phone | 35–70 ms | phone's native resolution, H.264/H.265/AV1, audio (Android 11+) |
| iPhone → PC | — | — | iOS has no public API for low-latency mirroring with control; AirPlay receivers exist but are limited. Not supported in v1. |

## Built-in stream

How it works (`crates/omnihub-core/src/capture/stream.rs`):

1. **Capture** with DXGI Desktop Duplication: the GPU hands over only frames
   that changed, so a static screen costs almost nothing. Falls back to GDI
   capture (remote desktop sessions, some virtual GPUs).
2. **Scale** with SIMD (fast_image_resize) to the preset width and **encode**
   to JPEG with SIMD (jpeg-encoder) straight from BGRA.
3. **Send** over a WebSocket with a 20-byte header (sequence, size, pointer
   position, encode time, timestamp).
4. The phone decodes with `createImageBitmap`, draws on a canvas, draws the
   pointer, then **acknowledges** the frame. The PC keeps at most two frames
   in flight; when the phone or the network is slower than the screen
   changes, intermediate frames are skipped rather than queued — this is
   what keeps latency bounded.
5. Every 0.7 s the quality (and if needed the size) is adjusted from the
   measured round trip; it climbs back when the link is fast.

Presets: Data saver (720p, 30 fps), Balanced (1080p, 60 fps), Sharp (1440p,
60 fps), Maximum (native up to 4K, 60 fps). JPEG at 4K60 is CPU- and
bandwidth-heavy (≈ 60–150 Mbit/s); use Sunshine for that.

**Control** (opt-in): touch (tap = click, long-press = right click, two
fingers = scroll) or trackpad mode, a keyboard field and special keys. Input
is injected with `SendInput`; Windows does not allow a non-elevated app to
control elevated windows (UAC prompts, admin tools).

Why JPEG and not H.264 in the browser? JPEG works in every phone browser over
plain HTTP, decodes in hardware on most phones, has no keyframe/GOP delay and
recovers instantly from a dropped frame. A hardware H.264 path (Media
Foundation encoder + WebCodecs decoder, which needs HTTPS) is the natural
next step; the protocol already carries a message type byte for it.

## Sunshine + Moonlight (best quality PC → phone)

Install Sunshine (https://app.lizardbyte.dev) on the PC and Moonlight
(https://moonlight-stream.org) on the phone; pair them once through
Sunshine's web UI (OmniHub links to `https://localhost:47990` and shows
whether Sunshine is running). Both are GPL-3.0 programs that OmniHub does not
bundle.

## scrcpy (Android → PC)

Install with `winget install --id Genymobile.scrcpy` (OmniHub also finds
scoop/Chocolatey installs, or a path set in Settings). Enable USB debugging
on the phone, plug it in, and launch from the Screen page. Presets:

- **Low latency** — 1920 px, 10 Mbit/s, 60 fps, H.264
- **Quality** — native resolution, 24 Mbit/s, 60 fps, H.265
- **Battery** — 1280 px, 4 Mbit/s, 30 fps

**Wireless:** with the phone on USB, "Switch to Wi-Fi" runs `adb tcpip 5555`
and connects to the phone's Wi-Fi address; on Android 11+ you can instead
pair with the code from *Developer options → Wireless debugging*.

## Both directions at once

Possible, but two encodes and two decodes compete for the GPU/CPU and the
network. Use Balanced (or Data saver) for the built-in stream and the
Battery preset for scrcpy while both run.
