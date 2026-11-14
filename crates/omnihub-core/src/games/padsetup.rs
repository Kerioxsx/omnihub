//! Controller auto-setup: give DuckStation and PCSX2 the standard mapping
//! for an Xbox-style controller (the same one their own "Automatic Mapping"
//! makes, through SDL), when the first controller port has nothing mapped
//! yet. A mapping you made yourself is never changed.

use std::path::{Path, PathBuf};

use serde::Serialize;

use super::ini::Doc;

/// Port 1's bindings: (button, SDL control). The same names in both.
pub const BINDINGS: &[(&str, &str)] = &[
    ("Up", "SDL-0/DPadUp"),
    ("Right", "SDL-0/DPadRight"),
    ("Down", "SDL-0/DPadDown"),
    ("Left", "SDL-0/DPadLeft"),
    ("Triangle", "SDL-0/Y"),
    ("Circle", "SDL-0/B"),
    ("Cross", "SDL-0/A"),
    ("Square", "SDL-0/X"),
    ("Select", "SDL-0/Back"),
    ("Start", "SDL-0/Start"),
    ("L1", "SDL-0/LeftShoulder"),
    ("R1", "SDL-0/RightShoulder"),
    ("L2", "SDL-0/+LeftTrigger"),
    ("R2", "SDL-0/+RightTrigger"),
    ("L3", "SDL-0/LeftStick"),
    ("R3", "SDL-0/RightStick"),
    ("LLeft", "SDL-0/-LeftX"),
    ("LRight", "SDL-0/+LeftX"),
    ("LDown", "SDL-0/+LeftY"),
    ("LUp", "SDL-0/-LeftY"),
    ("RLeft", "SDL-0/-RightX"),
    ("RRight", "SDL-0/+RightX"),
    ("RDown", "SDL-0/+RightY"),
    ("RUp", "SDL-0/-RightY"),
    ("SmallMotor", "SDL-0/SmallMotor"),
    ("LargeMotor", "SDL-0/LargeMotor"),
];

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PadResult {
    pub emulator: String,
    /// "set", "already" (a mapping is there), "notRun" (no settings file yet).
    pub status: String,
    pub file: String,
}

/// Map port 1 in this settings text when it has no buttons mapped; the
/// controller type is DuckStation's or PCSX2's analog pad.
pub fn apply(text: &[u8], pad_type: &str) -> Option<Vec<u8>> {
    let mut doc = Doc::ini(text);
    if BINDINGS.iter().take(10).any(|(k, _)| doc.get(Some("Pad1"), k).is_some_and(|v| !v.trim().is_empty())) {
        return None;
    }
    doc.set(Some("InputSources"), "SDL", "true", true);
    doc.set(Some("Pad1"), "Type", pad_type, true);
    for (k, v) in BINDINGS {
        doc.set(Some("Pad1"), k, v, true);
    }
    Some(doc.to_bytes())
}

/// The settings file: beside the program when it runs portable, else in Documents.
fn settings(exe: &Path, portable_rel: &str, docs_rel: &str) -> Option<PathBuf> {
    let dir = exe.parent()?;
    if dir.join("portable.txt").is_file() || dir.join("portable.ini").is_file() {
        return Some(dir.join(portable_rel));
    }
    dirs::document_dir().map(|d| d.join(docs_rel))
}

/// Set up port 1 for each of these emulators: (name, program).
pub fn setup(emulators: &[(String, String)]) -> Vec<PadResult> {
    let mut out = Vec::new();
    for (name, exe) in emulators {
        let (file, pad) = match name.as_str() {
            "DuckStation" => (settings(Path::new(exe), "settings.ini", "DuckStation/settings.ini"), "AnalogController"),
            "PCSX2" => (settings(Path::new(exe), "inis/PCSX2.ini", "PCSX2/inis/PCSX2.ini"), "DualShock2"),
            _ => continue,
        };
        let Some(file) = file else { continue };
        let status = match std::fs::read(&file) {
            Err(_) => "notRun",
            Ok(text) => match apply(&text, pad) {
                None => "already",
                Some(next) => {
                    let _ = std::fs::copy(&file, file.with_extension("ini.before-omnihub"));
                    if crate::settings::write_atomic(&file, &next).is_ok() {
                        "set"
                    } else {
                        "notRun"
                    }
                }
            },
        };
        out.push(PadResult { emulator: name.clone(), status: status.into(), file: file.display().to_string() });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_an_empty_port_only() {
        let text = b"[Main]\r\nSettingsVersion = 3\r\n\r\n[Pad1]\r\nType = AnalogController\r\n";
        let out = String::from_utf8(apply(text, "AnalogController").unwrap()).unwrap();
        assert!(out.contains("Cross=SDL-0/A\r\n") && out.contains("[InputSources]\r\nSDL=true") && out.contains("SettingsVersion = 3"));
        // Mapped already (by the user): left alone.
        assert!(apply(out.as_bytes(), "AnalogController").is_none());
        assert!(apply(b"[Pad1]\nUp = Keyboard/W\n", "DualShock2").is_none());
    }
}
