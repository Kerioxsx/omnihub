//! The phone as a whiteboard: strokes drawn on the phone show up live on the
//! PC (Tools → Whiteboard), where the drawing can be saved as a picture.

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

pub const MAX_STROKES: usize = 5000;
pub const MAX_POINTS: usize = 4000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Stroke {
    /// 0–1 across the board's width and height.
    pub points: Vec<[f32; 2]>,
    /// "#rrggbb".
    pub color: String,
    /// Fraction of the board's width.
    pub width: f32,
    /// Rubs out instead of drawing.
    #[serde(default)]
    pub eraser: bool,
}

impl Stroke {
    /// Bounded and well-formed, or None.
    pub fn clean(mut self) -> Option<Stroke> {
        let hex = self.color.len() == 7 && self.color.starts_with('#') && self.color[1..].bytes().all(|b| b.is_ascii_hexdigit());
        if !hex || self.points.is_empty() || !self.width.is_finite() {
            return None;
        }
        self.points.truncate(MAX_POINTS);
        self.points.retain(|p| p[0].is_finite() && p[1].is_finite());
        for p in &mut self.points {
            p[0] = p[0].clamp(0.0, 1.0);
            p[1] = p[1].clamp(0.0, 1.0);
        }
        self.width = self.width.clamp(0.001, 0.2);
        (!self.points.is_empty()).then_some(self)
    }
}

#[derive(Default)]
pub struct Board {
    strokes: Mutex<Vec<Stroke>>,
}

impl Board {
    pub fn strokes(&self) -> Vec<Stroke> {
        self.strokes.lock().clone()
    }

    /// Adds what's valid; returns what was added.
    pub fn add(&self, strokes: Vec<Stroke>) -> Vec<Stroke> {
        let added: Vec<Stroke> = strokes.into_iter().filter_map(Stroke::clean).collect();
        let mut s = self.strokes.lock();
        let room = MAX_STROKES.saturating_sub(s.len());
        let added: Vec<Stroke> = added.into_iter().take(room).collect();
        s.extend(added.iter().cloned());
        added
    }

    /// Takes back the last stroke.
    pub fn undo(&self) -> bool {
        self.strokes.lock().pop().is_some()
    }

    pub fn clear(&self) {
        self.strokes.lock().clear();
    }
}

/// Where saved drawings go.
pub fn folder() -> std::path::PathBuf {
    dirs::picture_dir().unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join("Pictures")).join("OmniHub Whiteboard")
}

/// Save the PC's picture of the board (a PNG data URL from the page).
pub fn save_png(data_url: &str) -> Result<std::path::PathBuf, String> {
    use base64::Engine;
    let b64 = data_url.strip_prefix("data:image/png;base64,").ok_or("That isn't a PNG picture.")?;
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64.trim()).map_err(|_| "That isn't a PNG picture.".to_string())?;
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err("That isn't a PNG picture.".into());
    }
    let dir = folder();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let p = crate::remote::transfer::unique_path(&dir, &format!("Whiteboard {}.png", chrono::Local::now().format("%Y-%m-%d %H-%M")));
    std::fs::write(&p, bytes).map_err(|e| e.to_string())?;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strokes_are_checked() {
        let b = Board::default();
        let ok = Stroke { points: vec![[0.1, 0.2], [2.0, -1.0]], color: "#ff0000".into(), width: 9.0, eraser: false };
        let bad = Stroke { points: vec![[0.1, 0.2]], color: "red;}".into(), width: 0.01, eraser: false };
        let added = b.add(vec![ok, bad]);
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].points[1], [1.0, 0.0]);
        assert_eq!(added[0].width, 0.2);
        assert!(b.undo() && !b.undo());
    }
}
