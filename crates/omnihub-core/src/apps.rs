//! Installed applications: desktop programs from the registry, Store apps,
//! and everything pinned in the Start menu, with icons, sizes and launch.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AppSource {
    /// Win32 program with an uninstall entry.
    Desktop,
    /// Microsoft Store / MSIX package.
    Store,
    /// Only known from a Start menu shortcut.
    StartMenu,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub id: String,
    pub name: String,
    pub publisher: String,
    pub version: String,
    pub source: AppSource,
    pub install_location: Option<String>,
    /// Unix seconds, when known.
    pub install_date: Option<i64>,
    /// Bytes: from the latest storage scan when it covers the install
    /// folder, otherwise the size the installer registered.
    pub size: Option<u64>,
    pub size_from_scan: bool,
    /// Application user model id, used to launch through the shell.
    pub aumid: Option<String>,
    pub launchable: bool,
    pub uninstallable: bool,
    /// Where the icon comes from (shell parsing name or file path).
    #[serde(skip)]
    pub icon_source: Option<String>,
    #[serde(skip)]
    pub uninstall_command: Option<String>,
}

pub fn app_id(key: &str) -> String {
    hex::encode(&blake3::hash(key.to_lowercase().as_bytes()).as_bytes()[..10])
}

/// Turn a registry `InstallDate` (YYYYMMDD) into Unix seconds.
pub fn parse_install_date(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.len() != 8 || !s.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let d = chrono::NaiveDate::parse_from_str(s, "%Y%m%d").ok()?;
    Some(d.and_hms_opt(12, 0, 0)?.and_utc().timestamp())
}

/// Split a `DisplayIcon` value like `"C:\x\app.exe",0` into the file path.
pub fn icon_path(display_icon: &str) -> Option<String> {
    let s = display_icon.trim();
    let s = if let Some(rest) = s.strip_prefix('"') { rest.split('"').next()? } else { s.rsplit_once(',').map_or(s, |(p, idx)| if idx.trim().trim_start_matches('-').chars().all(|c| c.is_ascii_digit()) { p } else { s }) };
    let s = s.trim();
    (!s.is_empty()).then(|| s.to_string())
}

/// Split an uninstall command into program and arguments.
pub fn split_command(cmd: &str) -> Option<(String, String)> {
    let cmd = cmd.trim();
    if let Some(rest) = cmd.strip_prefix('"') {
        let (exe, args) = rest.split_once('"')?;
        return Some((exe.to_string(), args.trim().to_string()));
    }
    let lower = cmd.to_lowercase();
    if let Some(i) = lower.find(".exe") {
        let (exe, args) = cmd.split_at(i + 4);
        return Some((exe.trim().to_string(), args.trim().to_string()));
    }
    let mut parts = cmd.splitn(2, ' ');
    Some((parts.next()?.to_string(), parts.next().unwrap_or("").to_string()))
}

fn normalise(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
}

/// Merge Start menu entries with registry and package entries by name.
pub fn merge(registry: Vec<AppInfo>, packages: Vec<AppInfo>, start: Vec<(String, String)>) -> Vec<AppInfo> {
    let mut out: Vec<AppInfo> = Vec::new();
    let mut by_family: HashMap<String, AppInfo> = HashMap::new();
    for p in packages {
        if let Some(a) = &p.aumid {
            by_family.insert(a.to_lowercase(), p);
        }
    }
    let mut registry = registry;
    for (name, aumid) in start {
        if name.trim().is_empty() || aumid.trim().is_empty() {
            continue;
        }
        let lower = name.to_lowercase();
        // Skip uninstall / help / readme shortcuts.
        if lower.starts_with("uninstall") || lower.contains("uninstall ") || lower.ends_with(" help") || lower.contains("readme") || lower.contains("release notes") {
            continue;
        }
        let shell_name = format!("shell:AppsFolder\\{aumid}");
        if let Some((family, _)) = aumid.split_once('!') {
            let mut app = by_family.remove(&family.to_lowercase()).unwrap_or_else(|| AppInfo {
                id: String::new(),
                name: String::new(),
                publisher: String::new(),
                version: String::new(),
                source: AppSource::Store,
                install_location: None,
                install_date: None,
                size: None,
                size_from_scan: false,
                aumid: None,
                launchable: true,
                uninstallable: true,
                icon_source: None,
                uninstall_command: None,
            });
            app.id = app_id(&aumid);
            app.name = name;
            app.aumid = Some(aumid);
            app.launchable = true;
            app.icon_source = Some(shell_name);
            out.push(app);
            continue;
        }
        let key = normalise(&name);
        let matched = registry.iter().position(|r| {
            let rk = normalise(&r.name);
            !rk.is_empty() && (rk == key || (key.len() >= 4 && rk.starts_with(&key)) || (rk.len() >= 4 && key.starts_with(&rk)))
        });
        let mut app = match matched {
            Some(i) => registry.remove(i),
            None => AppInfo {
                id: String::new(),
                name: name.clone(),
                publisher: String::new(),
                version: String::new(),
                source: AppSource::StartMenu,
                install_location: None,
                install_date: None,
                size: None,
                size_from_scan: false,
                aumid: None,
                launchable: true,
                uninstallable: false,
                icon_source: None,
                uninstall_command: None,
            },
        };
        app.id = app_id(&aumid);
        app.name = name;
        app.aumid = Some(aumid);
        app.launchable = true;
        app.icon_source = Some(shell_name);
        out.push(app);
    }
    // Programs that are installed but have no Start entry.
    for mut r in registry {
        r.id = app_id(&format!("reg:{}", r.name));
        out.push(r);
    }
    out.sort_by_key(|a| a.name.to_lowercase());
    out.dedup_by(|a, b| a.id == b.id);
    out
}

#[cfg(windows)]
mod win {
    use super::*;
    use std::os::windows::process::CommandExt;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY};
    use winreg::RegKey;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    pub fn registry_apps() -> Vec<AppInfo> {
        let mut out = Vec::new();
        let path = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
        let roots = [
            (RegKey::predef(HKEY_LOCAL_MACHINE), KEY_READ | KEY_WOW64_64KEY),
            (RegKey::predef(HKEY_LOCAL_MACHINE), KEY_READ | KEY_WOW64_32KEY),
            (RegKey::predef(HKEY_CURRENT_USER), KEY_READ),
        ];
        let mut seen = std::collections::HashSet::new();
        for (root, flags) in roots {
            let Ok(key) = root.open_subkey_with_flags(path, flags) else { continue };
            for sub in key.enum_keys().flatten() {
                let Ok(k) = key.open_subkey_with_flags(&sub, flags) else { continue };
                let get = |n: &str| k.get_value::<String, _>(n).unwrap_or_default();
                let name = get("DisplayName");
                if name.trim().is_empty() {
                    continue;
                }
                if k.get_value::<u32, _>("SystemComponent").unwrap_or(0) == 1 || !get("ParentKeyName").is_empty() {
                    continue;
                }
                let release = get("ReleaseType").to_lowercase();
                if release.contains("update") || release.contains("hotfix") {
                    continue;
                }
                if !seen.insert(name.to_lowercase()) {
                    continue;
                }
                let uninstall = Some(get("UninstallString")).filter(|s| !s.is_empty());
                let location = Some(get("InstallLocation").trim_matches('"').trim_end_matches('\\').to_string()).filter(|s| !s.is_empty());
                let icon = icon_path(&get("DisplayIcon")).filter(|p| !p.to_lowercase().contains("unins"));
                out.push(AppInfo {
                    id: String::new(),
                    name: name.trim().to_string(),
                    publisher: get("Publisher").trim().to_string(),
                    version: get("DisplayVersion").trim().to_string(),
                    source: AppSource::Desktop,
                    install_location: location,
                    install_date: parse_install_date(&get("InstallDate")),
                    size: k.get_value::<u32, _>("EstimatedSize").ok().map(|kb| kb as u64 * 1024),
                    size_from_scan: false,
                    aumid: None,
                    launchable: false,
                    uninstallable: uninstall.is_some(),
                    icon_source: icon,
                    uninstall_command: uninstall,
                });
            }
        }
        out
    }

    fn powershell_json(script: &str) -> Option<serde_json::Value> {
        let full = format!("[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; {script}");
        let out = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", &full])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&out.stdout);
        let v: serde_json::Value = serde_json::from_str(text.trim()).ok()?;
        // A single object is not wrapped in an array by ConvertTo-Json.
        Some(if v.is_array() { v } else { serde_json::Value::Array(vec![v]) })
    }

    pub fn start_apps() -> Vec<(String, String)> {
        powershell_json("Get-StartApps | Select-Object Name, AppID | ConvertTo-Json -Compress")
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|o| Some((o.get("Name")?.as_str()?.to_string(), o.get("AppID")?.as_str()?.to_string())))
            .collect()
    }

    pub fn packages() -> Vec<AppInfo> {
        let script = "Get-AppxPackage -PackageTypeFilter Main | Where-Object { -not $_.IsFramework -and $_.SignatureKind -ne 'System' } | Select-Object Name, PackageFamilyName, InstallLocation, Version, Publisher | ConvertTo-Json -Compress";
        powershell_json(script)
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|o| {
                let family = o.get("PackageFamilyName")?.as_str()?.to_string();
                let publisher = o.get("Publisher").and_then(|p| p.as_str()).unwrap_or("");
                let publisher = publisher.split(',').next().unwrap_or("").trim_start_matches("CN=").trim_matches('"').to_string();
                Some(AppInfo {
                    id: String::new(),
                    name: o.get("Name").and_then(|n| n.as_str()).unwrap_or("").to_string(),
                    publisher,
                    version: o.get("Version").and_then(|n| n.as_str()).unwrap_or("").to_string(),
                    source: AppSource::Store,
                    install_location: o.get("InstallLocation").and_then(|n| n.as_str()).map(str::to_string),
                    install_date: None,
                    size: None,
                    size_from_scan: false,
                    aumid: Some(family),
                    launchable: true,
                    uninstallable: true,
                    icon_source: None,
                    uninstall_command: None,
                })
            })
            .collect()
    }

    /// Render the shell icon of a parsing name (`shell:AppsFolder\...` or a
    /// file path) as PNG bytes.
    pub fn icon_png(source: &str, size: i32) -> Option<Vec<u8>> {
        use windows::core::HSTRING;
        use windows::Win32::Foundation::SIZE;
        use windows::Win32::Graphics::Gdi::{CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS};
        use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
        use windows::Win32::UI::Shell::{IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_BIGGERSIZEOK, SIIGBF_ICONONLY};
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let factory: IShellItemImageFactory = SHCreateItemFromParsingName(&HSTRING::from(source), None).ok()?;
            let hbm = factory.GetImage(SIZE { cx: size, cy: size }, SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK).ok()?;
            let mut bm = BITMAP::default();
            if GetObjectW(hbm.into(), std::mem::size_of::<BITMAP>() as i32, Some(&mut bm as *mut _ as *mut _)) == 0 {
                let _ = DeleteObject(hbm.into());
                return None;
            }
            let (w, h) = (bm.bmWidth, bm.bmHeight.abs());
            let mut info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w,
                    biHeight: -h,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut px = vec![0u8; (w * h * 4) as usize];
            let dc = CreateCompatibleDC(None);
            let lines = GetDIBits(dc, hbm, 0, h as u32, Some(px.as_mut_ptr() as *mut _), &mut info, DIB_RGB_COLORS);
            let _ = DeleteDC(dc);
            let _ = DeleteObject(hbm.into());
            if lines == 0 {
                return None;
            }
            bgra_to_png(&mut px, w as u32, h as u32)
        }
    }

    pub fn launch(aumid: &str) -> std::io::Result<()> {
        std::process::Command::new("explorer.exe").arg(format!("shell:AppsFolder\\{aumid}")).spawn().map(|_| ())
    }

    pub fn run_uninstaller(app: &AppInfo) -> std::io::Result<()> {
        if app.source == AppSource::Store {
            return std::process::Command::new("explorer.exe").arg("ms-settings:appsfeatures").spawn().map(|_| ());
        }
        let cmd = app.uninstall_command.as_deref().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no uninstaller registered"))?;
        let (exe, args) = split_command(cmd).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "bad uninstall command"))?;
        crate::system::shell::shell_execute(&exe, &args)
    }
}

/// BGRA (possibly premultiplied) pixels to a PNG.
pub fn bgra_to_png(px: &mut [u8], w: u32, h: u32) -> Option<Vec<u8>> {
    let any_alpha = px.chunks_exact(4).any(|p| p[3] != 0);
    let premultiplied = any_alpha && px.chunks_exact(4).all(|p| p[0] <= p[3] && p[1] <= p[3] && p[2] <= p[3]);
    for p in px.chunks_exact_mut(4) {
        p.swap(0, 2);
        if !any_alpha {
            p[3] = 255;
        } else if premultiplied && p[3] != 0 && p[3] != 255 {
            let a = p[3] as u32;
            for c in &mut p[..3] {
                *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
    let img = image::RgbaImage::from_raw(w, h, px.to_vec())?;
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png).ok()?;
    Some(out.into_inner())
}

#[cfg(windows)]
fn render_icon(app: &AppInfo) -> Option<Vec<u8>> {
    win::icon_png(app.icon_source.as_deref()?, 64)
}

#[cfg(not(windows))]
fn render_icon(_app: &AppInfo) -> Option<Vec<u8>> {
    None
}

/// The app catalogue with an icon cache on disk.
pub struct AppLibrary {
    cache_dir: PathBuf,
    apps: Mutex<Option<Vec<AppInfo>>>,
}

impl AppLibrary {
    pub fn new(cache_dir: &Path) -> Self {
        let dir = cache_dir.join("icons");
        let _ = std::fs::create_dir_all(&dir);
        AppLibrary { cache_dir: dir, apps: Mutex::new(None) }
    }

    /// Enumerate (slow: runs PowerShell); cached until `refresh`.
    pub fn list(&self, size_of: &dyn Fn(&str) -> Option<u64>) -> Vec<AppInfo> {
        let mut guard = self.apps.lock();
        if guard.is_none() {
            *guard = Some(Self::enumerate());
        }
        let mut apps = guard.clone().unwrap_or_default();
        for a in &mut apps {
            if let Some(loc) = &a.install_location {
                if let Some(s) = size_of(loc) {
                    a.size = Some(s);
                    a.size_from_scan = true;
                }
            }
        }
        apps
    }

    pub fn refresh(&self) {
        *self.apps.lock() = None;
    }

    #[cfg(windows)]
    fn enumerate() -> Vec<AppInfo> {
        let (registry, (packages, start)) = rayon::join(win::registry_apps, || rayon::join(win::packages, win::start_apps));
        merge(registry, packages, start)
    }

    #[cfg(not(windows))]
    fn enumerate() -> Vec<AppInfo> {
        Vec::new()
    }

    pub fn get(&self, id: &str) -> Option<AppInfo> {
        self.apps.lock().as_ref()?.iter().find(|a| a.id == id).cloned()
    }

    /// PNG icon as a `data:` URL, cached on disk.
    pub fn icon_data_url(&self, id: &str) -> Option<String> {
        use base64::Engine;
        let app = self.get(id)?;
        let file = self.cache_dir.join(format!("{id}.png"));
        let png = match std::fs::read(&file) {
            Ok(b) => b,
            Err(_) => {
                let png = render_icon(&app)?;
                let _ = std::fs::write(&file, &png);
                png
            }
        };
        Some(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png)))
    }

    pub fn launch(&self, id: &str) -> std::io::Result<String> {
        let app = self.get(id).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "unknown app"))?;
        let aumid = app.aumid.as_deref().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Unsupported, "this program has no Start menu entry to launch"))?;
        #[cfg(windows)]
        win::launch(aumid)?;
        #[cfg(not(windows))]
        let _ = aumid;
        Ok(app.name)
    }

    pub fn uninstall(&self, id: &str) -> std::io::Result<String> {
        let app = self.get(id).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "unknown app"))?;
        #[cfg(windows)]
        {
            win::run_uninstaller(&app)?;
            Ok(app.name)
        }
        #[cfg(not(windows))]
        Err(std::io::Error::new(std::io::ErrorKind::Unsupported, format!("cannot uninstall {} here", app.name)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reg(name: &str) -> AppInfo {
        AppInfo {
            id: String::new(),
            name: name.into(),
            publisher: "Pub".into(),
            version: "1.0".into(),
            source: AppSource::Desktop,
            install_location: Some(format!("C:\\Program Files\\{name}")),
            install_date: None,
            size: Some(1000),
            size_from_scan: false,
            aumid: None,
            launchable: false,
            uninstallable: true,
            icon_source: None,
            uninstall_command: None,
        }
    }

    #[test]
    fn merging_start_registry_and_store() {
        let mut pkg = reg("Microsoft.WindowsCalculator");
        pkg.source = AppSource::Store;
        pkg.aumid = Some("Microsoft.WindowsCalculator_8wekyb3d8bbwe".into());
        let apps = merge(
            vec![reg("Mozilla Firefox (x64 en-US)"), reg("7-Zip 23.01"), reg("Visual C++ Runtime")],
            vec![pkg],
            vec![
                ("Firefox".into(), "308046B0AF4A39CB".into()),
                ("Mozilla Firefox".into(), "Mozilla.Firefox".into()),
                ("Calculator".into(), "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App".into()),
                ("Uninstall 7-Zip".into(), "x".into()),
                ("Notepad".into(), "{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\\notepad.exe".into()),
            ],
        );
        let names: Vec<&str> = apps.iter().map(|a| a.name.as_str()).collect();
        assert!(names.contains(&"Calculator"));
        assert!(names.contains(&"Notepad"));
        assert!(!names.iter().any(|n| n.starts_with("Uninstall")));
        let calc = apps.iter().find(|a| a.name == "Calculator").unwrap();
        assert_eq!(calc.source, AppSource::Store);
        assert!(calc.launchable);
        let ff = apps.iter().find(|a| a.name == "Mozilla Firefox").unwrap();
        assert_eq!(ff.source, AppSource::Desktop, "matched to its registry entry");
        assert_eq!(ff.publisher, "Pub");
        let zip = apps.iter().find(|a| a.name.starts_with("7-Zip")).unwrap();
        assert!(!zip.launchable, "registry-only entries are not launchable");
        assert!(apps.iter().all(|a| !a.id.is_empty()));
    }

    #[test]
    fn parsing_helpers() {
        assert_eq!(parse_install_date("20240131").map(|t| t > 1_700_000_000), Some(true));
        assert_eq!(parse_install_date("2024-01-31"), None);
        assert_eq!(icon_path("\"C:\\Program Files\\App\\app.exe\",0").as_deref(), Some("C:\\Program Files\\App\\app.exe"));
        assert_eq!(icon_path("C:\\App\\app.exe,-101").as_deref(), Some("C:\\App\\app.exe"));
        assert_eq!(icon_path("C:\\App, Inc\\app.ico").as_deref(), Some("C:\\App, Inc\\app.ico"));
        assert_eq!(split_command("\"C:\\Program Files\\x\\unins000.exe\" /SILENT"), Some(("C:\\Program Files\\x\\unins000.exe".into(), "/SILENT".into())));
        assert_eq!(split_command("MsiExec.exe /X{1234}"), Some(("MsiExec.exe".into(), "/X{1234}".into())));
        assert_eq!(split_command("C:\\Program Files\\x\\u.exe --remove"), Some(("C:\\Program Files\\x\\u.exe".into(), "--remove".into())));
    }

    #[test]
    fn png_from_bgra() {
        let mut px = vec![255, 0, 0, 255, 0, 0, 0, 0, 0, 128, 0, 128, 10, 20, 30, 255];
        let png = bgra_to_png(&mut px, 2, 2).unwrap();
        let img = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(img.get_pixel(0, 0).0, [0, 0, 255, 255]);
        assert_eq!(img.get_pixel(0, 1).0, [0, 255, 0, 128], "un-premultiplied");
    }
}
