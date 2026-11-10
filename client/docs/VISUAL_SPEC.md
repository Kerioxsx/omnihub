# Vesper Client — Visual Specification (art direction for implementation)

Owner: visual-director. Audience: engineers writing Forge 1.8.9 / LWJGL2 code (GL11 immediate mode, `Tessellator`/`WorldRenderer`, `GlStateManager`).
Sources: `references/ANALYSIS.md`, `docs/FEATURE_MATRIX.md`, reference videos A (~18.4 s) and B (~22.0 s), curated frames `A_*.jpg`, `B_*.jpg`, plus extra frames pulled for this spec (splash 0–4.8 s, A 4–10 s).

## 0. Ground rules

- **Footage caveat.** Both references are portrait phone recordings (512×910) of a monitor, at an angle, with TikTok overlays, moiré and auto-exposure. Hex values below are *target* values (corrected for the phone's crushed blacks and boosted saturation), not pixel samples. Anything marked **[U]** is uncertain; treat it as a design decision, not a measured fact.
- **Originality.** Do not reproduce the reference logo (a hexagram), the "Larp" name, its textures, models, coin logos or sounds. Everything here is procedural geometry or runtime-generated textures (`DynamicTexture`) that we own.
- **Effects are not proof of kills.** Nothing in the footage shows a confirmed kill trigger. Kill effects (section 7) are our own designs, *inspired by* showcase shots.
- **Units.**
  - Screen: `H` = framebuffer height in pixels, `W` = width. GUI code should convert: `px = frac * H` (use `mc.displayHeight`, then divide by `ScaledResolution.getScaleFactor()` for GUI coordinates).
  - World: 1 block = 1.0. Player is 0.6 wide and 1.8 tall; head top at y+1.8, shoulders at y+1.4, waist at y+0.75.
  - Time: ms. `t` = normalized phase time 0..1.
- **Easing names** (used throughout):
  - `easeOutCubic(t) = 1-(1-t)^3`
  - `easeInOutCubic(t) = t<.5 ? 4t^3 : 1-(-2t+2)^3/2`
  - `easeOutBack(t) = 1 + 2.70158(t-1)^3 + 1.70158(t-1)^2`
  - `easeInQuad(t) = t^2`
  - `pulse(p) = 0.5 - 0.5*cos(2π·now/p)`
- **Shared glow sprite.** At startup, generate one 64×64 `DynamicTexture` called `vesper:glow`, with `alpha = (1 - r)^2.2` (r = distance from centre / 32, clamped) and RGB white. Generate a second one, `vesper:twinkle`: a four-point star made of two crossed thin falloffs, `alpha = max(e^(-|x|·18)·(1-|y|), e^(-|y|·18)·(1-|x|))`. All glows below are camera-facing quads using one of these, tinted by vertex colour.
- **Standard additive state.** Push this before any glow and restore it afterwards:
  ```
  disableLighting; enableBlend; blendFunc(GL_SRC_ALPHA, GL_ONE); depthMask(false);
  disableCull; alphaFunc(GL_GREATER, 0.003f); shadeModel(GL_SMOOTH)
  ```
  Restore afterwards: `blendFunc(SRC_ALPHA, ONE_MINUS_SRC_ALPHA)`, `depthMask(true)`, `enableCull`, `alphaFunc(GREATER, 0.1)`.
  Solid geometry (wing ribs, crown, cart) uses normal alpha blending with depth writes on.
- **Bloom substitute.** Real bloom is not available on fixed-function GL. Fake it by drawing every bright element twice:
  1. A core pass at 100% size and full alpha.
  2. A halo pass with `vesper:glow` at 2.5–3× size and 0.25–0.35 alpha.

---

## 1. Splash screen ("Press to enter")

**Reference:** A 0.0–1.6 s, B 0.0–1.6 s.
**What is seen:**
- A flat dark charcoal screen with a faint diagonal sheen.
- A white monoline logo, with the client name in bold sans directly below it and a smaller "Press to enter" line below that.
- B 0.0→1.2 s: the logo drifts slightly (camera shake, partly).
- B ~1.2–1.6 s: the subtitle fades out, the name fades, and the logo shrinks and travels to the top-left of the menu column.
- A ~1.0 s: the logo has a brief bright flash/sparkle **[U: may be monitor glare]**.
- A tiny white glint dot is seen drifting near the top-right in B 0.4–1.6 s **[U]**.

### Layout (16:9, scales with H)

| Element | Anchor | Size | Colour / alpha |
|---|---|---|---|
| Background | full screen | — | linear gradient, 135° (top-left → bottom-right): `#2B2D33` → `#1C1D22` (55%) → `#111215` |
| Sheen | full screen | a band 0.5H wide, rotated -20° | `#FFFFFF`, alpha 0.035, slides left→right once per 9 s |
| Vignette | full screen | radial, inner radius 0.45·max(W,H), outer 0.85 | `#000000`, alpha 0 → 0.55 |
| Sigil logo (section 3) | centre (0.5W, 0.40H) | height 0.16H | `#F2F3F5`, line width `max(1.5, 0.006H)` px |
| Name "VESPER" (or "Vesper Client") | centre-x, baseline 0.55H | cap height 0.032H, bold (MC font with 1px shadow off, drawn twice offset +0.5px for weight) | `#F2F3F5` |
| "Press to enter" | centre-x, 0.615H | cap height 0.018H | `#C9CCD3`, alpha pulses 0.45 ↔ 0.95, period 1800 ms, sine |

### Entrance (0 ms = screen opened)

| ms | Element | Motion |
|---|---|---|
| 0–400 | Background | fades from black, alpha 0→1, linear |
| 150–750 | Logo | alpha 0→1; scale 0.86→1.0 (`easeOutBack`); line "draw-on": each segment's visible length 0→100%, staggered 40 ms per segment |
| 650–900 | Sparkle | one `vesper:twinkle` sprite on the sigil's top tip. Size 0→0.09H→0, rotate 0→45°, colour `#FFFFFF`, additive |
| 500–950 | Name | alpha 0→1, y offset +0.012H→0, `easeOutCubic` |
| 900–1300 | Prompt | alpha 0→0.95, then starts its pulse |
| idle | Drift motes | 12 white points (1–2 px, alpha 0.15–0.5) drift upward at 0.004H/s, wrapping at the top; random x |

### Exit (any key or click; Esc never traps; 800 ms total)

| ms | Element | Motion |
|---|---|---|
| 0–250 | Prompt | alpha → 0 |
| 0–350 | Name | alpha → 0 |
| 0–700 | Logo | moves to the menu header slot (section 2) and scales 0.16H → 0.045H, `easeInOutCubic` |
| 350–800 | Menu items | fade and slide in (section 2) |

Do not destroy and re-create the logo between the two screens. Keep one shared transform so the hand-off is seamless (B 1.2–2.0 s shows one continuous move).

---

## 2. Main title menu

**Reference:** A 1.6–3.8 s (frame A_02.8), B 1.8–4.6 s (frames B_02.9, B_04.2).
**What is seen:**
- The same charcoal background.
- A left nav column with header "[logo] Client name", then small white items. B also shows "Minecraft Realms"; **we omit Realms**.
- On the right, 4–5 tall vertical **wavy curtain bands**: light grey, soft-edged, with darker troughs between them. They look like a satin curtain lit from the left.
- Small 5-point star outlines are scattered along the right edge. We use 4-point stars, which are original.
- A full-height player model stands in front of the curtain (A 3.0–3.7 s, B 3.0–4.2 s). In B it wears the blue comet cosmetic.
- B ~3.6 s: "Singleplayer" has a blue highlight box **[U: possibly an OS text selection, not intended hover]**.
- In A the column looks centre-aligned; in B it is left-aligned. **Decision: left-aligned.**

### Background

- Same as the splash, but the gradient goes `#2A2C31` (top-left) → `#18191D` (centre) → `#0F1013` (bottom-right).
- Add a vignette.

### Curtain arcs (right side)

- **Count:** 5 bands.
- **Position:** they occupy x ∈ [0.70W, 1.04W], overflowing past the right edge, at full height.
- **Geometry:** each band is a vertical triangle strip of 48 rows. Rows step by H/47; each row has 3 vertices across: left edge, crest, right edge.
- **Band centre-line** for band i (0..4):
  ```
  x_i(y,τ) = W·(0.74 + 0.065·i)
           + A·sin(2π·(y/H)/λ + φ_i + τ·ω)
           + 0.4·A·sin(2π·(y/H)/(0.37) + 1.7·φ_i - τ·ω·0.6)
  A = 0.022W·(1 + 0.25·i), λ = 0.62, φ_i = 0.9·i, ω = 2π/14 s
  ```
  Each band drifts very slowly (a 14 s cycle) **[U: the reference is nearly static; the motion is our addition]**.
- **Band width:** 0.05W.
- **Shading:** a cross-section gradient across the band (left edge → crest at 35% → right edge):
  - colours `#7E828C` → `#D9DBE0` → `#55585F`
  - alpha 0 → 0.55 → 0
  - Render back to front (i = 4 first), so the left bands sit over the right ones.
  - Multiply alpha by `(0.75 + 0.25·sin(y/H·π))`, so the top and bottom dim slightly.
- **Troughs:** draw a dark strip between bands: `#0A0B0D`, alpha 0.35, width 0.025W, at x_i + 0.035W.

### Star ornaments

- 7 four-point stars (the same shape as `vesper:twinkle`, or a GL_LINES outline of 8 points).
- Size 0.012–0.018H, colour `#A3A7B0`, alpha 0.6.
- Place them at fixed seeded positions within x ∈ [0.80W, 0.98W], y ∈ [0.08H, 0.92H], hugging band crests.
- **Twinkle:** alpha 0.35 ↔ 0.8, per-star period 2.4–4.0 s, random phase.
- One star at a time does a sparkle: scale ×1.6 and +45° rotation over 300 ms, then back. Trigger a random star every 2.5 s.

### Nav column

| Element | Value |
|---|---|
| Column left edge | x = 0.085W |
| Header (logo + name) | logo 0.045H tall at x = 0.085W, vertical centre 0.30H; name 0.012H right of the logo, cap height 0.024H, bold, `#F2F3F5` |
| Items | Singleplayer, Multiplayer, Minecraft Settings, Client, Quit Game |
| First item baseline | 0.40H |
| Spacing | 0.056H baseline to baseline. Add an extra 0.02H gap before "Quit Game" **[U: the reference shows a slightly larger gap before Client/Quit]** |
| Item cap height | 0.019H, regular weight, `#C9CCD3` |
| Hit box | text width + 0.04H × 0.045H, left-aligned |

### Item hover / focus (keyboard: Up/Down/Enter/Tab also focus)

All hover transitions run for 140 ms (`easeOutCubic`) and reverse on exit at the same speed.
- Text slides 0 → +0.008H right.
- Colour goes `#C9CCD3` → `#FFFFFF`.
- An accent bar appears at x = column − 0.012H: 2 px wide, 0.022H tall, colour = palette `accent`, height animating 0 → full.
- Optional soft pill behind the text: palette `accent`, alpha 0.12, corner radius 0.01H **[U: in the reference this may be a selection artefact]**.

### Player preview

- Use `GuiInventory.drawEntityOnScreen`-style rendering of the local player, or a dummy with the cached skin.
- **Placement:** feet at (0.72W, 0.88H), model height 0.52H. It renders in front of the curtain bands and behind the stars.
- **Idle:** yaw follows the mouse, ±25° clamped and lerped at 6%/frame; it breathes via the default idle.
- When the preview is on, the cosmetics shown are the equipped cosmetics, from section 4.
- Under the feet, draw a soft ellipse shadow: `#000000` at alpha 0.35, 0.18H × 0.025H.

### Entrance (from splash; 0 = splash exit start)

- Each item: alpha 0→1 and x offset −0.02H→0, `easeOutCubic` over 300 ms, staggered 50 ms starting at 350 ms.
- Curtain: alpha 0→1 over 600 ms, starting at 200 ms.
- Preview: alpha 0→1 plus y +0.03H→0 over 500 ms, starting at 500 ms.

### Transitions to sub-screens (B 4.2–4.6 s shows the menu blurring into the server list)

- Outgoing menu: alpha → 0 and scale 1 → 0.985 over 220 ms.
- Incoming screen: fades in over 220 ms.
- Keep the background and curtain persistent across our own screens.
- Never block vanilla screens.

---

## 3. Vesper sigil logo (original)

**Concept:** "evening star split by the horizon".
- An elongated four-point star cut vertically into two halves, with the left half dropped slightly. This reads as a sharp "split" and a hidden V.
- A small companion diamond sits upper-right.
- No hexagram, no crescent, no cross, and no other religious or political symbol.

**Unit square:** origin bottom-left, y up; the drawing occupies [0,1]². Draw as GL_LINE_STRIP / GL_LINE_LOOP with `GL_LINE_SMOOTH` and the hint set to `GL_NICEST`. Line width = `max(1.5, size_px / 28)`.

```
RIGHT HALF  (GL_LINE_STRIP, 5 pts)        LEFT HALF  (GL_LINE_STRIP, 5 pts, shifted down 0.04)
R0 (0.53, 0.95)  top tip                  L0 (0.47, 0.91)  top tip
R1 (0.58, 0.56)  upper waist              L1 (0.42, 0.52)  upper waist
R2 (0.86, 0.50)  east tip                 L2 (0.14, 0.46)  west tip
R3 (0.58, 0.44)  lower waist              L3 (0.42, 0.40)  lower waist
R4 (0.53, 0.09)  bottom tip               L4 (0.47, 0.05)  bottom tip

COMPANION DIAMOND (GL_LINE_LOOP, 4 pts)   HORIZON TICK (GL_LINES, optional at sizes >= 48 px)
D0 (0.80, 0.88)                           T0 (0.62, 0.27) -> T1 (0.74, 0.27)
D1 (0.85, 0.82)                           T2 (0.26, 0.23) -> T3 (0.38, 0.23)   (shifted with left half)
D2 (0.80, 0.76)
D3 (0.75, 0.82)
```

- **Proportions:** the star is 0.86 tall by 0.72 wide, so the vertical:horizontal ratio is about 1.2:1 and it reads "elongated". The split gap is 0.06 wide.
- **Filled variant** (HUD at ≤16 px, where lines get mushy): triangulate each half as a fan from its waist midpoint, (0.555, 0.50) on the right and (0.445, 0.46) on the left, and fill the diamond. Keep a 0.06 gap.
- **Colour:** `#F2F3F5` on dark backgrounds. On hover/flash, use the palette `accent` plus a halo (`vesper:glow` at 1.8× size, alpha 0.25).
- **Sparkle anchor:** R0, the top tip.
- **Draw-on order** (splash): R0→R4, then L0→L4, then the diamond, then the ticks.
- **Minimum clear space:** 0.15 × size on all sides.

---

## 4. Cosmetics

All cosmetics use `RenderPlayerEvent.Post` or a `LayerRenderer` attached to `RenderPlayer`, in model space after `GlStateManager.translate` to the body. They must:
- respect sneaking (lower by 0.2 and pitch the body),
- hide in first person unless "show in F5" is set,
- obey the palette (section 9) and an intensity slider of 0–100%, which scales alpha and particle count.

### 4.1 Wings

**Reference:**
- A 4.0–7.5 s (frame A_06.7): red bat-like wings on the back. The membrane is translucent crimson with darker veins; the wings sit slightly raised and flap gently while walking.
- B 6.6–11.8 s: dark bat wings, near-black and small, behind the shoulders, with spiky scalloped edges.

**Geometry (per wing, mirrored for the left wing):**
- **Root:** at the back, (±0.10, 1.38, +0.16) relative to the feet, in body space with +z = back.
- **Bones:** 3 finger bones fan out from the root, in the root's local plane before flap rotation:
  | Bone | Direction | Length |
  |---|---|---|
  | B1 (top edge) | (0.92, 0.38, 0.10) | 0.85 |
  | B2 | (0.95, −0.05, 0.12) | 0.75 |
  | B3 | (0.80, −0.45, 0.10) | 0.60 |
  Arm bone: a thin box 0.05×0.05 along B1. Bones B2 and B3 are boxes 0.035×0.035.
- **Membrane:** triangles (root, B1tip, M1), (root, M1, B2tip), (root, B2tip, M2), (root, M2, B3tip). Each M is a scallop point at 70% of the neighbouring tips' midpoint, pulled 0.10 toward the root, which gives the bat "notch".
- **Total span:** about 2.0 blocks, which is 3.3× the player width (A_06.7 shows a span of about 2.6–3× the body). The "B dark" variant is scaled 0.75 (span about 1.5).
- **Rendering:**
  - Membrane: two-sided, alpha 0.78, colour = palette `wingMembrane`. A vertex colour gradient goes from the root (darker, `wingShadow`) to the edge (`wingMembrane`).
  - Ribs: opaque `wingRib`.
  - Optional rim glow: an additive line strip along the outer edge in `accent`, alpha 0.35, line width 2.
- **Flap animation:** rotation about the body's Y axis (fold) plus Z axis (lift).
  - Idle: `fold = 18° + 8°·sin(2π·t/1600ms)` and `lift = 10° + 6°·sin(2π·t/1600ms + 0.6)`.
  - Sprinting or airborne: period 600 ms, fold amplitude 22°, lift amplitude 14°.
  - Sneaking: fold 40°, which tucks the wings.
- **Not shown in the reference:** trailing feathers or particles. Optionally emit 1 ember per 200 ms from the wing tips in the red palette **[our addition]**.

### 4.2 Comet plume (above head)

**Reference:**
- **B 5.0–7.8 s** (frame B_06.6), blue:
  - A bright cyan-white teardrop core sits ≈0.25 block above the head, about 0.35 wide and 0.6 tall.
  - A long, wavy, deep-blue flame tail rises 3–4 player-heights (≈5–7 blocks) and leans right at about 10–15°.
  - The tail breaks into lumps and fades to transparent toward the top.
  - The core is overexposed (white), the mid tail is saturated electric blue, and the outer tail is deep blue at low alpha.
- **B 8.0–11.0 s** (B_08.7), red/pink: the tail is mostly absent. A white-hot orb (~0.45 block) has a pink-red halo and a few faint streaks. Treat this as the "Orb" variant or a low-tail state **[U: the tail may have been switched off]**.
- **B 11.0–13.6 s** (B_11.8), violet: the core is white-lavender, with a long lavender/violet tail of about 5 blocks, smoother and less lumpy.

**Implementation:**
- **Anchor:** P0 = head top + (0, 0.30, 0). Follow the head's yaw, but **not** its pitch.
- **Core:** two stacked `vesper:glow` quads, camera-facing:
  - inner: 0.30 × 0.50, colour `#FFFFFF`, alpha 0.95
  - outer: 0.75 × 1.05, colour `coreTint`, alpha 0.6
  - The core is stretched vertically by drawing an axis-aligned billboard (cylindrical, about world Y).
- **Tail:** a chain of N = 28 nodes from P0 upward.
  - Node k is at `y = 0.25 + k·0.20` blocks (≈5.8 blocks max).
  - `x = 0.04k·cos(lean)`, with lean 12° to the player's right.
  - Plus sway `0.10·(k/N)^1.3 · sin(2π(now/1100ms) − 0.45k)` in x and the same at a 0.7 phase in z.
- **Each node:** a `vesper:glow` billboard.
  - Width: `0.42 · (1 − 0.55·k/N)`.
  - Alpha: `0.85 · (1 − k/N)^1.6`.
  - Colour: lerp `tailHot` → `tailMid` → `tailCold` at k/N = 0, 0.35, 1.
  - Draw the nodes in 2 passes (core width ×0.5 in white-ish `tailHot`, then halo width ×1.8 at 0.35 alpha).
- **Lumps:** every 3rd node, multiply width by `1 + 0.35·noise1D(k + now/400ms)`.
- **Sparks:** emit 2 per 100 ms from the core. Each spark is a 0.04 billboard with upward velocity 0.6–1.0 blocks/s and x jitter ±0.15; it lives 600–900 ms and fades out with `easeInQuad`.
- **Variants:**
  - `Comet` (default): N = 28.
  - `Orb`: N = 0. Core ×1.3, plus 6 radial spike streaks (0.25 long, additive lines, alpha 0.4) rotating at 20°/s.
  - `Wisp`: N = 18, smooth (no lumps).

### 4.3 Thorn crown under the comet

**Reference:** B_06.6 / B_08.7 / B_11.8. A dark segmented ring wraps under the comet core like a U or horseshoe open at the top. It is made of chunky dark-metal links, with two longer upright thorn spikes at the open ends that catch a coloured glint at the tips.

**Geometry:**
- **Ring centre:** P0 + (0, −0.05, 0). Radius 0.30, lying in the camera-facing vertical plane.
- **Extent:** the ring spans angle −215°…+35°, which is an arc of 250°, open at the top-right. This is mirrored so the opening faces up.
- **Links:** 11 boxes along the arc, each 0.07 × 0.07 × 0.10, rotated tangent to the arc.
  - Alternate colours `#24252B` / `#383A42`.
  - Each link carries one small "thorn" pyramid (base 0.04, height 0.06) on its outer face, pointing outward radially, in `#1A1B20`.
- **End spikes:** at each arc end, a tapered spike 0.20 tall, base 0.05, tilted 15° outward.
  - Colour gradient: `#2A2B31` at the base → palette `accent` at the tip.
  - Tip glint: a `vesper:twinkle` sprite, 0.09, alpha `0.4 + 0.5·pulse(1400ms)`.
- **Motion:** a slow wobble of ±4° about the Z axis with a 3 s period. Its radius pulses ±3%, synced with the core.
- **Lighting:** fake it with per-face vertex shades: top ×1.0, sides ×0.8, bottom ×0.6.

### 4.4 Orbiting glyph tokens (original; replaces the reference coin logos)

**Reference:** B 5.0–11.8 s. Four round coin-like discs about 0.22 block in diameter circle the player at shoulder/chest height (y≈1.25–1.4), in a horizontal orbit of radius ≈0.75. They face the camera and are flat-shaded with a white glyph. In the reference, the discs take the comet's palette colour (blue/orange/violet).

**Our tokens:** 4 discs, each with an original glyph drawn as white GL lines.

| Token | Glyph (unit disc, radius 1, line width 2) |
|---|---|
| **Spark** | Four-point star: (0, 0.7)(0.15, 0.15)(0.7, 0)(0.15, −0.15)(0, −0.7)(−0.15, −0.15)(−0.7, 0)(−0.15, 0.15), as a loop |
| **Chevron** | Two nested chevrons: (−0.5, 0.35)(0, −0.25)(0.5, 0.35); (−0.5, 0.0)(0, −0.6)(0.5, 0.0) |
| **Orbit** | A circle of r = 0.45 (24 segments), plus a dot at (0.45·cos 40°, 0.45·sin 40°) drawn as a filled 0.12 disc |
| **Split** | The Vesper sigil halves at 0.75 scale (section 3, re-centred) |

**Disc:**
- Fill: radial gradient from palette `token` at the centre to `tokenEdge` at the rim, alpha 0.95, 32-segment fan.
- Rim: a 1.5 px line in `#FFFFFF` at alpha 0.6.
- Halo: `vesper:glow` at 2.2× size, alpha 0.3, in `token`.

**Orbit motion:**
- `θ_i = 2π·now/6000ms + i·π/2`.
- Position: `(0.75·cos θ, 1.32 + 0.05·sin(2θ), 0.75·sin θ)` relative to the player's feet.
- Billboard them (spherical).
- Tokens behind the body (sin θ > 0 in view space) get alpha ×0.6 and are drawn before the player model.

**Settings:** count 2–6 (default 4) and radius 0.6–1.0.

### 4.5 Floor rings

**Reference:**
- **A 8.5–10.5 s** (A_08.9): around the target, a thin white ring lies on the ground.
  - It expands from radius ≈0.8 to ≈2.4 blocks over about 1.5 s while fading.
  - A second, fainter ring follows.
  - Small white glints sit on the ring line.
- **A 10.5 s** (A_10.5): a bright, thick, white glowing ring sits tight under one player's feet, radius ≈0.45. It has a strong halo and looks like a "selected" or spawn marker.

**Implementation:**
- **Ring:** a triangle-strip annulus at y = ground + 0.02, with 64 segments.
- **Pulse ring:**
  - Radius r(t) = 0.8 → 2.4 with `easeOutCubic`, width 0.06, alpha 0.9·(1−t).
  - Period 1600 ms; spawn a new ring every 800 ms (max 2 alive).
  - Colour `#FFFFFF` with an outer halo strip: a width 0.25 annulus with alpha falling from 0.3 at the inner edge to 0 at the outer.
- **Marker ring** (under self/target): radius 0.45, width 0.08, alpha 0.95.
  - Halo annulus 0.30 wide, alpha 0.4→0, plus a 0.05 inner soft glow.
  - "Breathes": radius ×(1 + 0.04·pulse(1200ms)).
  - Colour: `#FFFFFF` blended 20% toward palette `accent`.
- **Glints on the ring line:** 4 `vesper:twinkle` sprites, 0.12 in size, at angles that rotate 15°/s, alpha 0.7.

### 4.6 Ambient glints (all cosmetics)

- `vesper:twinkle` sprites, 0.06–0.14 in size, spawned within a radius of 1.2 around the player at y 0.3–2.4.
- Rate: 6/s at 100% intensity.
- Life: 500–900 ms. Scale follows 0→1→0 (sin), with rotation 0→45°.
- Colour: 70% `#FFFFFF`, 30% `accent`.

---

## 5. Skies and backdrops (procedural; no textures copied)

**Rendering:**
- In world, replace the sky through a `IRenderHandler` set on `WorldProvider.setSkyRenderer` (client-side only; toggleable; auto-off when "multiplayer-safe" is on).
- In menu/showroom, draw directly.

**Dome:**
- A sky sphere of radius 100, 32 longitude × 16 latitude segments, with vertex colours from the gradient stops below, keyed by elevation `e` from −0.2 (below the horizon) to 1.0 (zenith).
- Draw it at the camera position with depth test off and fog off.
- **Star layer:** precomputed GL_QUADS (in one display list) of camera-facing points on the sphere. Use a fixed seed (`0x5EED_7E5E`) for determinism.

**Twinkle:**
- Per-star alpha `a0·(0.7 + 0.3·sin(now·f + φ))`, with f ∈ [0.6, 2.0] rad/s.
- Do it in immediate mode only for the bright stars (top 10%). Bake the rest into the display list and vary only the list's colour.

### 5.1 Blue starfield night ("Nocturne")

**Reference:** A 4.0–5.6 s (the snow field) and B 5.0–7.8 s (B_06.6). A deep navy sky, faint cyan aurora streaks near the top (A 4.0–5.0), sparse white stars, and a soft grey cloud bank behind the horizon in B.

| e | Colour |
|---|---|
| −0.2 | `#0B1020` |
| 0.0 | `#1E2A44` |
| 0.15 | `#18233D` |
| 0.5 | `#0E1630` |
| 1.0 | `#070B1A` |

- **Stars:** 900 total.
  - Size distribution: 80% at 0.15–0.3, 17% at 0.3–0.5, 3% at 0.5–0.9 (in sphere units at r = 100).
  - Colour `#FFFFFF`, 15% of them tinted `#CFE3FF`. Alpha 0.35–1.0.
- **Aurora:** 2 ribbons, each a vertical curtain strip of 40 columns.
  - Base at e = 0.35, height 0.35, colour `#2FD3E8` → transparent at the top, additive alpha 0.18.
  - Columns sway at `0.04·sin(col·0.3 + now/3000ms)`.
- **Cloud bank:** a ring band at e 0.0–0.12 built from 120 overlapping `vesper:glow` billboards, sizes 8–18, colour `#5A6378` at alpha 0.25. It drifts 0.5°/s in yaw.

### 5.2 Crimson storm ("Bloodmoon")

**Reference:** A 6.0–7.5 s (A_06.7). Turbulent red clouds fill the sky: bright `#E0443F`-ish highlights over deep maroon. Thin red vertical streaks fall like rain (lines ~1/10 of screen height, a few at a time). There are no stars.

| e | Colour |
|---|---|
| −0.2 | `#2A0508` |
| 0.0 | `#8E1A22` |
| 0.2 | `#B3262D` |
| 0.55 | `#6E0F1B` |
| 1.0 | `#3A0612` |

- **Cloud billows:** 260 `vesper:glow` billboards at e 0.05–0.9, sizes 10–30.
  - Colours are a random pick from {`#D9483F` a0.22, `#A51E2B` a0.28, `#5C0A16` a0.35 (normal blend, darkening)}.
  - Drift 1.2°/s in yaw. Render the dark ones with normal blend first, then the bright ones additively.
- **Red rain streaks:** 40 alive at a time, in world space within 30 blocks.
  - Each is a vertical line 1.5–3 blocks long, colour `#FF3A3A`, alpha 0.5→0 toward the top end.
  - They fall at 18 blocks/s and respawn above.

### 5.3 Violet storm / rift ("Riftstorm") — transition seen in A

**Reference:** A 7.0–8.5 s. A purple, electrified cloud sky with bright white-violet filaments, plus **right-angled neon violet polyline bolts** drawn as world-space lines that crawl across the ground and sky. The player is tinted violet by the light.

| e | Colour |
|---|---|
| −0.2 | `#1A0830` |
| 0.0 | `#5B2A9A` |
| 0.3 | `#7B3FC4` |
| 0.7 | `#4A1D80` |
| 1.0 | `#22093F` |

- **Filaments:** 30 curves.
  - Each is a polyline of 12 points jittered ±3° in yaw and pitch. Draw additive, colour `#E6CCFF`, alpha 0.25, line width 1.5.
  - Re-randomize 3 of them every 400 ms (cross-fade over 200 ms).
- **Neon bolts:** up to 6 polylines with axis-aligned segments of length 2–6 blocks, 3–7 segments each, colour `#C77DFF`, width 2.5, with a halo copy at width 6 and alpha 0.25.
  - "Grow" over 300 ms (visible length 0→100%), hold 400 ms, fade over 300 ms.
- **Ambient light:** set `GlStateManager.color` on the world fog to `#5B2A9A`, mixed 30% **[U: the player tint in the reference may be a shader]**.

### 5.4 Orange inferno ("Inferno")

**Reference:** A 8.9–14.7 s (A_08.9 to A_14.2). The whole sky is dense churning fire clouds: bright orange `#FF8A1F` billows with dark umber/black gaps, glowing brightest near the horizon, with subtle falling ember streaks.

| e | Colour |
|---|---|
| −0.2 | `#3A1405` |
| 0.0 | `#FF9A2E` |
| 0.12 | `#F07A18` |
| 0.45 | `#B8460C` |
| 1.0 | `#3B1206` |

- **Billows:** 320 billboards, sizes 8–26.
  - Bright additive ones: `#FFB040` a0.25 and `#FF7A1A` a0.3.
  - Dark ones (normal blend): `#2A0E05` a0.45, about 35% of the total.
  - Billows nearer the horizon are larger and brighter.
  - Each billow's size and alpha oscillate ±10% at 0.3 Hz; drift 0.8°/s.
- **Embers:** 60 world-space quads, 0.05 in size, `#FFC060`, rising at 1–2 blocks/s with wobble.
- **Fog colour:** `#C2581A` (warm horizon haze over the terrain, matching the reference's haze).

### 5.5 Purple aurora ("Aurora Veil")

**Reference:** B 11.0–13.6 s (B_11.8).
- A dense starfield, much denser than Nocturne.
- A teal-green aurora wash on the left-centre, fading to violet/magenta on the right.
- A low row of jagged violet mountain silhouettes on the horizon.
- The ground is tinted purple.

| e | Colour |
|---|---|
| −0.2 | `#2B1440` |
| 0.0 | `#5E3A86` |
| 0.10 | `#3E2F6E` |
| 0.35 | `#1E3F5C` (teal band, see aurora) |
| 1.0 | `#0C0F24` |

- **Stars:** 2200 total, 0.12–0.6 in size, alpha 0.4–1.0, 10% tinted `#E6D6FF`.
- **Aurora:** 3 curtain ribbons, e 0.15–0.75, each 50 columns.
  - Colour along the ribbon length: `#2EE6B0` (start) → `#3BA8D8` → `#B04BE0` (end).
  - Additive alpha 0.22. Each column's height varies with noise; sway amplitude 0.05, period 5 s.
- **Mountains:** a ring of 180 vertices at e = 0.0 → peaks at e 0.04–0.11, with the height from 2-octave value noise plus sharp peaks every ~8 vertices.
  - Fill colour `#3D2560` at the bottom → `#5E3A86` at the peak edge, opaque.
  - The silhouette edge is a 1 px additive line in `#9A6BD6`, alpha 0.35.
- **Fog colour:** `#4A2E6E`.

**Sky transitions** (theme switch): cross-fade the dome colours over 1200 ms with `easeInOutCubic`. Fade the old particle sets out over 600 ms while the new ones fade in over 900 ms.

---

## 6. HUD

### 6.1 Top compact status bar

**Reference:** A 8.9–14.7 s (A_08.9).
**Layout:**
- **Logo tile:** a separate square tile at the top-left holds the white logo.
- **Pill bar:** to its right, a dark rounded bar. Each slot has a small icon plus text, separated by faint dividers.
  - In the reference the slots are a player avatar + username, an orange globe icon + server domain, a gauge icon + FPS, and a clock icon + "03:21".
  - Both elements are near-black, slightly translucent, with a corner radius of about 25% of their height.
- **Scale:** at 512 px phone width the bar spans about 45% of the screen width. On a desktop GUI scale, this is roughly a 13 px tall bar.

**Spec** (GUI coordinates, at `ScaledResolution` scale 2):

| Item | Value |
|---|---|
| Position | x = 6, y = 6, top-left. Draggable via the HUD editor |
| Logo tile | 14×14, radius 3, fill `#0E0F13` α 0.82, 1 px inner border `#FFFFFF` α 0.06, sigil (filled variant) 9×9 centred in `#F2F3F5` |
| Gap | 3 |
| Bar | height 14, radius 3, fill `#0E0F13` α 0.78, padding 6 horizontally |
| Slot | icon 7×7 (drawn with GL lines/fills, original), 3 gap, text in MC font at 0.75 scale, colour `#E4E5EA`; label tails (e.g. "fps") in `#9A9DA6` |
| Divider | 1×7 px vertical line `#FFFFFF` α 0.10, 5 px margin each side |

**Slots** (order configurable; defaults in **bold**):
1. **Player:** a 7×7 head from the skin face (8,8)-(16,16), plus the username. Optional; off in "streamer mode".
2. **Server:** globe icon (circle plus 2 meridian arcs) in palette `accent`; text = the server address, or "Singleplayer". Hide the address in streamer mode.
3. **FPS:** gauge icon (a half circle with a needle). Text `1408 fps`, coloured `#7CE38B` at ≥ 120, `#E4E5EA` at 60–119, `#FF6B6B` below 60.
4. **Ping:** a signal icon of 3 bars. Text `23 ms` from `NetworkPlayerInfo.getResponseTime()`; hidden in singleplayer.
5. **Clock:** a circle icon with 2 hands; local time `HH:mm`, with a 12/24h setting.
6. Optional slots: session timer, CPS (only if allowed), coordinates.

**Corner radius:** draw the rounded rects as a centre quad plus 4 quarter-circle fans of 6 segments each.
**Show animation:** on world join, the slots slide down from y−20 with alpha 0→1, staggered 60 ms, `easeOutBack` over 350 ms.
**Values:** update at most every 250 ms (FPS smoothing over a 1 s window).

### 6.2 Target info line (nametag style)

**Reference:** A 8.9–14.2 s, shown above an NPC's head.
- Line 1: the name (`TestNPC_91`), white.
- Line 2: `20 HP · 2m · 0ms` in light grey.
- Both lines sit on the vanilla-style translucent dark nametag background. In A_10.5 the text is about 1.25× vanilla nametag scale **[U]**.

**Spec:**
- **Anchor:** the target's head top + 0.55. Billboard with the vanilla nametag transform (`-0.025 × scale` uniform), scale 1.15 × vanilla.
- **Line 1:** the name in `#FFFFFF`.
- **Line 2:** `{hp} HP · {dist}m · {ping}ms` in `#D8DAE0`.
  - The middle dot is U+00B7 in `#8A8D96`.
  - Colour the HP value by fraction: `#7CE38B` above 60%, `#FFD166` at 30–60%, `#FF6B6B` below 30%.
- **Background:** each line gets its own box, padding 2 px, colour `#000000` α 0.25 (vanilla `0x40000000`).
- **Fade:** in over 150 ms when the target is acquired (crosshair or last hit within 3 s), out over 300 ms when lost.
- **Compliance:**
  - The whole HP/distance/ping line is **OFF by default on any multiplayer server**. In singleplayer and the local showroom/test NPC it defaults ON.
  - On multiplayer it is enabled only by an explicit per-server allow entry. Without that entry, show only the name, which vanilla already shows.
  - Never read data the vanilla client does not have.

---

## 7. Death/kill visuals → five original kill effects

### 7.1 What the reference shows (do not assume these are kill-triggered)

- **A 12.9 s** (A_12.9):
  - A dark-skinned player model lies horizontal in the foreground, with legs extended toward the camera **[U: it could be a posed model or a death-pose showcase]**.
  - About 10 short white light shards stand vertically on the ground in a scattered ring of radius ~1.5–2.5 blocks, each ~0.3 block long.
  - Several 4-point white twinkles float at waist to head height, one right on the face.
- **A 10.5 s:** a bright white ring under the player's feet (section 4.5 marker ring).
- **A 8.9 s:** an expanding white ground ring around the target.

**Shared vocabulary taken from this:** ground shard bursts, 4-point twinkles, ground rings, a horizontal body pose. Our five effects recombine these with distinct shapes and palettes.

**Common rules for all five effects:**
- Duration 1500 ms.
- Origin O = the victim's last position (feet).
- Client-only.
- Particle cap 160 per effect.
- Reduced-flash mode: halve alpha and remove any frame where full-screen brightness rises by more than 10%.
- The victim body is **never** gore. The optional "lay flat" pose rotates the dead entity 90° about its facing axis over 250 ms (`easeOutCubic`), which is vanilla-like.
- The camera is never moved in competitive mode.

### 7.2 Astral Collapse (palette: blue/teal)

| Phase | ms | Visual |
|---|---|---|
| Gather | 0–350 | 24 star points (twinkle 0.10) spawn on a sphere of r = 1.6 around O+(0, 1.0, 0) and start a spiral: `r(t) = 1.6·(1−easeInQuad(t))`, `θ += 540°` over the phase. A ground ring appears at r = 1.4 (width 0.06, `#7FE7FF`) |
| Compress | 350–700 | The points reach r ≈ 0.15. The ring contracts 1.4 → 0.3 (`easeInOutCubic`), alpha 1. The core glow at O+(0, 1, 0) grows 0 → 0.8 (`#FFFFFF` core, `#38C6F4` halo) |
| Burst | 700–850 | The core flashes to 1.2 then collapses to 0. 40 micro-stars eject radially at 3–5 blocks/s (drag 0.9/tick) |
| Dissolve | 850–1500 | The micro-stars twinkle (alpha flicker at 12 Hz, decaying) and drift down at 0.2 blocks/s. All alpha → 0 with `easeInQuad` |

### 7.3 Violet Rift (palette: violet/magenta)

| Phase | ms | Visual |
|---|---|---|
| Open | 0–300 | A vertical ring (plane facing the camera, cylindrical billboard) at O+(0, 1, 0) opens: radius 0 → 0.9 (`easeOutBack`). A wireframe hexadecagon outline (16 segments), line width 2.5, `#C77DFF`, with an inner rotating square (rotation 90°/s) in `#F0B8FF` |
| Tear | 300–650 | The interior fills with a radial gradient (`#14051F` at centre, α 0.85 → `#7B3FC4` at the edge). 6 right-angle neon bolts (as in section 5.3) crawl outward along the ground, length 1–2.5 |
| Shards | 450–1100 | 18 thin shards (quads 0.04 × 0.35, additive, `#E6CCFF`) rise from the ring at 1.5–3 blocks/s with slight spin, fading over their 500 ms life |
| Close + haze | 1000–1500 | The ring radius goes 0.9 → 0 (`easeInQuad`). The haze is 10 large glow billboards (size 1.2–2.0, `#7B3FC4`, α 0.2 → 0) expanding ×1.4 |

### 7.4 Solar Bloom (palette: red/amber)

| Phase | ms | Visual |
|---|---|---|
| Ignite | 0–150 | Core flare at O+(0, 0.9, 0): size 0 → 1.4, `#FFF4C2` core / `#FF9A2E` halo. 8 radial light rays (additive tapered quads 0.08 wide, length 0 → 1.6), rotating 30°/s |
| Wave | 150–600 | A ground shockwave annulus with r 0.3 → 3.0 (`easeOutCubic`), width 0.25 → 0.05, colour `#FFB040` → `#FF5A1F`, α 0.9 → 0 |
| Embers | 150–1300 | 50 ember **squares** (axis-aligned billboard quads, rotated 45°, size 0.05–0.10). Launched upward in a 50° cone at 2–4 blocks/s with gravity −6 blocks/s². Colour lerps `#FFE08A` → `#FF6A1A` → `#7A1A0A`. Life 700–1100 ms |
| Afterglow | 600–1500 | A floor disc glow (r 1.0, `#FF7A1A`, α 0.35 → 0) and the rays shrinking to 0 |

### 7.5 Frost Constellation (palette: blue/teal, ice variant)

| Phase | ms | Visual |
|---|---|---|
| Form | 0–500 | 8 six-arm crystal flakes appear one by one (60 ms stagger) on a horizontal ring of r = 1.1 at y = O+0.9. Each flake is drawn as 3 crossed lines (0°, 60°, 120°) plus small 30°-angled ticks at 60% arm length, size 0.22, `#E8FBFF`, line width 2. Scale 0 → 1 with `easeOutBack`. *(A 6-arm snowflake is not a religious symbol; keep the arms simple)* |
| Link | 400–800 | Thin constellation lines link neighbouring flakes (an 8-gon), plus 4 lines to a centre star at O+(0, 1.6, 0), `#9EE8FF`, α 0.6, drawn on progressively |
| Twinkle | 800–1100 | All flakes flash (α 1 → 0.4 → 1, twice). 30 tiny cold sparkles (`#FFFFFF`, 0.04) fall at 0.6 blocks/s inside the ring. A floor frost ring (r 1.1, width 0.12, `#BDEFFF`, α 0.5) |
| Fade | 1100–1500 | Flakes rotate +30° and shrink 1 → 0.6, α → 0 (`easeInQuad`). Lines fade first (1100–1300) |

### 7.6 Comet Ascension (palette: blue-white)

| Phase | ms | Visual |
|---|---|---|
| Strike | 0–200 | A ground flash ring at O, r 0.5, white, α 1 → 0.4 |
| Arcs | 100–1000 | 3 comets launch from O+(0, 0.3, 0) along Bézier arcs: P0 = O, P1 = O + (±1.2, 1.8, ±0.6), P2 = O + (±0.4, 3.6, ±0.2), with yaws spaced 120°. Each comet head is a glow of 0.25 (white core, `#7FD4FF` halo). The trail is the last 14 positions as a ribbon, width 0.18 → 0 and α 0.8 → 0, colour `#BFE8FF` → `#3A7BFF`. Easing along the path: `easeOutCubic` |
| Shards | 0–600 | **Reference echo (A 12.9):** 10 vertical white light shards (quads 0.03 × 0.3, additive) pop up on the ground at random r 1.2–2.2, then hold |
| Apex | 900–1100 | The comets converge near O+(0, 3.6, 0) in a single twinkle burst (size 0.6) and 12 sparks |
| Fade | 1100–1500 | Trails and shards fade (`easeInQuad`). The burst afterglow lingers to 1500 ms |

**Preview:** every effect must be playable on the showroom dummy (FEATURE_MATRIX P0 "Preview world"), from a button in the Client GUI.

---

## 8. Showroom prop: hover-cart + accretion ring + flame

**Reference:** B 14.0–16.6 s (frames B_14.4, B_16.0).
- **Car:** a dark luxury car model on a bright green field. We do not copy it.
- **Ring:** above the car hangs a black-hole symbol, made of:
  - a near-edge-on flattened accretion disc, about 3 car-widths wide,
  - a black central sphere,
  - a bright lensed arc looping over the top and a thinner one under the bottom.
- **Ring colouring:** a white-hot pink inner edge, magenta → deep red outward on the right side, and blue/white and pink speckle on the left side. This reads as Doppler-ish asymmetry **[U: which side is brighter varies between frames]**.
- **Flame:** below the ring and above the car floats a flame, about 1.2 blocks tall. It has a yellow-white core and orange/red edges, with a spiky red ring at its base (a crown variant).
- **Extras:** coin tokens float around the car, and white snowflake/sparkle particles drift around.

### 8.1 Original hover-cart ("Skiff") — boxes only, units in blocks, origin at the footprint centre on the ground

| Part | Box min → max (x, y, z) | Colour |
|---|---|---|
| Hull lower | (−0.70, 0.35, −1.30) → (0.70, 0.60, 1.30) | `#1B1D24` |
| Hull upper/deck | (−0.60, 0.60, −1.10) → (0.60, 0.75, 0.90) | `#262934` |
| Nose wedge | (−0.55, 0.35, 1.30) → (0.55, 0.55, 1.60), top face sloped to y 0.40 at z 1.60 | `#1B1D24` |
| Canopy | (−0.45, 0.75, −0.35) → (0.45, 1.10, 0.45), alpha 0.55 | `#2E3A55` with a 1 px rim `#9FB4D8` |
| Seat block | (−0.30, 0.75, −0.80) → (0.30, 1.00, −0.40) | `#3A2F44` |
| Tail fin ×2 | (±0.50, 0.75, −1.30) → (±0.62, 1.15, −0.95) | `#262934` |
| Hover pods ×4 | centres (±0.62, 0.30, ±0.95), size 0.30 × 0.14 × 0.40 | `#14151A` |
| Pod glow ×4 | discs under each pod at y 0.12, r 0.22, additive | palette `accent`, α `0.55 + 0.25·pulse(900ms)` |
| Trim strip | a line loop around the hull at y 0.60, width 2 | palette `accent`, α 0.8 |
| Emblem | the Vesper sigil on the nose, 0.25 tall | `#F2F3F5` |

- **Bob:** y offset `0.05·sin(2π·now/2400ms)`.
- **Turntable:** yaw 12°/s.
- **Not drivable**, and never spawned on a server.
- **Shading:** per-face vertex shades: top 1.0, ±x 0.75, ±z 0.85, bottom 0.5.

### 8.2 Accretion ring ("Eclipse Halo")

- **Centre:** C = cart origin + (0, 3.2, 0). It is cylindrically billboarded, so it always faces the camera in yaw. The disc plane is tilted 78° from vertical, so it is seen nearly edge-on.
- **Event horizon:** a camera-facing disc, r = 0.42, `#000000` α 1, drawn with depth write on.
- **Photon ring:** a camera-facing annulus, r 0.42–0.48, `#FFE9F4`, additive α 0.9, plus a halo annulus out to r 0.70 with α 0.4 → 0.
- **Accretion disc:** a flat annulus in the tilted plane.
  - Inner radius 0.55, outer radius 1.45, 96 segments × 4 radial rings.
  - Vertex colour by azimuth φ (0 = approaching side, screen-left) and radius ρ (0 inner .. 1 outer):
    - **Doppler factor:** `D = 0.5 + 0.5·cos(φ)`. The approaching side is brighter and bluer.
    - **Hue:** lerp(`#FF2E63` receding, `#8FD3FF` approaching, D) for the outer band. The inner band is always `#FFD6EC` → `#FFFFFF`.
    - **Radial falloff:** α = `(0.95 − 0.8ρ)·(0.55 + 0.45·D)`.
  - Rotate the texture-free speckle: 220 tiny additive points (0.025) riding the disc at angular velocity `ω(ρ) = 70°/s · ρ^−1.5`. Their colour follows the same Doppler rule; their α flickers.
- **Lensed arcs:** a top arc (half-annulus over the horizon disc, r 0.60–0.72 in the camera plane, from 10° to 170°) and a fainter bottom arc (r 0.52–0.58, from 200° to 340°). Colour `#FF8FC4` → `#FFFFFF` at the inner edge; additive α 0.75 for the top arc and 0.45 for the bottom.
- **Draw order:** far half of the disc → horizon disc → photon ring → lensed arcs → near half of the disc → speckle.
- **Wobble:** tilt ±2°, period 6 s.

### 8.3 Floating flame

- **Anchor:** cart + (0, 1.75, 0) as the base, rising to 2.95.
- **Core:** 5 stacked glow billboards, cylindrical.
  | Index | Height y | Size w × h | Colour |
  |---|---|---|---|
  | 0 | 0.10 | 0.45 × 0.55 | `#FFF6B0` |
  | 1 | 0.30 | 0.40 × 0.55 | `#FFE45C` |
  | 2 | 0.55 | 0.32 × 0.50 | `#FFB12E` |
  | 3 | 0.80 | 0.24 × 0.45 | `#FF7A1A` |
  | 4 | 1.05 | 0.14 × 0.35 | `#E0242B` |
  Each node sways in x by `0.04·k·sin(now/180ms + k)`.
  Alpha ramps 0.95 → 0.5 with k.
- **Outer halo:** a single glow of 1.2 × 1.6, `#FF5A1F`, α 0.3.
- **Licks:** 3 additive tongue quads (0.08 × 0.35), randomly re-seeded every 220 ms, `#FFD15C` → transparent.
- **Base crown:** the thorn crown from section 4.3, lying horizontal (ring plane XZ), r 0.30, 12 spikes, spike colour `#B3161E` → `#FF4A3A` at the tips.
- **Sparks:** 4/s, `#FFC060`, rising 1.2 blocks/s, life 600 ms.

### 8.4 Snow sparkles (the B 14.4 ambience)

- 40 six-arm flakes (from section 7.5, size 0.08–0.14) and twinkles drift within a 4-block-radius cylinder.
- Fall 0.25 blocks/s, sway ±0.2, `#FFFFFF` α 0.5–0.9.

---

## 9. Preset palettes

Every cosmetic, kill effect and sky reads from the active palette. Keys:

| Key | Use | **Tidewater** (blue/teal) | **Emberfall** (red/amber) | **Nightbloom** (violet/magenta) |
|---|---|---|---|---|
| `accent` | UI accents, rim lines, pod glow | `#38C6F4` | `#FF5A3C` | `#B45CFF` |
| `accentSoft` | hover pill, halos | `#1E6F99` | `#8C2A1C` | `#5E2A8C` |
| `coreTint` | comet/orb outer core | `#9FE8FF` | `#FF9AA8` | `#E2C6FF` |
| `tailHot` | comet tail start | `#E6FBFF` | `#FFE3E0` | `#F6EEFF` |
| `tailMid` | comet tail middle | `#2F7BFF` | `#FF3B4E` | `#A57BFF` |
| `tailCold` | comet tail end | `#0B2A9E` | `#7A0E2A` | `#4A1D80` |
| `wingMembrane` | wing fill | `#1F5C8F` | `#C21F3A` | `#6B2FA6` |
| `wingShadow` | wing root gradient | `#0B1E33` | `#4A0814` | `#220C3D` |
| `wingRib` | wing bones | `#0E1824` | `#3A0610` | `#1A0B2B` |
| `token` | orbit disc centre | `#2FA8E8` | `#FF8A2E` | `#9A5CF0` |
| `tokenEdge` | orbit disc rim | `#155A9E` | `#C2410C` | `#5A2AA8` |
| `glow` | generic additive glow | `#7FD4FF` | `#FFB040` | `#E07BFF` |
| `ring` | floor rings | `#DFF6FF` | `#FFE6CC` | `#F3E2FF` |
| `spark` | ambient glints | `#FFFFFF` / `#BFEFFF` | `#FFFFFF` / `#FFD08A` | `#FFFFFF` / `#F0C8FF` |
| `sky` | default sky preset | Nocturne (5.1) | Bloodmoon (5.2) or Inferno (5.4) | Aurora Veil (5.5) or Riftstorm (5.3) |
| `kill` | default kill effect | Astral Collapse / Frost Constellation / Comet Ascension | Solar Bloom | Violet Rift |

- **Neutral UI** (independent of palette):
  - backgrounds `#0E0F13`, `#18191D`, `#2A2C31`
  - text `#F2F3F5` (primary), `#C9CCD3` (secondary), `#9A9DA6` (muted)
  - dividers `#FFFFFF` at α 0.06–0.12
- **Status colours:** good `#7CE38B`, warn `#FFD166`, bad `#FF6B6B`.
- **Colourblind-safe option:** swap Emberfall's `tailMid` to `#FF8A1F` and the status colours to blue `#4DA3FF` / yellow `#FFD166` / magenta `#E05CFF`.

---

## 10. Asset list (all original, generated or hand-modelled in code)

- **Runtime textures:** `vesper:glow` (64², radial) and `vesper:twinkle` (64², four-point). Optionally `vesper:softsquare` (32², for embers).
- **Vector data:**
  - the Vesper sigil (section 3),
  - 4 token glyphs (section 4.4),
  - 7 UI icons: globe, gauge, signal, clock, head frame, gear, search,
  - the 6-arm flake.
- **Procedural meshes:** the wing membrane, thorn crown, hover-cart boxes, sky dome, mountain ring and accretion disc.
- **No bundled images** from the references. No sounds are specified here.

---

## 11. Fidelity checklist (screenshot comparison, 1920×1080 and 3840×2160)

1. **Splash:** the sigil is centred at about 0.40H and is 0.16H tall, with the name under it and a pulsing "Press to enter". The background is dark charcoal with a vignette, with no pure black at the centre. The logo is visibly *not* a hexagram.
2. **Splash → menu:** a single continuous logo move into the header (≤ 800 ms). There is no blank frame and no flash of the vanilla menu.
3. **Menu:**
   - Left column at about 0.085W, 5 items, spacing ≈ 0.056H, small type (cap ≈ 0.019H).
   - 5 grey satin bands on the right third with dark troughs.
   - 6–8 small twinkling 4-point stars at the right edge.
   - The player stands centred at about 0.72W, in front of the bands.
4. **Hover:** text shifts right and brightens, and the accent bar appears in ≤ 150 ms. Keyboard focus is visible.
5. **Wings:** span ≈ 3× the body width, translucent membrane with darker ribs, scalloped trailing edge, gentle flap (1.6 s idle).
6. **Comet:**
   - The core sits just above the head and is overexposed white.
   - The tail rises ≥ 3 player-heights, leans right, sways and fades.
   - The thorn crown hugs the core's underside, open at the top, with 2 glinting spikes.
7. **Tokens:** 4 discs at chest height, radius ≈ 0.75, facing the camera, with original glyphs (no currency logos), passing behind the body at reduced alpha.
8. **Skies:** each of the 5 presets is recognisable at a glance by its dominant hue (navy / crimson / violet storm / orange fire / teal-to-violet aurora with mountains). Stars are not visible in Bloodmoon or Inferno. Theme switching cross-fades with no pop.
9. **HUD bar:** a separate logo tile plus a dark rounded pill at the top-left, with icons, thin dividers and real FPS, ping, clock and server values. Nothing overlaps the vanilla boss bar or chat.
10. **Target line:** 2 lines, vanilla nametag background, `HP · m · ms` format. It is **hidden on multiplayer by default**.
11. **Floor rings:** the expanding ring around the target (0.8 → 2.4) and the bright tight marker ring (r 0.45) both read white with a soft halo.
12. **Kill effects:** each finishes in 1.5 s, stays under 160 particles, and is distinguishable by shape alone in greyscale (spiral/inward, ring + upward shards, radial rays + embers, 6-arm flakes + lines, 3 arcing comets). Reduced-flash mode works.
13. **Showroom:** the box-built Skiff (not a real car brand), the accretion ring with a black centre, a brighter blue approaching side and a pink/red receding side, top and bottom lensed arcs, and a flame with a red spiky base crown. There are no third-party logos anywhere.
14. **Performance:** with all cosmetics, sky and HUD on, the frame time increase is ≤ 1.5 ms on the reference machine. GL state is restored (no tinted chat or broken transparency after rendering).
