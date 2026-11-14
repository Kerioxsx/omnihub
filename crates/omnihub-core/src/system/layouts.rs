//! Window layouts: where your programs' windows sit ("Work", "Gaming"…),
//! saved and put back with one click, optionally starting the programs
//! that aren't open.

use std::path::PathBuf;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Placed {
    /// Lower-case executable name ("code.exe").
    pub key: String,
    pub name: String,
    pub title: String,
    pub path: String,
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    /// "normal", "maximized" or "minimized".
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Layout {
    pub name: String,
    pub windows: Vec<Placed>,
    pub saved_at: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Applied {
    pub placed: usize,
    pub launched: Vec<String>,
    /// Programs that weren't open (and weren't started).
    pub missing: Vec<String>,
}

/// Which saved window goes where: for each saved window, an open window of
/// the same program, the same title first; each open window used once.
pub fn assign(saved: &[Placed], open: &[(String, String)]) -> Vec<(usize, usize)> {
    let mut used = vec![false; open.len()];
    let mut out = Vec::new();
    // Exact titles first, so "Notes - Notepad" keeps its spot even when
    // another Notepad window is listed first.
    for exact in [true, false] {
        for (i, s) in saved.iter().enumerate() {
            if out.iter().any(|(si, _)| *si == i) {
                continue;
            }
            if let Some(j) = (0..open.len()).find(|&j| !used[j] && open[j].0 == s.key && (!exact || open[j].1 == s.title)) {
                used[j] = true;
                out.push((i, j));
            }
        }
    }
    out.sort();
    out
}

pub fn clean_name(name: &str) -> Result<String, String> {
    let n = name.trim();
    if n.is_empty() || n.chars().count() > 40 {
        return Err("Give the layout a name (up to 40 letters).".into());
    }
    Ok(n.to_string())
}

pub struct Layouts {
    file: PathBuf,
    list: Mutex<Vec<Layout>>,
}

impl Layouts {
    pub fn new(data_dir: &std::path::Path) -> Layouts {
        let file = data_dir.join("layouts.json");
        let list = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Layouts { file, list: Mutex::new(list) }
    }

    pub fn list(&self) -> Vec<Layout> {
        self.list.lock().clone()
    }

    fn write(&self, list: &[Layout]) -> Result<(), String> {
        let json = serde_json::to_vec_pretty(list).map_err(|e| e.to_string())?;
        let tmp = self.file.with_extension("json.tmp");
        std::fs::write(&tmp, json).and_then(|_| std::fs::rename(&tmp, &self.file)).map_err(|e| e.to_string())
    }

    /// Save the windows as they are now under `name` (replacing one of the same name).
    pub fn save(&self, name: &str) -> Result<Layout, String> {
        let name = clean_name(name)?;
        let windows = capture();
        if windows.is_empty() {
            return Err("No windows are open to save.".into());
        }
        let layout = Layout { name: name.clone(), windows, saved_at: chrono::Utc::now().timestamp() };
        let mut list = self.list.lock();
        list.retain(|l| !l.name.eq_ignore_ascii_case(&name));
        list.push(layout.clone());
        list.sort_by_key(|l| l.name.to_lowercase());
        self.write(&list)?;
        Ok(layout)
    }

    /// Add layouts from a settings file (replacing ones of the same name).
    pub fn import(&self, layouts: Vec<Layout>) {
        let mut list = self.list.lock();
        for l in layouts {
            if clean_name(&l.name).is_ok() {
                list.retain(|x| !x.name.eq_ignore_ascii_case(&l.name));
                list.push(l);
            }
        }
        list.sort_by_key(|l| l.name.to_lowercase());
        let _ = self.write(&list);
    }

    pub fn delete(&self, name: &str) -> Result<(), String> {
        let mut list = self.list.lock();
        list.retain(|l| l.name != name);
        self.write(&list)
    }

    /// Put the windows back. With `launch`, programs that aren't open are
    /// started and placed once their windows appear (this waits a few seconds).
    pub fn apply(&self, name: &str, launch: bool) -> Result<Applied, String> {
        let layout = self.list.lock().iter().find(|l| l.name == name).cloned().ok_or("That layout is gone.")?;
        let mut done = vec![false; layout.windows.len()];
        let mut result = Applied { placed: place(&layout.windows, &mut done), ..Default::default() };
        let mut missing: Vec<&Placed> = layout.windows.iter().zip(&done).filter(|(_, d)| !**d).map(|(w, _)| w).collect();
        missing.dedup_by(|a, b| a.key == b.key);
        if launch && !missing.is_empty() {
            for w in &missing {
                if std::path::Path::new(&w.path).is_file() && std::process::Command::new(&w.path).spawn().is_ok() {
                    result.launched.push(w.name.clone());
                }
            }
            // Wait for the new windows, up to ~10 s.
            for _ in 0..10 {
                std::thread::sleep(std::time::Duration::from_secs(1));
                result.placed += place(&layout.windows, &mut done);
                if done.iter().all(|d| *d) {
                    break;
                }
            }
        }
        let mut still: Vec<String> = layout.windows.iter().zip(&done).filter(|(_, d)| !**d).map(|(w, _)| w.name.clone()).collect();
        still.dedup();
        result.missing = still;
        Ok(result)
    }
}

#[cfg(windows)]
fn capture() -> Vec<Placed> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{GetWindowPlacement, SW_SHOWMAXIMIZED, SW_SHOWMINIMIZED, WINDOWPLACEMENT};
    super::open_apps::win::handles()
        .into_iter()
        .filter(|(_, key, _, _)| key != "omnihub.exe")
        .filter_map(|(h, key, title, path)| {
            let mut wp = WINDOWPLACEMENT { length: std::mem::size_of::<WINDOWPLACEMENT>() as u32, ..Default::default() };
            unsafe { GetWindowPlacement(HWND(h as *mut _), &mut wp) }.ok()?;
            let r = wp.rcNormalPosition;
            let state = match wp.showCmd as i32 {
                x if x == SW_SHOWMAXIMIZED.0 => "maximized",
                x if x == SW_SHOWMINIMIZED.0 || x == 6 || x == 7 => "minimized",
                _ => "normal",
            };
            Some(Placed { name: crate::media::mixer::name_of(&key), key, title, path, left: r.left, top: r.top, right: r.right, bottom: r.bottom, state: state.into() })
        })
        .collect()
}

#[cfg(not(windows))]
fn capture() -> Vec<Placed> {
    Vec::new()
}

/// Place the windows not yet `done`; returns how many were placed.
#[cfg(windows)]
fn place(saved: &[Placed], done: &mut [bool]) -> usize {
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::UI::WindowsAndMessaging::{SetWindowPlacement, SHOW_WINDOW_CMD, SW_SHOWMAXIMIZED, SW_SHOWMINNOACTIVE, SW_SHOWNORMAL, WINDOWPLACEMENT};
    let open = super::open_apps::win::handles();
    let pairs: Vec<(String, String)> = open.iter().map(|(_, k, t, _)| (k.clone(), t.clone())).collect();
    let todo: Vec<usize> = (0..saved.len()).filter(|i| !done[*i]).collect();
    let wanted: Vec<Placed> = todo.iter().map(|i| saved[*i].clone()).collect();
    let mut n = 0;
    for (wi, oi) in assign(&wanted, &pairs) {
        let w = &wanted[wi];
        let show: SHOW_WINDOW_CMD = match w.state.as_str() {
            "maximized" => SW_SHOWMAXIMIZED,
            "minimized" => SW_SHOWMINNOACTIVE,
            _ => SW_SHOWNORMAL,
        };
        let wp = WINDOWPLACEMENT { length: std::mem::size_of::<WINDOWPLACEMENT>() as u32, showCmd: show.0 as u32, rcNormalPosition: RECT { left: w.left, top: w.top, right: w.right, bottom: w.bottom }, ..Default::default() };
        if unsafe { SetWindowPlacement(HWND(open[oi].0 as *mut _), &wp) }.is_ok() {
            done[todo[wi]] = true;
            n += 1;
        }
    }
    n
}

#[cfg(not(windows))]
fn place(_saved: &[Placed], _done: &mut [bool]) -> usize {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(key: &str, title: &str) -> Placed {
        Placed { key: key.into(), name: key.into(), title: title.into(), path: String::new(), left: 0, top: 0, right: 100, bottom: 100, state: "normal".into() }
    }

    #[test]
    fn matches_windows_to_their_spots() {
        let saved = vec![w("notepad.exe", "Notes - Notepad"), w("notepad.exe", "Todo - Notepad"), w("code.exe", "omnihub - VS Code"), w("spotify.exe", "Spotify")];
        let open = vec![("notepad.exe".to_string(), "Todo - Notepad".to_string()), ("code.exe".into(), "other - VS Code".into()), ("notepad.exe".into(), "Notes - Notepad".into())];
        // Exact titles win; VS Code takes its only window; Spotify isn't open.
        assert_eq!(assign(&saved, &open), vec![(0, 2), (1, 0), (2, 1)]);
        assert!(clean_name("  ").is_err());
        assert_eq!(clean_name(" Work ").unwrap(), "Work");

        let dir = tempfile::tempdir().unwrap();
        let l = Layouts::new(dir.path());
        assert!(l.list().is_empty());
        assert!(l.apply("Work", false).is_err());
        l.list.lock().push(Layout { name: "Work".into(), windows: saved.clone(), saved_at: 1 });
        l.write(&l.list()).unwrap();
        assert_eq!(Layouts::new(dir.path()).list().len(), 1);
        if !cfg!(windows) {
            let r = l.apply("Work", false).unwrap();
            assert_eq!(r.missing, vec!["notepad.exe", "code.exe", "spotify.exe"]);
        }
        l.delete("Work").unwrap();
        assert!(Layouts::new(dir.path()).list().is_empty());
    }
}
