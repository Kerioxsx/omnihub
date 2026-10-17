//! Lighting effects and profiles. The same maths runs here (for the real
//! LEDs, through OpenRGB) and in the window's 3D view
//! (`src/desktop/lib/rig/effects.ts`), so the preview is what the PC shows.

use serde::{Deserialize, Serialize};

use super::openrgb::Rgb;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Effect {
    Off,
    /// One colour, or the colours spread across each device.
    #[default]
    Static,
    /// Fading in and out, a colour per breath.
    Breathing,
    /// The whole PC through every hue.
    Spectrum,
    /// A rainbow running along the LEDs.
    Wave,
    /// The colours flowing slowly along the LEDs.
    Gradient,
    /// Pulsing with the music the PC plays.
    Music,
    /// Cool to hot with the processor's temperature.
    Temperature,
}

impl Effect {
    /// Changes from frame to frame (needs the effect loop running).
    pub fn animated(self) -> bool {
        !matches!(self, Effect::Off | Effect::Static | Effect::Temperature)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct LightProfile {
    pub id: String,
    pub name: String,
    pub effect: Effect,
    /// "#rrggbb".
    pub colors: Vec<String>,
    /// 0.2 (slow) to 3 (fast).
    pub speed: f32,
    /// 0–100.
    pub brightness: u8,
}

impl Default for LightProfile {
    fn default() -> Self {
        LightProfile { id: String::new(), name: "My lighting".into(), effect: Effect::Static, colors: vec!["#00a2ff".into()], speed: 1.0, brightness: 100 }
    }
}

/// The profiles OmniHub comes with (kept, and restorable, by id).
pub fn built_in() -> Vec<LightProfile> {
    let p = |id: &str, name: &str, effect: Effect, colors: &[&str], speed: f32| LightProfile { id: id.into(), name: name.into(), effect, colors: colors.iter().map(|c| c.to_string()).collect(), speed, brightness: 100 };
    vec![
        p("aurora", "Aurora", Effect::Music, &["#7c3aed", "#00d4ff", "#ff2bd6"], 1.0),
        p("ice", "Ice", Effect::Static, &["#00a2ff", "#e0f7ff"], 1.0),
        p("rainbow", "Rainbow wave", Effect::Wave, &[], 1.0),
        p("spectrum", "Spectrum", Effect::Spectrum, &[], 0.6),
        p("ember", "Ember", Effect::Breathing, &["#ff3b00", "#ff9500"], 0.7),
        p("sunset", "Sunset", Effect::Gradient, &["#ff4d6d", "#ffb703", "#7b2cbf"], 1.0),
        p("heat", "Temperature", Effect::Temperature, &["#00a2ff", "#00ff9c", "#ffd000", "#ff2a00"], 1.0),
        p("white", "Clean white", Effect::Static, &["#ffffff"], 1.0),
        p("off", "Lights off", Effect::Off, &[], 1.0),
    ]
}

/// What an effect reacts to.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Inputs {
    /// Seconds since the effect started.
    pub t: f32,
    /// Music: overall level and beat, 0–1.
    pub level: f32,
    pub beat: f32,
    /// Processor temperature, °C.
    pub temp: Option<f32>,
}

fn lerp(a: Rgb, b: Rgb, k: f32) -> Rgb {
    let k = k.clamp(0.0, 1.0);
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * k).round() as u8;
    Rgb(m(a.0, b.0), m(a.1, b.1), m(a.2, b.2))
}

fn scale(c: Rgb, k: f32) -> Rgb {
    let k = k.clamp(0.0, 1.0);
    let m = |x: u8| (x as f32 * k).round() as u8;
    Rgb(m(c.0), m(c.1), m(c.2))
}

/// Hue (0–1) at full saturation and value.
pub fn hue(h: f32) -> Rgb {
    let h = h.rem_euclid(1.0) * 6.0;
    let x = 1.0 - (h % 2.0 - 1.0).abs();
    let (r, g, b) = match h as u32 {
        0 => (1.0, x, 0.0),
        1 => (x, 1.0, 0.0),
        2 => (0.0, 1.0, x),
        3 => (0.0, x, 1.0),
        4 => (x, 0.0, 1.0),
        _ => (1.0, 0.0, x),
    };
    let v = |c: f32| (c * 255.0).round() as u8;
    Rgb(v(r), v(g), v(b))
}

/// A point (0–1) along the colours, end to end (`wrap`: back to the first).
fn along(colors: &[Rgb], x: f32, wrap: bool) -> Rgb {
    match colors.len() {
        0 => Rgb(255, 255, 255),
        1 => colors[0],
        n => {
            let stops = if wrap { n } else { n - 1 };
            let p = if wrap { x.rem_euclid(1.0) } else { x.clamp(0.0, 1.0) } * stops as f32;
            let i = (p.floor() as usize).min(stops - 1);
            lerp(colors[i], colors[(i + 1) % n], p - i as f32)
        }
    }
}

pub fn palette(p: &LightProfile) -> Vec<Rgb> {
    p.colors.iter().filter_map(|c| Rgb::parse(c)).collect()
}

/// The colour of the LED at `x` (0–1 along its device) on device `dev`.
pub fn color_at(p: &LightProfile, colors: &[Rgb], x: f32, dev: usize, inp: &Inputs) -> Rgb {
    let speed = p.speed.clamp(0.2, 3.0);
    let t = inp.t * speed;
    let c = match p.effect {
        Effect::Off => Rgb(0, 0, 0),
        Effect::Static => along(colors, x, false),
        Effect::Breathing => {
            let phase = t * 0.4;
            let k = 0.06 + 0.94 * (0.5 - 0.5 * (std::f32::consts::TAU * phase).cos());
            let n = colors.len().max(1);
            let base = colors.get(phase.floor() as usize % n).copied().unwrap_or(Rgb(255, 255, 255));
            scale(base, k)
        }
        Effect::Spectrum => hue(t * 0.08),
        Effect::Wave => hue(x * 0.8 + dev as f32 * 0.13 - t * 0.25),
        Effect::Gradient => along(colors, x * 0.5 + dev as f32 * 0.17 + t * 0.05, true),
        Effect::Music => {
            let k = (0.12 + inp.level * 1.1 + inp.beat * 0.6).min(1.0);
            let base = along(colors, x * 0.6 + dev as f32 * 0.2 + t * 0.03, true);
            let flash = along(colors, x * 0.6 + dev as f32 * 0.2 + t * 0.03 + 0.5, true);
            scale(lerp(base, flash, inp.beat), k)
        }
        Effect::Temperature => {
            let temp = inp.temp.unwrap_or(40.0);
            along(colors, (temp - 35.0) / 50.0, false)
        }
    };
    scale(c, p.brightness.min(100) as f32 / 100.0)
}

/// Colours for a device with `leds` LEDs.
pub fn frame(p: &LightProfile, colors: &[Rgb], leds: usize, dev: usize, inp: &Inputs) -> Vec<Rgb> {
    (0..leds).map(|i| color_at(p, colors, if leds > 1 { i as f32 / (leds - 1) as f32 } else { 0.0 }, dev, inp)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(effect: Effect, colors: &[&str]) -> LightProfile {
        LightProfile { effect, colors: colors.iter().map(|c| c.to_string()).collect(), ..Default::default() }
    }

    /// The window's copy (`src/desktop/lib/rig/effects.ts`) gives the same values.
    #[test]
    fn effects_match_the_preview() {
        let red_blue = profile(Effect::Static, &["#ff0000", "#0000ff"]);
        let c = palette(&red_blue);
        let at = |p: &LightProfile, x: f32, t: f32| color_at(p, &palette(p), x, 0, &Inputs { t, ..Default::default() });
        assert_eq!(at(&red_blue, 0.0, 0.0), Rgb(255, 0, 0));
        assert_eq!(at(&red_blue, 0.5, 0.0), Rgb(128, 0, 128));
        assert_eq!(at(&red_blue, 1.0, 0.0), Rgb(0, 0, 255));
        assert_eq!(frame(&red_blue, &c, 3, 0, &Inputs::default()), vec![Rgb(255, 0, 0), Rgb(128, 0, 128), Rgb(0, 0, 255)]);
        assert_eq!(hue(0.0), Rgb(255, 0, 0));
        assert_eq!(hue(1.0 / 3.0), Rgb(0, 255, 0));
        let breathe = profile(Effect::Breathing, &["#ff0000"]);
        assert_eq!(at(&breathe, 0.0, 0.0), Rgb(15, 0, 0), "starts dim");
        assert_eq!(at(&breathe, 0.0, 1.25), Rgb(255, 0, 0), "full half a breath later");
        let heat = profile(Effect::Temperature, &["#0000ff", "#ff0000"]);
        assert_eq!(color_at(&heat, &palette(&heat), 0.0, 0, &Inputs { temp: Some(35.0), ..Default::default() }), Rgb(0, 0, 255));
        assert_eq!(color_at(&heat, &palette(&heat), 0.0, 0, &Inputs { temp: Some(95.0), ..Default::default() }), Rgb(255, 0, 0));
        let off = profile(Effect::Off, &[]);
        assert_eq!(at(&off, 0.3, 2.0), Rgb(0, 0, 0));
        let dim = LightProfile { brightness: 50, ..profile(Effect::Static, &["#ffffff"]) };
        assert_eq!(at(&dim, 0.0, 0.0), Rgb(128, 128, 128));
        let music = profile(Effect::Music, &["#ff0000"]);
        let quiet = color_at(&music, &palette(&music), 0.0, 0, &Inputs::default());
        let loud = color_at(&music, &palette(&music), 0.0, 0, &Inputs { level: 0.8, beat: 1.0, ..Default::default() });
        assert!(quiet.0 < 40 && loud.0 == 255, "{quiet:?} {loud:?}");
        assert!(built_in().iter().all(|p| !p.id.is_empty()));
    }
}
