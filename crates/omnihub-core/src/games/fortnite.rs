//! Fortnite helpers beyond its settings file: old replays, the settings an
//! FPS record was taken with, Wi-Fi power saving, and how old the graphics driver is (with a nudge to
//! clear the shader caches after it changes).

use std::path::{Path, PathBuf};

use serde::Serialize;

use super::ini::Doc;

// ---------- replays ----------

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Replay {
    pub name: String,
    pub bytes: u64,
    /// Unix seconds.
    pub at: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Replays {
    pub dir: Option<String>,
    pub list: Vec<Replay>,
    pub bytes: u64,
}

/// Where Fortnite keeps replays (`%LOCALAPPDATA%\FortniteGame\Saved\Demos`).
pub fn replays_dir(local: &Path) -> PathBuf {
    local.join(r"FortniteGame\Saved\Demos")
}

fn mtime(p: &Path) -> i64 {
    std::fs::metadata(p).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs() as i64)
}

/// Replays, newest first.
pub fn list_replays(dir: &Path) -> Replays {
    let mut list: Vec<Replay> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("replay")))
        .map(|e| Replay { name: e.file_name().to_string_lossy().into_owned(), bytes: e.metadata().map(|m| m.len()).unwrap_or(0), at: mtime(&e.path()) })
        .collect();
    list.sort_by_key(|r| std::cmp::Reverse(r.at));
    let bytes = list.iter().map(|r| r.bytes).sum();
    Replays { dir: dir.is_dir().then(|| dir.to_string_lossy().into_owned()), list, bytes }
}

/// The replays to remove: all but the newest `keep`, and of those only ones
/// older than `days` (when given).
pub fn replays_to_clean(list: &[Replay], keep: usize, days: Option<u32>, now: i64) -> Vec<&Replay> {
    list.iter().skip(keep).filter(|r| days.is_none_or(|d| now - r.at > d as i64 * 86_400)).collect()
}

/// Send old replays to the Recycle Bin; returns how many and their size.
pub fn clean_replays(dir: &Path, keep: usize, days: Option<u32>, now: i64) -> Result<(usize, u64), String> {
    let all = list_replays(dir);
    let mut n = 0;
    let mut bytes = 0;
    for r in replays_to_clean(&all.list, keep, days, now) {
        let p = dir.join(&r.name);
        if trash::delete(&p).is_ok() {
            n += 1;
            bytes += r.bytes;
        }
    }
    Ok((n, bytes))
}

// ---------- the settings an FPS record was taken with ----------

/// A short description of Fortnite's current video settings, read from its
/// own file ("Performance · 240 FPS cap · view near · 100%").
pub fn settings_label(doc: &Doc) -> Option<String> {
    let s = Some("/Script/FortniteGame.FortGameUserSettings");
    let g = Some("ScalabilityGroups");
    let rhi = Some("D3DRHIPreference");
    let mode = match (doc.get(rhi, "PreferredFeatureLevel").as_deref(), doc.get(rhi, "PreferredRHI").as_deref()) {
        (Some(l), _) if l.eq_ignore_ascii_case("es31") => "Performance",
        (_, Some(r)) if r.eq_ignore_ascii_case("dx12") => "DirectX 12",
        (_, Some(r)) if r.eq_ignore_ascii_case("dx11") => "DirectX 11",
        _ => "Default renderer",
    };
    let cap = doc.get(s, "FrameRateLimit").and_then(|v| v.parse::<f32>().ok()).map(|f| f.round() as u32);
    let cap = match cap {
        Some(0) | None => "no FPS cap".to_string(),
        Some(c) => format!("{c} FPS cap"),
    };
    let quality = doc.get(g, "sg.ShadowQuality").and_then(|v| v.parse::<u8>().ok()).map(|q| ["low", "medium", "high", "epic", "cinematic"][q.min(4) as usize]);
    let res = doc.get(g, "sg.ResolutionQuality").and_then(|v| v.parse::<f32>().ok()).map(|r| format!("{}%", r.round() as u32));
    let mut parts = vec![mode.to_string(), cap];
    if let Some(q) = quality {
        parts.push(format!("{q} quality"));
    }
    if let Some(r) = res {
        parts.push(r);
    }
    Some(parts.join(" · "))
}

/// The label for the first of `paths` that reads.
pub fn settings_label_from(paths: &[PathBuf]) -> Option<String> {
    paths.iter().find_map(|p| std::fs::read(p).ok()).and_then(|b| settings_label(&Doc::ini(&b)))
}

// ---------- Wi-Fi power saving ----------

/// Windows' "Wireless Adapter Settings → Power Saving Mode" in the power plan.
pub const WIFI_SUBGROUP: &str = "19cbb8fa-5279-450e-9fac-8a3d5fedd0c1";
pub const WIFI_SETTING: &str = "12bbebe6-58d6-4636-95bb-3217ef867c1a";

/// The plugged-in and battery values from `powercfg /query` (its words are
/// translated; the two "0x…" numbers at the end are not).
pub fn parse_power_values(out: &str) -> Option<(u32, u32)> {
    let hex: Vec<u32> = out.split_whitespace().filter_map(|w| w.strip_prefix("0x")).filter(|h| h.len() == 8).filter_map(|h| u32::from_str_radix(h, 16).ok()).collect();
    match hex.as_slice() {
        [.., ac, dc] => Some((*ac, *dc)),
        _ => None,
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NetCheck {
    /// Connected over Wi-Fi (None: couldn't tell).
    pub on_wifi: Option<bool>,
    /// Wi-Fi power saving while plugged in: 0 maximum performance … 3 maximum saving.
    pub wifi_saving: Option<u32>,
}

// ---------- graphics driver ----------

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpuDriver {
    pub name: String,
    pub version: String,
    /// Unix seconds, when Windows knows.
    pub date: Option<i64>,
    pub days_old: Option<i64>,
    /// Where to get a newer one.
    pub update_url: Option<String>,
    /// Changed since the shader caches were last cleaned here: a good time to clean them.
    pub changed: bool,
}

pub fn update_url(name: &str) -> Option<&'static str> {
    let n = name.to_lowercase();
    if n.contains("nvidia") || n.contains("geforce") || n.contains("rtx") || n.contains("gtx") {
        Some("https://www.nvidia.com/en-us/geforce/drivers/")
    } else if n.contains("amd") || n.contains("radeon") {
        Some("https://www.amd.com/en/support/download/drivers.html")
    } else if n.contains("intel") || n.contains("arc") {
        Some("https://www.intel.com/content/www/us/en/support/detect.html")
    } else {
        None
    }
}

/// Graphics drivers, the main card first (integrated graphics last).
pub fn gpu_drivers(seen_file: &Path, now: i64) -> Vec<GpuDriver> {
    let seen = std::fs::read_to_string(seen_file).unwrap_or_default();
    let mut out: Vec<GpuDriver> = imp::display_drivers()
        .into_iter()
        .map(|(name, version, date)| {
            let days_old = date.map(|d| (now - d).max(0) / 86_400);
            let changed = !seen.trim().is_empty() && !seen.lines().any(|l| l.trim() == version);
            GpuDriver { update_url: update_url(&name).map(String::from), name, version, date, days_old, changed }
        })
        .collect();
    out.sort_by_key(|d| {
        let n = d.name.to_lowercase();
        (n.contains("microsoft basic") as u8, (n.contains("intel") && !n.contains("arc")) as u8)
    });
    // First look: remember these versions without calling them "changed".
    if seen.trim().is_empty() && !out.is_empty() {
        remember_drivers(seen_file, &out);
    }
    out
}

/// Remember the drivers as they are now (after the shader caches were cleaned).
pub fn remember_drivers(seen_file: &Path, drivers: &[GpuDriver]) {
    let list: Vec<&str> = drivers.iter().map(|d| d.version.as_str()).collect();
    let _ = std::fs::write(seen_file, list.join("\n"));
}

#[cfg(windows)]
mod imp {
    use windows::Win32::Devices::DeviceAndDriverInstallation::{SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo, SetupDiGetClassDevsW, SetupDiGetDevicePropertyW, DIGCF_PRESENT, HDEVINFO, SP_DEVINFO_DATA};
    use windows::Win32::Devices::Properties::{DEVPKEY_Device_DeviceDesc, DEVPKEY_Device_DriverDate, DEVPKEY_Device_DriverVersion, DEVPROPTYPE};
    use windows::Win32::Foundation::DEVPROPKEY;

    /// The "Display adapters" class.
    const DISPLAY: windows::core::GUID = windows::core::GUID::from_u128(0x4d36e968_e325_11ce_bfc1_08002be10318);

    fn prop(set: HDEVINFO, data: &SP_DEVINFO_DATA, key: &DEVPROPKEY) -> Option<Vec<u8>> {
        let mut ty = DEVPROPTYPE::default();
        let mut buf = vec![0u8; 512];
        let mut need = 0u32;
        unsafe { SetupDiGetDevicePropertyW(set, data, key, &mut ty, Some(&mut buf), Some(&mut need), 0) }.ok()?;
        buf.truncate(need as usize);
        Some(buf)
    }

    fn wide(b: &[u8]) -> String {
        let u: Vec<u16> = b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&c| c != 0).collect();
        String::from_utf16_lossy(&u)
    }

    /// (name, driver version, driver date as unix seconds)
    pub fn display_drivers() -> Vec<(String, String, Option<i64>)> {
        let Ok(set) = (unsafe { SetupDiGetClassDevsW(Some(&DISPLAY), None, None, DIGCF_PRESENT) }) else { return Vec::new() };
        let mut out = Vec::new();
        for i in 0..32u32 {
            let mut data = SP_DEVINFO_DATA { cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32, ..Default::default() };
            if unsafe { SetupDiEnumDeviceInfo(set, i, &mut data) }.is_err() {
                break;
            }
            let name = prop(set, &data, &DEVPKEY_Device_DeviceDesc).map(|b| wide(&b)).unwrap_or_default();
            let version = prop(set, &data, &DEVPKEY_Device_DriverVersion).map(|b| wide(&b)).unwrap_or_default();
            // A FILETIME: 100 ns steps since 1601.
            let date = prop(set, &data, &DEVPKEY_Device_DriverDate).filter(|b| b.len() >= 8).map(|b| {
                let ft = u64::from_le_bytes(b[..8].try_into().unwrap_or_default());
                (ft / 10_000_000) as i64 - 11_644_473_600
            });
            if !name.is_empty() {
                out.push((name, version, date));
            }
        }
        let _ = unsafe { SetupDiDestroyDeviceInfoList(set) };
        out
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn display_drivers() -> Vec<(String, String, Option<i64>)> {
        Vec::new()
    }
}

// ---------- crashes ----------

/// Fortnite's last crash, read from its own crash reports, with the likely
/// cause in plain words.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Crash {
    /// "crash" (the game closed) or "kick" (thrown out of a match).
    pub kind: String,
    /// When (milliseconds since 1970).
    pub at: i64,
    /// Fortnite's own message (shortened).
    pub message: String,
    /// gpu | vram | memory | files | textures | other
    pub cause: String,
    pub title: String,
    pub advice: String,
}

fn tag(xml: &str, name: &str) -> Option<String> {
    let open = format!("<{name}>");
    let a = xml.find(&open)? + open.len();
    let b = xml[a..].find(&format!("</{name}>"))? + a;
    let t = xml[a..b].replace("&apos;", "'").replace("&quot;", "\"").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&");
    Some(t.trim().to_string()).filter(|t| !t.is_empty())
}

fn mtime_ms(p: &Path) -> Option<i64> {
    let t = std::fs::metadata(p).ok()?.modified().ok()?;
    Some(t.duration_since(std::time::UNIX_EPOCH).ok()?.as_millis() as i64)
}

/// What a crash message most likely means. `textures_out` is when OmniHub
/// had Fortnite's optional texture packs out (start, end in ms).
pub fn classify(message: &str, at: i64, textures_out: Option<(i64, i64)>) -> (String, String, String) {
    let m = message.to_ascii_lowercase();
    let has = |ws: &[&str]| ws.iter().any(|w| m.contains(w));
    let (cause, title, advice) = if textures_out.is_some_and(|(a, b)| at >= a - 60_000 && at <= b + 60_000) {
        ("textures", "Fortnite crashed while its texture packs were out", "Parts of the game can still ask for the optional texture packs, so playing without them can crash after a while. They are back now. Don’t use Remove & play for now; for flat textures use Potato textures (NVIDIA) or the Potato preset instead.")
    } else if has(&["out of video memory", "video memory", "outofvideomemory"]) {
        ("vram", "The graphics card ran out of memory", "Lower Textures and View distance (or use the Potato preset), close other programs that use the graphics card (browsers, recording), and cap the frame rate.")
    } else if has(&["device removed", "device_removed", "device hung", "device_hung", "gpu crash", "dxgi_error", "d3d12", "d3d11", "nvwgf", "nvlddmkm", "atidxx", "amdxx", "igdumd", "gpu has crashed", "d3ddevicelost"]) {
        ("gpu", "The graphics driver stopped responding", "Usually heat or an unstable overclock while the card runs flat out at very high frame rates, or the driver itself. Cap the frame rate near your monitor’s refresh rate, set Potato textures to Off to rule it out, undo any graphics overclock, and clean-install the latest driver.")
    } else if has(&["out of memory", "ran out of memory", "outofmemory", "failed to allocate", "malloc"]) {
        ("memory", "Windows ran out of memory", "Close browsers and other programs while you play, and make sure Windows’ page file is on (System → About → Advanced system settings → Performance → Advanced → Virtual memory: automatic).")
    } else if has(&[".pak", ".utoc", ".ucas", "pakchunk", "corrupt", "missing", "couldn't find", "could not find", "failed to load", "signature", "pak file"]) {
        ("files", "Fortnite couldn’t read some of its files", "If texture packs are out, put them back. Then run Verify in the Epic Games launcher (Library → Fortnite → ⋯ → Manage → Verify).")
    } else {
        ("other", "Fortnite crashed", "If it keeps happening, cap the frame rate, set Potato textures to Off, put your own settings back (Put back mine) and see whether it stops; that tells which change it was.")
    };
    (cause.into(), title.into(), advice.into())
}

/// The newest crash report under `<local>\FortniteGame\Saved\Crashes`.
pub fn last_crash(local: &Path, textures_out: Option<(i64, i64)>) -> Option<Crash> {
    let dir = local.join("FortniteGame").join("Saved").join("Crashes");
    let (at, message) = std::fs::read_dir(&dir)
        .ok()?
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| {
            let f = e.path().join("CrashContext.runtime-xml");
            let at = mtime_ms(&f)?;
            let bytes = std::fs::read(&f).ok()?;
            let xml = String::from_utf8_lossy(&bytes[..bytes.len().min(256 * 1024)]).into_owned();
            let msg = tag(&xml, "ErrorMessage").or_else(|| tag(&xml, "CrashType")).unwrap_or_else(|| "No message".into());
            Some((at, msg))
        })
        .max_by_key(|(at, _)| *at)?;
    let message: String = message.lines().next().unwrap_or("").chars().take(300).collect();
    let (cause, title, advice) = classify(&message, at, textures_out);
    Some(Crash { kind: "crash".into(), at, message, cause, title, advice })
}

/// `[2026.11.15-20.31.12:345]` at the start of a log line, in ms (UTC).
fn log_time(line: &str) -> Option<i64> {
    let t = line.strip_prefix('[')?.get(..23)?;
    let d = chrono::NaiveDateTime::parse_from_str(t, "%Y.%m.%d-%H.%M.%S:%3f").ok()?;
    Some(d.and_utc().timestamp_millis())
}

/// What a disconnect line most likely means.
pub fn classify_kick(line: &str, at: i64, textures_out: Option<(i64, i64)>) -> (String, String, String) {
    let m = line.to_ascii_lowercase();
    let has = |ws: &[&str]| ws.iter().any(|w| m.contains(w));
    let (cause, title, advice) = if textures_out.is_some_and(|(a, b)| at >= a - 60_000 && at <= b + 60_000) {
        ("textures", "Fortnite threw you out while its texture packs were out", "Fortnite’s anti-cheat checks the game’s files during a match, and missing texture packs can get you removed. They are back now, and OmniHub no longer offers taking them out. For flat textures use Potato textures (NVIDIA) or the Potato preset.")
    } else if has(&["anticheat", "anti-cheat", "easyanticheat", "battleye", "integrity", "untrusted", "violation"]) {
        ("anticheat", "Fortnite’s anti-cheat removed you from the match", "Usually a game file it didn’t expect or a program it doesn’t trust. Put texture packs back if they are out, run Verify in the Epic Games launcher (Library → Fortnite → ⋯ → Manage → Verify), and close overlays, macro tools and auto clickers while you play.")
    } else if has(&["package", "mismatch", "checksum", "missing", ".pak", "pakchunk"]) {
        ("files", "Your game files didn’t match the server’s", "Put texture packs back if they are out, let the Epic Games launcher finish any update, then run Verify (Library → Fortnite → ⋯ → Manage → Verify).")
    } else if has(&["idle", "afk", "inactiv"]) {
        ("idle", "Fortnite removed you for being idle", "Fortnite kicks players who don’t move for a while.")
    } else if has(&["timeout", "timed out", "connectionlost", "connection lost", "pendingconnectionfailure", "connection to the host"]) {
        ("network", "Your connection to the match dropped", "Your PC stopped hearing from Fortnite’s server for a few seconds. On Wi-Fi, move closer to the router or use a cable, pause downloads and streams on your network, and try turning off Wi-Fi low-latency mode in the boost. Check the ping and packet loss test in Games.")
    } else {
        ("other", "Fortnite took you out of the match", "If it keeps happening, put your own settings back (Put back mine) and see whether it stops; that tells whether a change was the cause.")
    };
    (cause.into(), title.into(), advice.into())
}

/// The last time Fortnite's own log says you were disconnected or kicked
/// from a match, in its newest logs (`Saved\Logs\FortniteGame*.log`).
pub fn last_kick(local: &Path, textures_out: Option<(i64, i64)>) -> Option<Crash> {
    let dir = local.join("FortniteGame").join("Saved").join("Logs");
    let mut logs: Vec<(i64, PathBuf)> = std::fs::read_dir(&dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("FortniteGame") && n.ends_with(".log")))
        .filter_map(|p| Some((mtime_ms(&p)?, p)))
        .collect();
    logs.sort_by_key(|l| std::cmp::Reverse(l.0));
    for (mtime, path) in logs.into_iter().take(4) {
        let Ok(bytes) = std::fs::read(&path) else { continue };
        // The end of a long log is enough.
        let text = String::from_utf8_lossy(&bytes[bytes.len().saturating_sub(16 << 20)..]).into_owned();
        let hit = text.lines().rev().find(|l| {
            let m = l.to_ascii_lowercase();
            ["network failure", "networkfailure", "travel failure", "travelfailure", "kicked", "kick player", "anticheat", "battleye"].iter().any(|w| m.contains(w)) && (m.contains("warning") || m.contains("error") || m.contains("kick") || m.contains("disconnect"))
        });
        if let Some(line) = hit {
            let at = log_time(line).unwrap_or(mtime);
            let message: String = line.chars().take(300).collect();
            let (cause, title, advice) = classify_kick(line, at, textures_out);
            return Some(Crash { kind: "kick".into(), at, message, cause, title, advice });
        }
    }
    None
}

/// The most recent crash or kick.
pub fn last_problem(local: &Path, textures_out: Option<(i64, i64)>) -> Option<Crash> {
    [last_crash(local, textures_out), last_kick(local, textures_out)].into_iter().flatten().max_by_key(|c| c.at)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replays_keep_the_newest_and_clean_old_ones() {
        let list: Vec<Replay> = (0..6).map(|i| Replay { name: format!("r{i}.replay"), bytes: 100, at: 1_000_000 - i * 86_400 }).collect();
        let now = 1_000_000;
        assert_eq!(replays_to_clean(&list, 2, None, now).len(), 4);
        let old = replays_to_clean(&list, 2, Some(3), now);
        assert_eq!(old.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), vec!["r4.replay", "r5.replay"], "only those over 3 days old");

        let t = tempfile::tempdir().unwrap();
        std::fs::write(t.path().join("a.replay"), vec![0u8; 10]).unwrap();
        std::fs::write(t.path().join("notes.txt"), b"x").unwrap();
        let r = list_replays(t.path());
        assert_eq!((r.list.len(), r.bytes), (1, 10));
    }

    #[test]
    fn reads_the_settings_label() {
        let doc = Doc::ini(b"[/Script/FortniteGame.FortGameUserSettings]\nFrameRateLimit=240.000000\n[ScalabilityGroups]\nsg.ResolutionQuality=100.000000\nsg.ShadowQuality=0\n[D3DRHIPreference]\nPreferredFeatureLevel=es31\n");
        assert_eq!(settings_label(&doc).as_deref(), Some("Performance · 240 FPS cap · low quality · 100%"));
        let doc = Doc::ini(b"[D3DRHIPreference]\nPreferredRHI=dx12\n[/Script/FortniteGame.FortGameUserSettings]\nFrameRateLimit=0.000000\n");
        assert_eq!(settings_label(&doc).as_deref(), Some("DirectX 12 · no FPS cap"));
    }

    #[test]
    fn power_values_and_driver_links() {
        let out = "Power Setting GUID: 12bbebe6-58d6-4636-95bb-3217ef867c1a  (Power Saving Mode)\n  Possible Setting Index: 000\n  Possible Setting Index: 003\nCurrent AC Power Setting Index: 0x00000000\nCurrent DC Power Setting Index: 0x00000002\n";
        assert_eq!(parse_power_values(out), Some((0, 2)));
        assert_eq!(parse_power_values("nothing"), None);
        assert!(update_url("NVIDIA GeForce RTX 4070").unwrap().contains("nvidia"));
        assert!(update_url("AMD Radeon RX 7800 XT").unwrap().contains("amd"));
        assert_eq!(update_url("Microsoft Basic Display Adapter"), None);
    }

    #[test]
    fn reads_and_explains_the_last_crash() {
        let t = tempfile::tempdir().unwrap();
        let d = t.path().join("FortniteGame/Saved/Crashes/UECC-Windows-1_0000");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("CrashContext.runtime-xml"), "<FGenericCrashContext><RuntimeProperties><ErrorMessage>LowLevelFatalError [File:Unknown] D3D Device Removed: DXGI_ERROR_DEVICE_HUNG</ErrorMessage><CrashType>GPUCrash</CrashType></RuntimeProperties></FGenericCrashContext>").unwrap();
        let c = last_crash(t.path(), None).unwrap();
        assert_eq!(c.cause, "gpu");
        assert!(c.message.contains("DEVICE_HUNG"));
        let c = last_crash(t.path(), Some((c.at - 1000, c.at + 1000))).unwrap();
        assert_eq!(c.cause, "textures", "a crash while the packs were out points at them");
        assert_eq!(classify("Ran out of memory allocating 12 bytes", 0, None).0, "memory");
        assert_eq!(classify("Corrupt pak file pakchunk10optional", 0, None).0, "files");
        assert_eq!(classify("Unhandled Exception: EXCEPTION_ACCESS_VIOLATION", 0, None).0, "other");
        assert!(last_crash(tempfile::tempdir().unwrap().path(), None).is_none());
    }

    #[test]
    fn finds_the_last_kick_in_the_log() {
        let t = tempfile::tempdir().unwrap();
        let d = t.path().join("FortniteGame/Saved/Logs");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("FortniteGame.log"), "[2026.11.15-20.00.00:000][  1]LogInit: Build\n[2026.11.15-20.31.12:345][900]LogNet: Warning: Network Failure: GameNetDriver[ConnectionTimeout]: UNetConnection::Tick: Connection TIMED OUT\n[2026.11.15-20.31.13:000][901]LogLoad: Took 1s\n").unwrap();
        let k = last_kick(t.path(), None).unwrap();
        assert_eq!((k.kind.as_str(), k.cause.as_str()), ("kick", "network"));
        assert_eq!(k.at, chrono::NaiveDate::from_ymd_opt(2026, 11, 15).unwrap().and_hms_milli_opt(20, 31, 12, 345).unwrap().and_utc().timestamp_millis());
        assert_eq!(last_kick(t.path(), Some((k.at - 5000, k.at + 5000))).unwrap().cause, "textures");
        assert_eq!(classify_kick("LogEasyAntiCheatClient: Error: Kicked: Integrity violation", 0, None).0, "anticheat");
        assert_eq!(classify_kick("LogNet: Warning: Travel Failure: [PackageMismatch]", 0, None).0, "files");
        assert_eq!(last_problem(t.path(), None).unwrap().kind, "kick");
        assert!(last_kick(tempfile::tempdir().unwrap().path(), None).is_none());
    }
}
