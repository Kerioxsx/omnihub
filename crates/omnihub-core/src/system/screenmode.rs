//! The main screen's mode (resolution and refresh rate), HDR on every
//! screen that has it, and a warmer picture at night (each screen's gamma
//! ramp, the way f.lux does it). Mode changes are made for this session
//! only (not saved by Windows), so a restart always brings back your own.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "camelCase")]
pub struct Mode {
    pub width: u32,
    pub height: u32,
    pub hz: u32,
}

/// Red, green and blue strengths (0–1) for a colour temperature, from
/// 6500 K (as is) down to 1900 K (candlelight).
pub fn warmth(kelvin: u32) -> (f32, f32, f32) {
    let k = kelvin.clamp(1900, 6500) as f32;
    let t = (6500.0 - k) / (6500.0 - 1900.0);
    // Blue goes first, green follows, red stays (close to f.lux's curve).
    (1.0, 1.0 - 0.38 * t, 1.0 - 0.78 * t.powf(0.9))
}

/// A gamma ramp (256 steps per colour) with these strengths and brightness (0.3–1).
pub fn ramp(rgb: (f32, f32, f32), brightness: f32) -> [[u16; 256]; 3] {
    let b = brightness.clamp(0.3, 1.0);
    let mut r = [[0u16; 256]; 3];
    for (c, k) in [rgb.0, rgb.1, rgb.2].into_iter().enumerate() {
        for (i, x) in r[c].iter_mut().enumerate() {
            *x = (i as f32 / 255.0 * 65535.0 * b * k) as u16;
        }
    }
    r
}

pub fn current() -> Option<Mode> {
    imp::current()
}

/// The main screen's modes, highest first.
pub fn modes() -> Vec<Mode> {
    let mut m = imp::modes();
    m.sort_by_key(|b| std::cmp::Reverse((b.width * b.height, b.hz)));
    m.dedup();
    m
}

pub fn set(m: Mode) -> Result<(), String> {
    let all = modes();
    if !all.contains(&m) {
        return Err(format!("Your screen doesn't offer {}×{} at {} Hz", m.width, m.height, m.hz));
    }
    // Already there: nothing to do (no flicker).
    if current() == Some(m) {
        return Ok(());
    }
    let native = all.first().map(|n| (n.width, n.height)) == Some((m.width, m.height));
    imp::set(m, !native)
}

/// The best mode for a resolution: the current refresh rate if the screen
/// offers it there (the quickest switch), else the highest one.
pub fn pick(modes: &[Mode], width: u32, height: u32, prefer_hz: u32) -> Option<Mode> {
    let at: Vec<&Mode> = modes.iter().filter(|m| m.width == width && m.height == height).collect();
    at.iter().find(|m| m.hz == prefer_hz).or_else(|| at.iter().max_by_key(|m| m.hz)).map(|m| **m)
}

/// Sets a resolution at the best refresh rate for it (see [`pick`]).
pub fn set_resolution(width: u32, height: u32) -> Result<Mode, String> {
    let now = current().map(|m| m.hz).unwrap_or(0);
    let m = pick(&modes(), width, height, now).ok_or_else(|| format!("Your screen doesn't offer {width}×{height}"))?;
    set(m)?;
    Ok(m)
}

/// A friendly name for well-known resolutions ("1440p (2K)").
pub fn label(width: u32, height: u32) -> Option<&'static str> {
    Some(match (width, height) {
        (3840, 2160) => "4K",
        (3440, 1440) => "Ultrawide 1440p",
        (2560, 1440) => "1440p (2K)",
        (2560, 1080) => "Ultrawide 1080p",
        (1920, 1080) => "1080p",
        (1600, 900) => "900p",
        (1280, 720) => "720p",
        (1920, 1440) => "Stretched 1440 (4:3)",
        (1440, 1080) => "Stretched 1080 (4:3)",
        (1280, 960) => "Stretched 960 (4:3)",
        (1680, 1050) => "1050p (16:10)",
        _ => return None,
    })
}

/// Whether HDR is on (None: no screen supports it).
pub fn hdr() -> Option<bool> {
    imp::hdr()
}

pub fn set_hdr(on: bool) -> Result<(), String> {
    imp::set_hdr(on)
}

/// Every screen's colour warmth (6500 = normal) and brightness.
pub fn set_warmth(kelvin: u32, brightness: f32) -> Result<(), String> {
    imp::set_ramp(&ramp(warmth(kelvin), brightness))
}

#[cfg(windows)]
mod imp {
    use super::Mode;
    use windows::core::PCWSTR;
    use windows::Win32::Devices::Display::{DisplayConfigGetDeviceInfo, DisplayConfigSetDeviceInfo, GetDisplayConfigBufferSizes, QueryDisplayConfig, DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO, DISPLAYCONFIG_DEVICE_INFO_HEADER, DISPLAYCONFIG_DEVICE_INFO_SET_ADVANCED_COLOR_STATE, DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO, DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_SET_ADVANCED_COLOR_STATE, QDC_ONLY_ACTIVE_PATHS};
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::Graphics::Gdi::{ChangeDisplaySettingsExW, CreateDCW, DeleteDC, EnumDisplayDevicesW, EnumDisplaySettingsW, CDS_TYPE, DEVMODEW, DISPLAY_DEVICEW, DISPLAY_DEVICE_ATTACHED_TO_DESKTOP, DISP_CHANGE_SUCCESSFUL, DMDFO_DEFAULT, DMDFO_STRETCH, DM_BITSPERPEL, DM_DISPLAYFIXEDOUTPUT, DM_DISPLAYFREQUENCY, DM_PELSHEIGHT, DM_PELSWIDTH, ENUM_CURRENT_SETTINGS, ENUM_DISPLAY_SETTINGS_MODE};
    use windows::Win32::UI::ColorSystem::SetDeviceGammaRamp;

    fn devmode() -> DEVMODEW {
        DEVMODEW { dmSize: std::mem::size_of::<DEVMODEW>() as u16, ..Default::default() }
    }

    pub fn current() -> Option<Mode> {
        let mut d = devmode();
        unsafe { EnumDisplaySettingsW(PCWSTR::null(), ENUM_CURRENT_SETTINGS, &mut d) }.as_bool().then_some(Mode { width: d.dmPelsWidth, height: d.dmPelsHeight, hz: d.dmDisplayFrequency })
    }

    pub fn modes() -> Vec<Mode> {
        let mut out = Vec::new();
        let bits = {
            let mut d = devmode();
            let _ = unsafe { EnumDisplaySettingsW(PCWSTR::null(), ENUM_CURRENT_SETTINGS, &mut d) };
            d.dmBitsPerPel
        };
        for i in 0..2000 {
            let mut d = devmode();
            if !unsafe { EnumDisplaySettingsW(PCWSTR::null(), ENUM_DISPLAY_SETTINGS_MODE(i), &mut d) }.as_bool() {
                break;
            }
            if d.dmBitsPerPel == bits && d.dmPelsWidth >= 800 && d.dmDisplayFrequency > 1 {
                out.push(Mode { width: d.dmPelsWidth, height: d.dmPelsHeight, hz: d.dmDisplayFrequency });
            }
        }
        out
    }

    /// `scaled`: below the native resolution, ask the graphics card to
    /// scale the picture (the monitor keeps its own signal, so there's no
    /// black re-sync and the switch is near instant). Falls back to a plain
    /// mode change if the driver refuses.
    pub fn set(m: Mode, scaled: bool) -> Result<(), String> {
        let mut d = devmode();
        // Start from the current mode: same colour depth and the rest, so only the size changes.
        let _ = unsafe { EnumDisplaySettingsW(PCWSTR::null(), ENUM_CURRENT_SETTINGS, &mut d) };
        d.dmPelsWidth = m.width;
        d.dmPelsHeight = m.height;
        d.dmDisplayFrequency = m.hz;
        d.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_DISPLAYFREQUENCY | DM_BITSPERPEL;
        let mut fixed = d;
        // Writing a union field is safe; ask the graphics card to scale (no monitor re-sync).
        fixed.Anonymous1.Anonymous2.dmDisplayFixedOutput = if scaled { DMDFO_STRETCH } else { DMDFO_DEFAULT };
        fixed.dmFields |= DM_DISPLAYFIXEDOUTPUT;
        // For this session only: Windows keeps your own mode for next time.
        let mut r = unsafe { ChangeDisplaySettingsExW(PCWSTR::null(), Some(&fixed), None, CDS_TYPE(0), None) };
        if r != DISP_CHANGE_SUCCESSFUL {
            r = unsafe { ChangeDisplaySettingsExW(PCWSTR::null(), Some(&d), None, CDS_TYPE(0), None) };
        }
        if r == DISP_CHANGE_SUCCESSFUL {
            Ok(())
        } else {
            Err(format!("Windows didn't switch the screen (code {})", r.0))
        }
    }

    fn targets() -> Vec<DISPLAYCONFIG_PATH_INFO> {
        unsafe {
            let (mut np, mut nm) = (0u32, 0u32);
            if GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut np, &mut nm) != ERROR_SUCCESS {
                return Vec::new();
            }
            let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); np as usize];
            let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); nm as usize];
            if QueryDisplayConfig(QDC_ONLY_ACTIVE_PATHS, &mut np, paths.as_mut_ptr(), &mut nm, modes.as_mut_ptr(), None) != ERROR_SUCCESS {
                return Vec::new();
            }
            paths.truncate(np as usize);
            paths
        }
    }

    /// (supported, on) for one screen.
    fn color_info(p: &DISPLAYCONFIG_PATH_INFO) -> Option<(bool, bool)> {
        let mut info = DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO {
            header: DISPLAYCONFIG_DEVICE_INFO_HEADER { r#type: DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO, size: std::mem::size_of::<DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO>() as u32, adapterId: p.targetInfo.adapterId, id: p.targetInfo.id },
            ..Default::default()
        };
        if unsafe { DisplayConfigGetDeviceInfo(&mut info.header) } != 0 {
            return None;
        }
        let v = unsafe { info.Anonymous.value };
        // Bit 0: advancedColorSupported, bit 1: advancedColorEnabled.
        Some((v & 1 != 0, v & 2 != 0))
    }

    pub fn hdr() -> Option<bool> {
        let s: Vec<(bool, bool)> = targets().iter().filter_map(color_info).filter(|c| c.0).collect();
        (!s.is_empty()).then(|| s.iter().any(|c| c.1))
    }

    pub fn set_hdr(on: bool) -> Result<(), String> {
        let mut any = false;
        for p in targets() {
            if !color_info(&p).is_some_and(|c| c.0) {
                continue;
            }
            any = true;
            let mut st = DISPLAYCONFIG_SET_ADVANCED_COLOR_STATE {
                header: DISPLAYCONFIG_DEVICE_INFO_HEADER { r#type: DISPLAYCONFIG_DEVICE_INFO_SET_ADVANCED_COLOR_STATE, size: std::mem::size_of::<DISPLAYCONFIG_SET_ADVANCED_COLOR_STATE>() as u32, adapterId: p.targetInfo.adapterId, id: p.targetInfo.id },
                ..Default::default()
            };
            st.Anonymous.value = u32::from(on);
            if unsafe { DisplayConfigSetDeviceInfo(&st.header) } != 0 {
                return Err("Windows didn't change HDR".into());
            }
        }
        if any {
            Ok(())
        } else {
            Err("None of your screens supports HDR".into())
        }
    }

    pub fn set_ramp(r: &[[u16; 256]; 3]) -> Result<(), String> {
        let mut done = 0;
        for i in 0..16 {
            let mut dd = DISPLAY_DEVICEW { cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32, ..Default::default() };
            if !unsafe { EnumDisplayDevicesW(PCWSTR::null(), i, &mut dd, 0) }.as_bool() {
                break;
            }
            if !dd.StateFlags.contains(DISPLAY_DEVICE_ATTACHED_TO_DESKTOP) {
                continue;
            }
            unsafe {
                let dc = CreateDCW(PCWSTR(dd.DeviceName.as_ptr()), PCWSTR(dd.DeviceName.as_ptr()), PCWSTR::null(), None);
                if dc.is_invalid() {
                    continue;
                }
                if SetDeviceGammaRamp(dc, r.as_ptr().cast()).as_bool() {
                    done += 1;
                }
                let _ = DeleteDC(dc);
            }
        }
        if done > 0 {
            Ok(())
        } else {
            Err("Windows didn't change the screen colours (some drivers don't allow it)".into())
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::Mode;
    pub fn current() -> Option<Mode> {
        None
    }
    pub fn modes() -> Vec<Mode> {
        Vec::new()
    }
    pub fn set(_m: Mode, _scaled: bool) -> Result<(), String> {
        Err("Screen modes are changed on Windows".into())
    }
    pub fn hdr() -> Option<bool> {
        None
    }
    pub fn set_hdr(_on: bool) -> Result<(), String> {
        Err("HDR is changed on Windows".into())
    }
    pub fn set_ramp(_r: &[[u16; 256]; 3]) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_keeps_the_refresh_rate_when_it_can() {
        let m = |w, h, hz| Mode { width: w, height: h, hz };
        let modes = [m(2560, 1440, 165), m(2560, 1440, 60), m(1920, 1080, 165), m(1920, 1080, 144), m(1920, 1080, 60), m(1280, 720, 60)];
        assert_eq!(pick(&modes, 1920, 1080, 165), Some(m(1920, 1080, 165)));
        assert_eq!(pick(&modes, 1920, 1080, 120), Some(m(1920, 1080, 165)), "else the highest");
        assert_eq!(pick(&modes, 1280, 720, 165), Some(m(1280, 720, 60)));
        assert_eq!(pick(&modes, 3840, 2160, 165), None);
        assert_eq!(label(2560, 1440), Some("1440p (2K)"));
        assert_eq!(label(3840, 2160), Some("4K"));
    }

    #[test]
    fn warmer_takes_blue_first() {
        assert_eq!(warmth(6500), (1.0, 1.0, 1.0));
        let (r, g, b) = warmth(3400);
        assert!(r == 1.0 && g < 1.0 && b < g, "{r} {g} {b}");
        assert_eq!(warmth(100), warmth(1900), "clamped");
        let rp = ramp(warmth(6500), 1.0);
        assert_eq!((rp[0][0], rp[0][255], rp[2][255]), (0, 65535, 65535));
        assert_eq!(ramp(warmth(6500), 0.0)[0][255], ramp(warmth(6500), 0.3)[0][255], "never darker than 30%");
    }
}
