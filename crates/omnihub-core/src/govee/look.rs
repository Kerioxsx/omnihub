//! What a Govee light shows, frame by frame: the screen's colours around a
//! zone, the music (the album cover's colours, pulsing with the beat), or
//! one of the lighting effects. Pure maths: the hub feeds it and sends the
//! result.

use serde::{Deserialize, Serialize};

use crate::capture::sampler::Grid;
use crate::rig::lighting::{self, Effect, Inputs, LightProfile};
use crate::rig::openrgb::Rgb;

/// Where on the screen a light takes its colours from.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Zone {
    /// The whole screen, one colour.
    #[default]
    Whole,
    /// Along the left edge, top to bottom.
    Left,
    /// Along the right edge, top to bottom.
    Right,
    /// Along the top edge, left to right.
    Top,
    /// Along the bottom edge, left to right.
    Bottom,
    /// Up the left edge, across the top, down the right (a strip behind
    /// the monitor or TV, starting bottom left).
    Around,
    /// Left to right across the whole height (a light bar or a row of lamps).
    Across,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MusicStyle {
    /// The whole light pulses with the beat, a new colour on each one.
    #[default]
    Pulse,
    /// Each segment a band of frequencies, bass first.
    Spectrum,
    /// The colours flow along the light at the song's tempo; beats send a
    /// flash from the middle.
    Flow,
}

/// A colour with channels 0–1.
pub type C = [f32; 3];

pub fn to_c(c: Rgb) -> C {
    [c.0 as f32 / 255.0, c.1 as f32 / 255.0, c.2 as f32 / 255.0]
}

pub fn to_rgb(c: C) -> Rgb {
    let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    Rgb(q(c[0]), q(c[1]), q(c[2]))
}

fn scale(c: C, k: f32) -> C {
    [c[0] * k, c[1] * k, c[2] * k]
}

fn hsv(c: C) -> (f32, f32, f32) {
    let max = c[0].max(c[1]).max(c[2]);
    let min = c[0].min(c[1]).min(c[2]);
    let d = max - min;
    let sector = if d <= f32::EPSILON {
        0.0
    } else if max == c[0] {
        ((c[1] - c[2]) / d).rem_euclid(6.0)
    } else if max == c[1] {
        (c[2] - c[0]) / d + 2.0
    } else {
        (c[0] - c[1]) / d + 4.0
    };
    let h = sector / 6.0;
    (h, if max <= f32::EPSILON { 0.0 } else { d / max }, max)
}

fn from_hsv(h: f32, s: f32, v: f32) -> C {
    let h6 = h.rem_euclid(1.0) * 6.0;
    let x = 1.0 - (h6 % 2.0 - 1.0).abs();
    let (r, g, b) = match h6 as u32 {
        0 => (1.0, x, 0.0),
        1 => (x, 1.0, 0.0),
        2 => (0.0, 1.0, x),
        3 => (0.0, x, 1.0),
        4 => (x, 0.0, 1.0),
        _ => (1.0, 0.0, x),
    };
    let s = s.clamp(0.0, 1.0);
    let mix = |c: f32| v * (1.0 - s + s * c);
    [mix(r), mix(g), mix(b)]
}

/// More (or less) saturated: `amount` 1 = as is, 2 = twice.
pub fn saturate(c: C, amount: f32) -> C {
    let (h, s, v) = hsv(c);
    from_hsv(h, s * amount, v)
}

/// Saturated for LEDs, which wash pale colours out to white: a soft curve
/// lifts pale colours first, then `amount` (1 = as is) scales it. Greys
/// stay grey.
fn vivid(c: C, amount: f32) -> C {
    let (h, s, v) = hsv(c);
    if s < 0.08 {
        return c;
    }
    from_hsv(h, (1.0 - (1.0 - s) * (1.0 - s)) * amount, v)
}

/// The average of a rectangle of cells; with `skip_dark`, black cells
/// (letterbox bars, dark menus) don't pull the colour down. Colourful
/// cells count more than grey ones, so a red car on a grey road reads as
/// red rather than a muddy brown. Many colours mixed fade towards white,
/// so a colourful rectangle whose mix comes out pale shows its main colour
/// instead (lights with few segments see big rectangles).
fn average(g: &Grid, x0: usize, x1: usize, y0: usize, y1: usize, skip_dark: bool) -> C {
    let (mut sum, mut n, mut weight, mut dark, mut vivid) = ([0f32; 3], 0f32, 0f32, 0f32, 0f32);
    // Colourful cells by hue, in twelve 30° steps.
    let mut hues = [([0f32; 3], 0f32); 12];
    let (y0, x0) = (y0.min(g.h - 1), x0.min(g.w - 1));
    let (y1, x1) = (y1.max(y0 + 1).min(g.h), x1.max(x0 + 1).min(g.w));
    for y in y0..y1 {
        for x in x0..x1 {
            let p = g.at(x, y);
            if skip_dark && p.iter().all(|&v| v < 14) {
                dark += 1.0;
                continue;
            }
            let (hi, lo) = (p.iter().copied().max().unwrap_or(0), p.iter().copied().min().unwrap_or(0));
            let w = 0.3 + 2.0 * (hi - lo) as f32 / 255.0;
            let c = [p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0];
            let (h, s, _) = hsv(c);
            for i in 0..3 {
                sum[i] += w * c[i];
            }
            n += 1.0;
            weight += w;
            vivid += w * s;
            if hi - lo > 25 {
                let b = &mut hues[((h * 12.0) as usize).min(11)];
                for (a, v) in b.0.iter_mut().zip(c) {
                    *a += w * v;
                }
                b.1 += w;
            }
        }
    }
    if n == 0.0 || n < dark * 0.05 {
        return [0.0; 3];
    }
    let mixed = scale(sum, 1.0 / weight);
    let (want, have) = (0.8 * vivid / weight, hsv(mixed).1);
    if want > 0.16 && have < want {
        // The strongest 90° of hues (a step and its neighbours).
        let around = |i: usize| [(i + 11) % 12, i, (i + 1) % 12];
        let best = (0..12).max_by(|&a, &b| around(a).iter().map(|&k| hues[k].1).sum::<f32>().total_cmp(&around(b).iter().map(|&k| hues[k].1).sum::<f32>())).unwrap_or(0);
        let (mut c, mut w) = ([0f32; 3], 0f32);
        for k in around(best) {
            for (a, v) in c.iter_mut().zip(hues[k].0) {
                *a += v;
            }
            w += hues[k].1;
        }
        if w > 0.0 {
            let main = scale(c, 1.0 / w);
            let t = ((want - have) / (0.5 * want)).clamp(0.0, 1.0);
            return [mixed[0] + (main[0] - mixed[0]) * t, mixed[1] + (main[1] - mixed[1]) * t, mixed[2] + (main[2] - mixed[2]) * t];
        }
    }
    mixed
}

/// The screen's colours for a light of `n` segments along `zone`.
pub fn screen_colors(g: &Grid, zone: Zone, n: usize, skip_dark: bool) -> Vec<C> {
    let n = n.max(1);
    if g.w == 0 || g.h == 0 {
        return vec![[0.0; 3]; n];
    }
    let (w, h) = (g.w, g.h);
    // How deep into the picture an edge zone looks.
    let dx = (w as f32 * 0.18).ceil() as usize;
    let dy = (h as f32 * 0.22).ceil() as usize;
    let part = |i: usize, len: usize| (i * len / n, ((i + 1) * len / n).max(i * len / n + 1));
    match zone {
        Zone::Whole => vec![average(g, 0, w, 0, h, skip_dark); n],
        Zone::Across => (0..n).map(|i| part(i, w)).map(|(a, b)| average(g, a, b, 0, h, skip_dark)).collect(),
        Zone::Top => (0..n).map(|i| part(i, w)).map(|(a, b)| average(g, a, b, 0, dy, skip_dark)).collect(),
        Zone::Bottom => (0..n).map(|i| part(i, w)).map(|(a, b)| average(g, a, b, h - dy, h, skip_dark)).collect(),
        Zone::Left => (0..n).map(|i| part(i, h)).map(|(a, b)| average(g, 0, dx, a, b, skip_dark)).collect(),
        Zone::Right => (0..n).map(|i| part(i, h)).map(|(a, b)| average(g, w - dx, w, a, b, skip_dark)).collect(),
        Zone::Around => {
            // The path: up the left edge, across the top, down the right.
            let len = h + w + h;
            (0..n)
                .map(|i| {
                    let (a, b) = part(i, len);
                    let (mut sum, mut k) = ([0f32; 3], 0f32);
                    for p in a..b {
                        let c = if p < h {
                            let y = h - 1 - p;
                            average(g, 0, dx, y, y + 1, skip_dark)
                        } else if p < h + w {
                            let x = p - h;
                            average(g, x, x + 1, 0, dy, skip_dark)
                        } else {
                            let y = p - h - w;
                            average(g, w - dx, w, y, y + 1, skip_dark)
                        };
                        for j in 0..3 {
                            sum[j] += c[j];
                        }
                        k += 1.0;
                    }
                    scale(sum, 1.0 / k.max(1.0))
                })
                .collect()
        }
    }
}

/// Screen colours as lights show them well: livelier colours, dark scenes
/// a little darker (LEDs wash out mid tones), then dim colours lifted by
/// `boost` so the lights aren't much dimmer than the screen looks.
pub fn screen_tone(c: C, saturation: f32, boost: f32) -> C {
    let c = vivid(c, saturation);
    lift([c[0].powf(1.3), c[1].powf(1.3), c[2].powf(1.3)], boost)
}

/// Brighter, same colour: a dim colour is scaled toward full brightness by
/// `boost` (0 leaves it, 1 makes every colour full). Near-black stays
/// black, the lift fades in over the darkest tones so dark scenes don't
/// flicker bright, and pale colours are lifted less (LEDs show a bright
/// pale colour as white).
pub fn lift(c: C, boost: f32) -> C {
    let v = c[0].max(c[1]).max(c[2]);
    if v < 0.03 || boost <= 0.0 {
        return c;
    }
    let fade = ((v - 0.03) / 0.12).clamp(0.0, 1.0);
    let pale = 0.35 + 0.65 * hsv(c).1;
    let target = v + (1.0 - v) * boost.clamp(0.0, 1.0) * fade * pale;
    scale(c, target / v)
}

/// One of the PC lighting effects across `n` segments.
pub fn effect_colors(effect: Effect, colors: &[Rgb], speed: f32, n: usize, dev: usize, inp: &Inputs) -> Vec<C> {
    let p = LightProfile { effect, colors: colors.iter().map(|c| c.hex()).collect(), speed, brightness: 100, ..Default::default() };
    lighting::frame(&p, colors, n.max(1), dev, inp).into_iter().map(to_c).collect()
}

/// What the music sounds like now (from the PC's sound analysis).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Sound {
    pub level: f32,
    pub low: f32,
    pub beat: f32,
    pub bands: Vec<f32>,
    pub active: bool,
    /// Beats so far: a change is a new beat.
    pub beats: u32,
    pub bpm: Option<f32>,
}

impl Sound {
    pub fn from_json(v: &serde_json::Value) -> Sound {
        let f = |k: &str| v[k].as_f64().unwrap_or(0.0) as f32;
        Sound {
            level: f("level"),
            low: f("low"),
            beat: f("beat"),
            bands: v["bands"].as_array().map(|a| a.iter().map(|b| b.as_f64().unwrap_or(0.0) as f32).collect()).unwrap_or_default(),
            active: v["active"].as_bool().unwrap_or(false),
            beats: v["beats"].as_u64().unwrap_or(0) as u32,
            bpm: v["bpm"].as_f64().map(|b| b as f32),
        }
    }
}

/// Music state kept between frames, per light.
#[derive(Debug, Clone, Default)]
pub struct MusicRun {
    pub beats: u32,
    pub color: usize,
    pub flash: f32,
    pub phase: f32,
    /// Where in the palette the colour is now (glides towards `color`).
    pub pos: f32,
}

/// The music across `n` segments in the palette's colours. `gain` is the
/// sensitivity (1 = as heard); `blend` how long (s) a beat's new colour
/// takes to glide in.
#[allow(clippy::too_many_arguments)]
pub fn music_colors(style: MusicStyle, palette: &[Rgb], s: &Sound, n: usize, dt: f32, gain: f32, blend: f32, run: &mut MusicRun) -> Vec<C> {
    let n = n.max(1);
    let pal: Vec<Rgb> = if palette.is_empty() { vec![Rgb(255, 0, 80), Rgb(0, 160, 255), Rgb(255, 170, 0)] } else { palette.to_vec() };
    if s.beats != run.beats {
        if s.active {
            run.color = (run.color + 1) % pal.len();
            run.flash = 1.0;
        }
        run.beats = s.beats;
    }
    run.flash *= (-dt / 0.22).exp();
    let len = pal.len() as f32;
    let mut d = (run.color as f32 - run.pos).rem_euclid(len);
    if d > len / 2.0 {
        d -= len;
    }
    let k = if blend <= 0.0 { 1.0 } else { 1.0 - (-dt / blend).exp() };
    run.pos = (run.pos + d * k).rem_euclid(len);
    let energy = ((s.low * 0.7 + s.level * 0.6) * gain).clamp(0.0, 1.0);
    let idle = if s.active { 0.0 } else { 0.12 };
    let at = |x: f32| to_c(lighting::along(&pal, x, true));
    match style {
        MusicStyle::Pulse => {
            // Never far down between beats, so a lamp doesn't look switched off.
            let k = if s.active { (0.32 + 0.5 * energy + 0.4 * run.flash).min(1.0) } else { idle };
            let base = run.pos / len;
            (0..n).map(|i| scale(at(base + 0.18 * i as f32 / n as f32), k)).collect()
        }
        MusicStyle::Spectrum => {
            let bands = if s.bands.is_empty() { vec![s.low, s.level, s.level * 0.6] } else { s.bands.clone() };
            (0..n)
                .map(|i| {
                    let (a, b) = (i * bands.len() / n, ((i + 1) * bands.len() / n).max(i * bands.len() / n + 1));
                    let v = bands[a.min(bands.len() - 1)..b.min(bands.len())].iter().copied().fold(0.0, f32::max);
                    let k = if s.active { (0.06 + 0.94 * (v * gain).clamp(0.0, 1.0) + 0.2 * run.flash).min(1.0) } else { idle };
                    scale(at(i as f32 / n as f32 * (pal.len() as f32 - 1.0).max(1.0) / pal.len() as f32), k)
                })
                .collect()
        }
        MusicStyle::Flow => {
            let rate = s.bpm.map(|b| b / 60.0 / 8.0).unwrap_or(0.06);
            run.phase = (run.phase + dt * rate * (0.5 + energy)).rem_euclid(1.0);
            // The flash spreads from the middle as it fades.
            let front = (1.0 - run.flash) * 0.5;
            (0..n)
                .map(|i| {
                    let x = (i as f32 + 0.5) / n as f32;
                    let from_mid = (x - 0.5).abs();
                    let wave = run.flash * (-((from_mid - front).abs() * 8.0)).exp();
                    let k = if s.active { (0.32 + 0.45 * energy + 0.6 * wave).min(1.0) } else { idle };
                    scale(at(x + run.phase), k)
                })
                .collect()
        }
    }
}

/// Eases colours towards their targets: fast up, slower down for music
/// (a beat lands at once and fades), even both ways for the screen.
#[derive(Debug, Clone, Default)]
pub struct Smooth {
    cur: Vec<C>,
}

impl Smooth {
    pub fn step(&mut self, target: &[C], dt: f32, rise: f32, fall: f32) -> Vec<Rgb> {
        if self.cur.len() != target.len() {
            self.cur = target.to_vec();
        }
        let k = |tau: f32| if tau <= 0.0 { 1.0 } else { 1.0 - (-dt / tau).exp() };
        let (ku, kd) = (k(rise), k(fall));
        for (c, t) in self.cur.iter_mut().zip(target) {
            let up = t[0] + t[1] + t[2] >= c[0] + c[1] + c[2];
            let kk = if up { ku } else { kd };
            for j in 0..3 {
                c[j] += (t[j] - c[j]) * kk;
            }
        }
        self.cur.iter().copied().map(to_rgb).collect()
    }

    pub fn reset(&mut self) {
        self.cur.clear();
    }
}

/// The colours of a cover, as lights show them well: up to five, the most
/// present and lively first, at full brightness.
pub fn palette(bytes: &[u8]) -> Vec<Rgb> {
    let Ok(img) = image::load_from_memory(bytes) else { return Vec::new() };
    let small = img.thumbnail_exact(32, 32).to_rgb8();
    let px: Vec<C> = small.pixels().map(|p| [p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0]).collect();
    palette_of(&px)
}

pub fn palette_of(px: &[C]) -> Vec<Rgb> {
    if px.is_empty() {
        return Vec::new();
    }
    // k-means, started from pixels spread through the picture.
    let k = 6.min(px.len());
    let mut centers: Vec<C> = (0..k).map(|i| px[i * px.len() / k]).collect();
    let mut members = vec![0usize; px.len()];
    for _ in 0..10 {
        for (i, p) in px.iter().enumerate() {
            members[i] = (0..k).min_by(|&a, &b| dist(p, &centers[a]).total_cmp(&dist(p, &centers[b]))).unwrap_or(0);
        }
        for (c, center) in centers.iter_mut().enumerate() {
            let (mut sum, mut n) = ([0f32; 3], 0f32);
            for (i, p) in px.iter().enumerate() {
                if members[i] == c {
                    for j in 0..3 {
                        sum[j] += p[j];
                    }
                    n += 1.0;
                }
            }
            if n > 0.0 {
                *center = scale(sum, 1.0 / n);
            }
        }
    }
    let mut groups: Vec<(f32, C)> = centers
        .iter()
        .enumerate()
        .filter_map(|(c, center)| {
            let share = members.iter().filter(|&&m| m == c).count() as f32 / px.len() as f32;
            let (_, s, v) = hsv(*center);
            // Black, white and grey make dull light: only when nothing else.
            let lively = s > 0.22 && v > 0.12;
            (share > 0.0).then_some((share * if lively { 0.4 + s } else { 0.05 }, *center))
        })
        .collect();
    groups.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut out: Vec<Rgb> = Vec::new();
    for (_, c) in groups {
        let (h, s, _) = hsv(c);
        let rgb = to_rgb(from_hsv(h, (s * 1.15).max(if s > 0.12 { 0.55 } else { s }), 1.0));
        // Skip near-duplicates.
        if out.iter().all(|o| dist(&to_c(*o), &to_c(rgb)) > 0.04) {
            out.push(rgb);
        }
        if out.len() == 5 {
            break;
        }
    }
    out
}

fn dist(a: &C, b: &C) -> f32 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(f: impl Fn(usize, usize) -> [u8; 3]) -> Grid {
        let (w, h) = (16, 9);
        Grid { w, h, px: (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).map(|(x, y)| f(x, y)).collect() }
    }

    #[test]
    fn vivid_colours_and_brightness_lift() {
        // A small red car on a grey road reads red, not a muddy grey.
        let g = grid(|x, y| if x < 4 && y < 3 { [220, 20, 20] } else { [90, 90, 90] });
        let c = to_rgb(screen_colors(&g, Zone::Whole, 1, false)[0]);
        assert!(c.0 as i32 - c.1 as i32 > 50, "{c:?}");
        // A dim colour gets brighter with the same hue; near-black stays.
        let dim = [0.2, 0.1, 0.0];
        let up = lift(dim, 0.6);
        assert!(up[0] > 0.6 && (up[1] / up[0] - 0.5).abs() < 1e-4 && up[2] == 0.0, "{up:?}");
        assert!((lift(dim, 1.0)[0] - 1.0).abs() < 1e-4);
        assert_eq!(lift(dim, 0.0), dim);
        assert_eq!(lift([0.01, 0.0, 0.0], 1.0), [0.01, 0.0, 0.0]);
        let c = [0.3, 0.15, 0.05];
        assert!(to_rgb(screen_tone(c, 1.0, 0.65)).0 > to_rgb(screen_tone(c, 1.0, 0.0)).0 + 60);
    }

    #[test]
    fn few_segments_keep_their_colour() {
        // Half red, a third green, the rest blue: the mix leans red, not grey.
        let g = grid(|x, _| if x < 8 { [255, 0, 0] } else if x < 13 { [0, 255, 0] } else { [0, 0, 255] });
        let c = to_rgb(screen_colors(&g, Zone::Whole, 1, false)[0]);
        assert!(c.0 as u32 > 2 * c.1 as u32 && c.0 as u32 > 2 * c.2 as u32, "{c:?}");
        // Opposite colours would mix to grey: one of them shows instead.
        let g = grid(|x, _| if x < 8 { [255, 0, 0] } else { [0, 255, 255] });
        let (_, s, _) = hsv(screen_colors(&g, Zone::Whole, 1, false)[0]);
        assert!(s > 0.8, "{s}");
        // A grey screen stays grey, and pale colours are lifted less than vivid ones.
        let g = grid(|_, _| [120, 120, 120]);
        assert!(hsv(screen_colors(&g, Zone::Whole, 1, false)[0]).1 < 0.01);
        assert!(lift([0.3, 0.27, 0.27], 0.65)[0] < lift([0.3, 0.05, 0.05], 0.65)[0] - 0.1);
    }

    #[test]
    fn pale_screen_colours_get_vivid() {
        let pale = [0.8, 0.6, 0.6];
        let (_, s0, _) = hsv(pale);
        let (_, s1, v1) = hsv(vivid(pale, 1.0));
        assert!(s1 > s0 + 0.15 && (v1 - 0.8).abs() < 1e-4, "{s0} {s1}");
        assert_eq!(vivid([0.5, 0.5, 0.52], 1.5), [0.5, 0.5, 0.52]);
    }

    #[test]
    fn music_colour_glides_on_a_beat() {
        let pal = [Rgb(255, 0, 0), Rgb(0, 0, 255)];
        let s = Sound { active: true, beats: 1, ..Default::default() };
        let mut run = MusicRun::default();
        let a = music_colors(MusicStyle::Pulse, &pal, &s, 1, 0.03, 1.0, 0.3, &mut run);
        assert!(a[0][0] > a[0][2], "{a:?}");
        for _ in 0..60 {
            music_colors(MusicStyle::Pulse, &pal, &s, 1, 0.03, 1.0, 0.3, &mut run);
        }
        let b = music_colors(MusicStyle::Pulse, &pal, &s, 1, 0.03, 1.0, 0.3, &mut run);
        assert!(b[0][2] > b[0][0], "{b:?}");
    }

    #[test]
    fn screen_zones() {
        // Left half red, right half blue.
        let g = grid(|x, _| if x < 8 { [255, 0, 0] } else { [0, 0, 255] });
        let across = screen_colors(&g, Zone::Across, 2, false);
        assert_eq!(across.iter().copied().map(to_rgb).collect::<Vec<_>>(), [Rgb(255, 0, 0), Rgb(0, 0, 255)]);
        assert_eq!(to_rgb(screen_colors(&g, Zone::Left, 3, false)[1]), Rgb(255, 0, 0));
        assert_eq!(to_rgb(screen_colors(&g, Zone::Right, 3, false)[2]), Rgb(0, 0, 255));
        let whole = to_rgb(screen_colors(&g, Zone::Whole, 1, false)[0]);
        assert_eq!(whole, Rgb(128, 0, 128));
        // Around: starts up the left (red) and ends down the right (blue).
        let around = screen_colors(&g, Zone::Around, 6, false);
        assert_eq!((to_rgb(around[0]), to_rgb(around[5])), (Rgb(255, 0, 0), Rgb(0, 0, 255)));
        // Top row green: the top edge sees green, the bottom doesn't.
        let g = grid(|_, y| if y == 0 { [0, 255, 0] } else { [10, 10, 10] });
        assert!(to_rgb(screen_colors(&g, Zone::Top, 4, false)[0]).1 > 100);
        assert!(to_rgb(screen_colors(&g, Zone::Bottom, 4, false)[0]).1 < 20);
        // Letterbox bars don't darken the picture.
        let g = grid(|_, y| if !(2..7).contains(&y) { [0, 0, 0] } else { [200, 100, 0] });
        assert_eq!(to_rgb(screen_colors(&g, Zone::Whole, 1, true)[0]), Rgb(200, 100, 0));
        assert!(to_rgb(screen_colors(&g, Zone::Whole, 1, false)[0]).0 < 185);
        // An all-black screen is black.
        assert_eq!(to_rgb(screen_colors(&grid(|_, _| [0, 0, 0]), Zone::Whole, 2, true)[1]), Rgb(0, 0, 0));
    }

    #[test]
    fn tones_and_smoothing() {
        let grey_red = to_c(Rgb(200, 120, 120));
        let vivid = to_rgb(saturate(grey_red, 2.0));
        assert!(vivid.0 == 200 && vivid.1 < 60, "{vivid:?}");
        assert_eq!(to_rgb(saturate(grey_red, 1.0)), Rgb(200, 120, 120));
        let mut s = Smooth::default();
        assert_eq!(s.step(&[[1.0, 0.0, 0.0]], 0.03, 0.1, 0.1), [Rgb(255, 0, 0)], "starts where it's told");
        let half = s.step(&[[0.0, 0.0, 0.0]], 0.1, 0.1, 0.1)[0];
        assert!(half.0 > 80 && half.0 < 110, "{half:?}");
        let mut s = Smooth::default();
        s.step(&[[0.0; 3]], 0.03, 0.0, 0.3);
        assert_eq!(s.step(&[[0.0, 1.0, 0.0]], 0.03, 0.0, 0.3), [Rgb(0, 255, 0)], "music rises at once");
    }

    #[test]
    fn music_follows_the_beat_in_the_covers_colours() {
        let pal = [Rgb(255, 0, 0), Rgb(0, 0, 255)];
        let mut run = MusicRun::default();
        let quiet = Sound { active: true, ..Default::default() };
        let a = music_colors(MusicStyle::Pulse, &pal, &quiet, 3, 0.03, 1.0, 0.0, &mut run);
        let beat = Sound { active: true, beats: 1, level: 0.8, low: 0.9, ..Default::default() };
        let b = music_colors(MusicStyle::Pulse, &pal, &beat, 3, 0.03, 1.0, 0.0, &mut run);
        // Between beats it stays dimmer, but never so low the lamp looks switched off.
        assert!(to_rgb(b[0]).2 > 200 && (60..100).contains(&to_rgb(a[0]).0), "a beat lights it in the next colour: {a:?} {b:?}");
        // Spectrum: loud bass lights the first segment, not the last.
        let mut bands = vec![0.0; 16];
        bands[0] = 1.0;
        let s = Sound { active: true, bands, ..Default::default() };
        let sp = music_colors(MusicStyle::Spectrum, &pal, &s, 4, 0.03, 1.0, 0.0, &mut MusicRun::default());
        assert!(to_rgb(sp[0]).0 > 200 && to_rgb(sp[3]).2 < 30, "{sp:?}");
        // Nothing playing: a faint glow.
        let off = music_colors(MusicStyle::Flow, &pal, &Sound::default(), 5, 0.03, 1.0, 0.0, &mut MusicRun::default());
        assert!(off.iter().all(|c| c.iter().all(|&v| v <= 0.13)));
        let v = serde_json::json!({ "level": 0.5, "low": 0.25, "beat": 1.0, "bands": [0.1, 0.2], "active": true, "beats": 7, "bpm": 120.0 });
        let snd = Sound::from_json(&v);
        assert_eq!((snd.beats, snd.bpm, snd.bands.len()), (7, Some(120.0), 2));
        assert_eq!(effect_colors(Effect::Static, &[Rgb(0, 255, 0)], 1.0, 3, 0, &Inputs::default()).len(), 3);
    }

    #[test]
    fn cover_colours() {
        // Mostly dark blue with a red patch and some grey: blue and red, lively.
        let mut px = vec![[0.05, 0.1, 0.5]; 600];
        px.extend(vec![[0.8, 0.1, 0.1]; 250]);
        px.extend(vec![[0.5, 0.5, 0.5]; 174]);
        let p = palette_of(&px);
        assert!(p.len() >= 2, "{p:?}");
        assert!(p[0].2 == 255 && p[0].0 < 60, "blue first: {p:?}");
        assert!(p.iter().any(|c| c.0 == 255 && c.2 < 60), "red: {p:?}");
        assert!(p.iter().take(2).all(|c| c.0.max(c.1).max(c.2) == 255), "full brightness");
        assert!(palette(b"not an image").is_empty());
        // A real (tiny) PNG.
        let mut img = image::RgbImage::new(4, 4);
        for p in img.pixels_mut() {
            *p = image::Rgb([20, 200, 40]);
        }
        let mut png = Vec::new();
        image::DynamicImage::ImageRgb8(img).write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
        let p = palette(&png);
        assert_eq!(p.len(), 1);
        assert!(p[0].1 == 255 && p[0].0 < 60);
    }
}
