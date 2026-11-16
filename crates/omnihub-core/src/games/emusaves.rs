//! Emulator saves, memory cards and save states in one place: where each
//! emulator keeps them on this PC, how much is there, and a one-click
//! backup of all of them into a zip (Documents\OmniHub Backups). Saves next
//! to your ROMs (mGBA, melonDS) are backed up with the ROM folders' saves.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::emulators::Emulator;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SavePlace {
    pub emulator: String,
    /// "Memory cards", "Save states", "Saves".
    pub what: String,
    pub path: String,
    pub files: usize,
    pub bytes: u64,
}

/// Where each emulator keeps its saves: (emulator, what, folder under
/// Documents / under the emulator's own folder / under %APPDATA%).
enum Base {
    Docs(&'static str),
    Exe(&'static str),
    AppData(&'static str),
}

const PLACES: &[(&str, &str, Base)] = &[
    ("DuckStation", "Memory cards", Base::Docs("DuckStation/memcards")),
    ("DuckStation", "Save states", Base::Docs("DuckStation/savestates")),
    ("PCSX2", "Memory cards", Base::Docs("PCSX2/memcards")),
    ("PCSX2", "Save states", Base::Docs("PCSX2/sstates")),
    ("Dolphin", "GameCube saves", Base::Docs("Dolphin Emulator/GC")),
    ("Dolphin", "Wii saves", Base::Docs("Dolphin Emulator/Wii/title")),
    ("Dolphin", "Save states", Base::Docs("Dolphin Emulator/StateSaves")),
    ("PPSSPP", "Saves", Base::Docs("PPSSPP/PSP/SAVEDATA")),
    ("PPSSPP", "Save states", Base::Docs("PPSSPP/PSP/PPSSPP_STATE")),
    ("PPSSPP", "Saves", Base::Exe("memstick/PSP/SAVEDATA")),
    ("RetroArch", "Saves", Base::Exe("saves")),
    ("RetroArch", "Save states", Base::Exe("states")),
    ("Cemu", "Saves", Base::Exe("mlc01/usr/save")),
    ("RPCS3", "Saves", Base::Exe("dev_hdd0/home/00000001/savedata")),
    ("Azahar", "Saves", Base::AppData("Azahar/sdmc/Nintendo 3DS")),
    ("Azahar", "Save states", Base::AppData("Azahar/states")),
];

fn size(dir: &Path) -> (usize, u64) {
    walkdir::WalkDir::new(dir).into_iter().filter_map(Result::ok).filter(|e| e.file_type().is_file()).filter_map(|e| e.metadata().ok()).fold((0, 0), |(n, b), m| (n + 1, b + m.len()))
}

/// The save folders that exist, for the emulators you have (and the usual
/// Documents folders, which outlive an emulator).
pub fn places(emulators: &[Emulator], docs: Option<&Path>, appdata: Option<&Path>) -> Vec<SavePlace> {
    let mut out: Vec<SavePlace> = Vec::new();
    for (emu, what, base) in PLACES {
        let dirs: Vec<PathBuf> = match base {
            Base::Docs(rel) => docs.map(|d| d.join(rel)).into_iter().collect(),
            Base::AppData(rel) => appdata.map(|d| d.join(rel)).into_iter().collect(),
            Base::Exe(rel) => emulators.iter().filter(|e| e.name == *emu).filter_map(|e| Path::new(&e.exe).parent().map(|p| p.join(rel))).collect(),
        };
        for d in dirs.into_iter().filter(|d| d.is_dir()) {
            let path = d.display().to_string();
            if out.iter().any(|p| p.path == path) {
                continue;
            }
            let (files, bytes) = size(&d);
            if files > 0 {
                out.push(SavePlace { emulator: emu.to_string(), what: what.to_string(), path, files, bytes });
            }
        }
    }
    out
}

/// Zip these folders (and the saves beside ROMs) into `dest_dir`; returns the zip.
pub fn backup(places: &[SavePlace], rom_saves: &[String], dest_dir: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dest_dir).map_err(|e| e.to_string())?;
    let out = dest_dir.join(format!("Emulator saves {}.zip", chrono::Local::now().format("%Y-%m-%d %H-%M")));
    let part = out.with_extension("zip.part");
    let r = (|| -> Result<(), String> {
        let mut z = zip::ZipWriter::new(std::fs::File::create(&part).map_err(|e| e.to_string())?);
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for p in places {
            let base = Path::new(&p.path);
            for e in walkdir::WalkDir::new(base).into_iter().flatten().filter(|e| e.file_type().is_file()) {
                let rel = e.path().strip_prefix(base).map_err(|e| e.to_string())?.to_string_lossy().replace('\\', "/");
                let name = format!("{}/{}/{rel}", p.emulator, p.what);
                let Ok(mut f) = std::fs::File::open(e.path()) else { continue };
                z.start_file(name, opts.large_file(e.metadata().map(|m| m.len()).unwrap_or(0) >= 1 << 32)).map_err(|e| e.to_string())?;
                std::io::copy(&mut f, &mut z).map_err(|e| e.to_string())?;
            }
        }
        for s in rom_saves {
            let p = Path::new(s);
            let Ok(mut f) = std::fs::File::open(p) else { continue };
            let file = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            z.start_file(format!("Next to ROMs/{file}"), opts).map_err(|e| e.to_string())?;
            std::io::copy(&mut f, &mut z).map_err(|e| e.to_string())?;
        }
        z.finish().map_err(|e| e.to_string())?.flush().map_err(|e| e.to_string())
    })();
    if let Err(e) = r {
        let _ = std::fs::remove_file(&part);
        return Err(format!("The backup failed: {e}"));
    }
    std::fs::rename(&part, &out).map_err(|e| e.to_string())?;
    Ok(out)
}

/// The save folders on this PC.
pub fn on_this_pc(emulators: &[Emulator]) -> Vec<SavePlace> {
    places(emulators, dirs::document_dir().as_deref(), dirs::config_dir().as_deref())
}

/// Back up everything into Documents\OmniHub Backups.
pub fn backup_all(state: &super::emulators::EmuState) -> Result<PathBuf, String> {
    let places = on_this_pc(&state.emulators);
    let rom_saves: Vec<String> = state.roms.iter().filter_map(|r| r.save.clone()).collect();
    if places.is_empty() && rom_saves.is_empty() {
        return Err("No emulator saves found yet".into());
    }
    backup(&places, &rom_saves, &dirs::document_dir().ok_or("No Documents folder")?.join("OmniHub Backups"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_and_backs_up_saves() {
        let t = tempfile::tempdir().unwrap();
        let docs = t.path().join("Documents");
        std::fs::create_dir_all(docs.join("PCSX2").join("memcards")).unwrap();
        std::fs::write(docs.join("PCSX2").join("memcards").join("Mcd001.ps2"), vec![1u8; 100]).unwrap();
        let ra = t.path().join("RetroArch");
        std::fs::create_dir_all(ra.join("saves")).unwrap();
        std::fs::write(ra.join("saves").join("Game.srm"), b"x").unwrap();
        std::fs::write(ra.join("retroarch.exe"), b"").unwrap();
        let emus = vec![Emulator { id: "ra".into(), name: "RetroArch".into(), exe: ra.join("retroarch.exe").display().to_string(), platforms: vec![], args: String::new(), found: true }];
        let p = places(&emus, Some(&docs), None);
        assert_eq!(p.iter().map(|x| (x.emulator.as_str(), x.what.as_str(), x.files)).collect::<Vec<_>>(), vec![("PCSX2", "Memory cards", 1), ("RetroArch", "Saves", 1)]);
        let rom_save = t.path().join("Emerald.sav");
        std::fs::write(&rom_save, b"s").unwrap();
        let zip = backup(&p, &[rom_save.display().to_string()], &t.path().join("out")).unwrap();
        let z = zip::ZipArchive::new(std::fs::File::open(zip).unwrap()).unwrap();
        let mut names: Vec<&str> = z.file_names().collect();
        names.sort();
        assert_eq!(names, vec!["Next to ROMs/Emerald.sav", "PCSX2/Memory cards/Mcd001.ps2", "RetroArch/Saves/Game.srm"]);
    }
}
