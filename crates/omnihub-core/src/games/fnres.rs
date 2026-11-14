//! Fortnite resolution advice: from the monitor, the graphics card and the
//! processor, the display resolution, 3D resolution and frame cap that
//! should hold the monitor's refresh rate in Performance mode.
//!
//! The frame estimates are rough (from public benchmarks of Performance
//! mode at 1080p, lowest settings); they pick a starting point, and the
//! FPS counter in the game is the real answer.

use super::pc::Display;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Advice {
    /// The graphics card it was worked out for ("" when unknown).
    pub gpu: String,
    pub vram_gb: f64,
    /// 0 entry (integrated) … 4 high-end.
    pub tier: u8,
    pub tier_label: String,
    /// The monitor's own resolution and refresh rate.
    pub native_width: u32,
    pub native_height: u32,
    pub hz: u32,
    /// What to set.
    pub width: u32,
    pub height: u32,
    pub resolution_scale: u8,
    /// 0 = unlimited.
    pub frame_limit: u32,
    pub lowest_quality: bool,
    /// Rough frames per second with that advice.
    pub expected_fps: u32,
    /// Why, one line each.
    pub reasons: Vec<String>,
    /// Other display resolutions worth trying, largest first (16:9 or the
    /// monitor's own shape).
    pub resolutions: Vec<(u32, u32)>,
}

/// How fast a graphics card is, from its name (and video memory when the
/// name says nothing): 0 integrated/entry … 4 high-end.
pub fn gpu_tier(name: &str, vram: u64) -> u8 {
    let n = name.to_ascii_lowercase();
    let laptop = n.contains("laptop") || n.contains("mobile") || n.contains("max-q");
    let num_after = |key: &str| -> Option<u32> {
        let i = n.find(key)? + key.len();
        let digits: String = n[i..].trim_start().chars().take_while(|c| c.is_ascii_digit()).collect();
        digits.parse().ok()
    };
    let tier = if let Some(m) = num_after("rtx").or_else(|| num_after("gtx")) {
        let (generation, class) = (m / 100, m % 100);
        match generation {
            g if g >= 40 => match class {
                c if c >= 80 => 4,
                c if c >= 60 => 3,
                _ => 2,
            },
            30 => match class {
                c if c >= 70 => 3,
                c if c >= 60 => 2,
                _ => 1,
            },
            20 => {
                if class >= 60 {
                    2
                } else {
                    1
                }
            }
            16 => 1,
            10 => match class {
                c if c >= 70 => 2,
                c if c >= 50 => 1,
                _ => 0,
            },
            _ => 1,
        }
    } else if let Some(m) = num_after("rx") {
        match m {
            9000..=9999 => {
                if m % 100 >= 70 {
                    4
                } else {
                    3
                }
            }
            1000..=8999 => {
                let (generation, class) = (m / 1000, (m % 1000) / 100);
                match (generation, class) {
                    (g, c) if g >= 7 && c >= 9 => 4,
                    (g, c) if g >= 6 && c >= 7 => 3,
                    (g, 6) if g >= 6 => 2,
                    (5, c) if c >= 6 => 2,
                    _ => 1,
                }
            }
            _ => 1, // RX 470/570/580/590
        }
    } else if n.contains("arc") && (n.contains(" b5") || n.contains(" b7") || n.contains(" a7")) {
        2
    } else if n.contains("arc") || n.contains("780m") || n.contains("880m") || n.contains("890m") || n.contains("8060s") {
        1
    } else if n.contains("intel") || n.contains("radeon(tm) graphics") || n.contains("vega") || n == "radeon graphics" {
        0
    } else {
        // Unknown name: go by video memory.
        let gb = vram / (1 << 30);
        match gb {
            g if g >= 16 => 3,
            g if g >= 8 => 2,
            g if g >= 4 => 1,
            _ => 0,
        }
    };
    if laptop && tier >= 2 {
        tier - 1
    } else {
        tier
    }
}

const TIER_LABELS: [&str; 5] = ["Entry level", "Older or budget", "Mid-range", "Fast", "High-end"];
/// Rough frames in Performance mode, lowest settings, 1080p, 100% 3D resolution.
const BASE_FPS: [f64; 5] = [70.0, 150.0, 260.0, 400.0, 600.0];
/// Fortnite's own frame cap choices.
const CAPS: [u32; 9] = [60, 120, 144, 165, 240, 360, 480, 540, 1000];
const SCALES: [u8; 6] = [100, 90, 85, 75, 67, 50];

/// What the processor can feed, roughly (Performance mode leans on it).
fn cpu_cap(threads: usize) -> f64 {
    match threads {
        t if t >= 16 => 700.0,
        t if t >= 12 => 550.0,
        t if t >= 8 => 400.0,
        t if t >= 6 => 260.0,
        0 => 400.0, // unknown
        _ => 160.0,
    }
}

fn estimate(tier: u8, threads: usize, w: u32, h: u32, scale: u8) -> f64 {
    let px = (w as f64 * h as f64 * (scale as f64 / 100.0).powi(2)).max(1.0);
    let gpu = BASE_FPS[tier.min(4) as usize] * (1920.0 * 1080.0 / px).powf(0.75);
    gpu.min(cpu_cap(threads))
}

/// Smaller display resolutions with the monitor's shape.
fn candidates(w: u32, h: u32) -> Vec<(u32, u32)> {
    let ratio = w as f64 / h.max(1) as f64;
    let mut out: Vec<(u32, u32)> = [(3840, 2160), (3440, 1440), (2560, 1440), (2560, 1080), (1920, 1080), (1680, 1050), (1600, 900), (1440, 1080), (1280, 720)]
        .into_iter()
        // Not under 1080 lines: lower looks too rough on a big monitor.
        .filter(|&(cw, ch)| cw <= w && ch <= h && ch >= h.min(1080) && ((cw as f64 / ch as f64) - ratio).abs() < 0.05)
        .collect();
    if !out.contains(&(w, h)) {
        out.insert(0, (w, h));
    }
    out
}

pub fn advise(gpu: &str, vram: u64, threads: usize, display: Option<&Display>) -> Advice {
    let tier = gpu_tier(gpu, vram);
    let (nw, nh, hz) = display.filter(|d| d.width > 0 && d.height > 0).map(|d| (d.width, d.height, d.max_hz.max(d.hz).max(60))).unwrap_or((1920, 1080, 60));
    let mut reasons = Vec::new();
    let mut pick = (nw, nh, 100u8);
    let mut fps = estimate(tier, threads, nw, nh, 100);
    let target = hz as f64;
    if fps < target {
        // First the 3D resolution (the menus and text stay sharp), not below
        // 75% at 1080p where it gets blurry, then a smaller display resolution.
        let floor = |h: u32| if h > 1080 { 67 } else { 75 };
        let mut found = false;
        'outer: for (w, h) in candidates(nw, nh) {
            let floor = floor(h);
            for s in SCALES.into_iter().filter(|&s| s >= floor) {
                let f = estimate(tier, threads, w, h, s);
                if f >= target {
                    pick = (w, h, s);
                    fps = f;
                    found = true;
                    break 'outer;
                }
            }
        }
        if !found {
            // Can't reach the refresh rate: the lightest sensible setup.
            let (w, h) = if nw > 1920 && candidates(nw, nh).contains(&(1920, 1080)) { (1920, 1080) } else { (nw, nh) };
            pick = (w, h, floor(h));
            fps = estimate(tier, threads, w, h, floor(h));
        }
    }
    let (w, h, scale) = pick;
    if (w, h) != (nw, nh) {
        reasons.push(format!("{w}×{h} instead of {nw}×{nh}: fewer pixels for the card to draw. Needs Fullscreen mode (windowed fullscreen always uses the desktop’s resolution)."));
    } else {
        reasons.push(format!("Your monitor’s own {nw}×{nh}: the sharpest picture."));
    }
    if scale < 100 {
        reasons.push(format!("3D resolution {scale}%: the world is drawn smaller and scaled up, the menus and text stay sharp."));
    }
    // Cap a little under what it can hold, never below the refresh rate.
    let holdable = fps * 0.9;
    let frame_limit = if holdable >= 1000.0 {
        0
    } else {
        CAPS.iter().rev().copied().find(|&c| c as f64 <= holdable && c >= hz).or_else(|| CAPS.iter().rev().copied().find(|&c| c as f64 <= holdable)).unwrap_or(60)
    };
    if frame_limit == 0 {
        reasons.push("No frame cap: your PC can go far beyond any cap.".into());
    } else if frame_limit >= hz {
        reasons.push(format!("Frame cap {frame_limit}: a little under what it can hold, for steady frame times and less input delay; at or above your {hz} Hz."));
    } else {
        reasons.push(format!("Frame cap {frame_limit}: what it can hold steadily. Your monitor goes to {hz} Hz, but a steady cap beats a jumpy uncapped rate."));
    }
    let lowest_quality = tier <= 3;
    reasons.push(if lowest_quality { "Performance mode, lowest quality: the most frames, what most competitive players use.".into() } else { "Performance mode: the most frames; your card has room for medium view distance.".into() });
    if vram > 0 && vram < 3 * (1 << 30) {
        reasons.push("Under 3 GB of video memory: keep textures low to avoid stutters.".into());
    }
    if threads > 0 && threads < 8 {
        reasons.push(format!("{threads} processor threads: the processor likely limits frames; closing background apps helps more than lower resolution."));
    }
    let mut resolutions = candidates(nw, nh);
    resolutions.truncate(6);
    Advice {
        gpu: gpu.to_string(),
        vram_gb: (vram as f64 / (1u64 << 30) as f64 * 10.0).round() / 10.0,
        tier,
        tier_label: TIER_LABELS[tier.min(4) as usize].into(),
        native_width: nw,
        native_height: nh,
        hz,
        width: w,
        height: h,
        resolution_scale: scale,
        frame_limit,
        lowest_quality,
        expected_fps: (fps / 10.0).round() as u32 * 10,
        reasons,
        resolutions,
    }
}

/// Advice for this PC.
pub fn advise_here() -> Advice {
    let m = super::pc::machine();
    let (gpu, vram) = crate::system::gpu::main_adapter().unwrap_or_else(|| (m.gpus.first().cloned().unwrap_or_default(), 0));
    advise(&gpu, vram, m.threads, m.displays.first())
}

#[cfg(test)]
mod tests {
    use super::*;
    const GB: u64 = 1 << 30;

    fn d(w: u32, h: u32, hz: u32) -> Display {
        Display { name: "x".into(), width: w, height: h, hz, max_hz: hz }
    }

    #[test]
    fn tiers_from_names() {
        assert_eq!(gpu_tier("NVIDIA GeForce RTX 4090", 24 * GB), 4);
        assert_eq!(gpu_tier("NVIDIA GeForce RTX 4060", 8 * GB), 3);
        assert_eq!(gpu_tier("NVIDIA GeForce RTX 3060", 12 * GB), 2);
        assert_eq!(gpu_tier("NVIDIA GeForce RTX 3060 Laptop GPU", 6 * GB), 1);
        assert_eq!(gpu_tier("NVIDIA GeForce GTX 1650", 4 * GB), 1);
        assert_eq!(gpu_tier("NVIDIA GeForce GTX 1080 Ti", 11 * GB), 2);
        assert_eq!(gpu_tier("AMD Radeon RX 7900 XTX", 24 * GB), 4);
        assert_eq!(gpu_tier("AMD Radeon RX 9070 XT", 16 * GB), 4);
        assert_eq!(gpu_tier("AMD Radeon RX 6600", 8 * GB), 2);
        assert_eq!(gpu_tier("Radeon RX 580 Series", 8 * GB), 1);
        assert_eq!(gpu_tier("Intel(R) UHD Graphics 620", 128 << 20), 0);
        assert_eq!(gpu_tier("Intel(R) Arc(TM) B580 Graphics", 12 * GB), 2);
        assert_eq!(gpu_tier("Something New", 16 * GB), 3);
    }

    #[test]
    fn fast_pc_keeps_native() {
        let a = advise("NVIDIA GeForce RTX 4080", 16 * GB, 16, Some(&d(2560, 1440, 165)));
        assert_eq!((a.width, a.height, a.resolution_scale), (2560, 1440, 100));
        assert!(a.frame_limit == 0 || a.frame_limit >= 165, "{}", a.frame_limit);
    }

    #[test]
    fn slow_pc_on_fast_monitor_lowers_resolution() {
        let a = advise("NVIDIA GeForce GTX 1650", 4 * GB, 8, Some(&d(2560, 1440, 240)));
        assert!(a.resolution_scale < 100 || a.width < 2560, "{a:?}");
        assert!(a.frame_limit > 0 && a.frame_limit <= a.expected_fps.max(60), "{a:?}");
    }

    #[test]
    fn integrated_at_1080p_60() {
        let a = advise("Intel(R) UHD Graphics", 0, 8, Some(&d(1920, 1080, 60)));
        assert_eq!((a.width, a.height), (1920, 1080));
        assert!(a.resolution_scale >= 75);
        assert!(a.frame_limit >= 60);
    }

    #[test]
    fn no_display_falls_back() {
        let a = advise("", 0, 0, None);
        assert_eq!((a.native_width, a.hz), (1920, 60));
        assert!(!a.reasons.is_empty());
    }
}
