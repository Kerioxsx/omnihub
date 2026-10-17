//! Fan profiles: Silent, Default (Auto) and Fast. Auto hands every fan
//! back to the motherboard's, graphics card's or cooler's own control;
//! Silent and Fast follow a temperature curve. Fans on the motherboard and
//! graphics card are driven through LibreHardwareMonitor (OmniHub sets
//! the duty every two seconds from the curve); liquid coolers and hubs get
//! the curve once through liquidctl and run it themselves.
//!
//! Safety comes first: no fan goes below a floor (30%, pumps 50%),
//! everything runs flat out from 90 °C, a control without a temperature to
//! follow is handed back to Auto, and quitting OmniHub hands every fan
//! back too.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FanProfile {
    Silent,
    #[default]
    Auto,
    Fast,
}

/// What a control cools, which decides its curve and its temperature.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FanRole {
    /// Case, CPU cooler and radiator fans (follow the processor).
    Case,
    /// The graphics card's own fans (follow the graphics card).
    Gpu,
    /// A liquid cooler's pump.
    Pump,
}

pub type Curve = &'static [(f32, f32)];

/// Temperature (°C) → duty (%) for a profile; `None` is Auto.
pub fn curve(profile: FanProfile, role: FanRole) -> Option<Curve> {
    use FanProfile::*;
    use FanRole::*;
    Some(match (profile, role) {
        (Auto, _) => return None,
        (Silent, Case) => &[(30.0, 30.0), (50.0, 32.0), (65.0, 45.0), (75.0, 70.0), (85.0, 100.0)],
        (Silent, Gpu) => &[(40.0, 30.0), (60.0, 35.0), (70.0, 50.0), (78.0, 75.0), (85.0, 100.0)],
        (Silent, Pump) => &[(30.0, 50.0), (60.0, 60.0), (75.0, 80.0), (85.0, 100.0)],
        (Fast, Case) => &[(30.0, 50.0), (50.0, 65.0), (65.0, 85.0), (75.0, 100.0)],
        (Fast, Gpu) => &[(40.0, 45.0), (55.0, 60.0), (65.0, 80.0), (75.0, 100.0)],
        (Fast, Pump) => &[(30.0, 80.0), (50.0, 100.0)],
    })
}

/// Liquid temperature (°C) → duty, for coolers that run their own curve.
pub fn liquid_curve(profile: FanProfile, role: FanRole) -> Curve {
    use FanProfile::*;
    use FanRole::*;
    match (profile, role) {
        (Silent, Pump) => &[(20.0, 50.0), (30.0, 55.0), (35.0, 70.0), (40.0, 85.0), (45.0, 100.0)],
        (Auto, Pump) => &[(20.0, 60.0), (30.0, 70.0), (35.0, 80.0), (40.0, 90.0), (45.0, 100.0)],
        (Fast, Pump) => &[(20.0, 80.0), (30.0, 90.0), (35.0, 100.0)],
        (Silent, _) => &[(20.0, 25.0), (30.0, 30.0), (35.0, 45.0), (40.0, 70.0), (45.0, 100.0)],
        (Auto, _) => &[(20.0, 30.0), (30.0, 40.0), (35.0, 55.0), (40.0, 80.0), (45.0, 100.0)],
        (Fast, _) => &[(20.0, 50.0), (30.0, 65.0), (35.0, 80.0), (40.0, 100.0)],
    }
}

pub const HOT: f32 = 90.0;

pub fn floor(role: FanRole) -> f32 {
    match role {
        FanRole::Pump => 50.0,
        _ => 30.0,
    }
}

/// The curve's duty at `temp` (straight lines between points).
pub fn duty(curve: Curve, temp: f32, role: FanRole) -> f32 {
    let d = if temp >= HOT {
        100.0
    } else {
        match curve.iter().position(|&(t, _)| temp < t) {
            Some(0) => curve[0].1,
            None => curve.last().map_or(100.0, |p| p.1),
            Some(i) => {
                let (t0, d0) = curve[i - 1];
                let (t1, d1) = curve[i];
                d0 + (d1 - d0) * (temp - t0) / (t1 - t0)
            }
        }
    };
    d.clamp(floor(role), 100.0)
}

/// Speed up at once, slow down gently (no hunting up and down).
pub fn smooth(previous: Option<f32>, target: f32) -> f32 {
    match previous {
        Some(p) if target < p => (p - 3.0).max(target),
        _ => target,
    }
}

/// A LibreHardwareMonitor control's role, from its id and name.
pub fn role_of(id: &str, name: &str) -> FanRole {
    if id.starts_with("/gpu") {
        FanRole::Gpu
    } else if name.to_ascii_lowercase().contains("pump") || name.to_ascii_lowercase().contains("aio") {
        FanRole::Pump
    } else {
        FanRole::Case
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curves_have_floors_and_go_flat_out_when_hot() {
        let c = curve(FanProfile::Silent, FanRole::Case).unwrap();
        assert_eq!(duty(c, 20.0, FanRole::Case), 30.0);
        assert_eq!(duty(c, 70.0, FanRole::Case), 57.5);
        assert_eq!(duty(c, 88.0, FanRole::Case), 100.0);
        assert_eq!(duty(&[(30.0, 10.0)], 20.0, FanRole::Case), 30.0, "never under the floor");
        assert_eq!(duty(&[(30.0, 10.0)], 20.0, FanRole::Pump), 50.0, "pumps keep half speed");
        assert_eq!(duty(&[(30.0, 30.0)], 95.0, FanRole::Gpu), 100.0);
        assert!(curve(FanProfile::Auto, FanRole::Gpu).is_none());
        for p in [FanProfile::Silent, FanProfile::Fast] {
            for r in [FanRole::Case, FanRole::Gpu, FanRole::Pump] {
                let c = curve(p, r).unwrap();
                assert!(c.windows(2).all(|w| w[0].0 < w[1].0 && w[0].1 <= w[1].1), "{p:?} {r:?} rises");
                assert_eq!(duty(c, 89.0, r), 100.0, "{p:?} {r:?} is at 100% before 90 °C");
            }
        }
        assert!(duty(curve(FanProfile::Fast, FanRole::Case).unwrap(), 60.0, FanRole::Case) > duty(curve(FanProfile::Silent, FanRole::Case).unwrap(), 60.0, FanRole::Case));
    }

    #[test]
    fn slows_down_gently() {
        assert_eq!(smooth(None, 40.0), 40.0);
        assert_eq!(smooth(Some(40.0), 80.0), 80.0);
        assert_eq!(smooth(Some(80.0), 40.0), 77.0);
        assert_eq!(smooth(Some(41.0), 40.0), 40.0);
        assert_eq!(role_of("/gpu-nvidia/0/control/1", "GPU Fan"), FanRole::Gpu);
        assert_eq!(role_of("/lpc/nct6799d/0/control/3", "AIO Pump"), FanRole::Pump);
        assert_eq!(role_of("/lpc/nct6799d/0/control/1", "CPU Fan"), FanRole::Case);
    }
}
