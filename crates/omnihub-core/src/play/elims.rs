//! Clip eliminations in any game: while a game runs with instant replay on,
//! the middle of the screen is read every couple of seconds with Windows'
//! own OCR, and words like "Eliminated" save a clip a few seconds later.
//! Nothing is kept or uploaded: each picture is read and dropped. Games in
//! exclusive fullscreen can show black to screen capture, so borderless or
//! windowed mode works best.

/// The words to look for by default (in the user's Windows language too).
pub const DEFAULT_WORDS: &[&str] = &["eliminated", "knocked down", "elimination"];

/// After a match, wait this long before looking again.
pub const COOLDOWN_SECS: u64 = 12;

/// Whether the text has one of the words, as your elimination (not
/// "eliminated by", which is you going down).
pub fn matches(text: &str, words: &[String]) -> bool {
    text.lines().any(|line| {
        let l = line.to_lowercase();
        if l.contains("you were") || l.contains("you've been") {
            return false;
        }
        words.iter().take(20).map(|w| w.trim().to_lowercase()).filter(|w| w.chars().count() >= 3).any(|w| l.match_indices(&w).any(|(i, _)| !l[i + w.len()..].trim_start().starts_with("by ")))
    })
}

/// The part of a `w`×`h` screen read: the middle, from a fifth in on each
/// side and from half way down to 85 %.
pub fn region(w: u32, h: u32) -> (u32, u32, u32, u32) {
    (w / 5, h / 2, w * 3 / 5, h * 35 / 100)
}

/// Read monitor `index` (0 = the first) and return the middle as PNG bytes.
#[cfg(windows)]
pub fn grab(index: u32) -> Result<Vec<u8>, String> {
    let monitors = xcap::Monitor::all().map_err(|e| e.to_string())?;
    let m = monitors.get(index as usize).or_else(|| monitors.first()).ok_or("No monitor")?;
    let img = m.capture_image().map_err(|e| e.to_string())?;
    let (x, y, w, h) = region(img.width(), img.height());
    let part = image::imageops::crop_imm(&img, x, y, w, h).to_image();
    let mut out = std::io::Cursor::new(Vec::new());
    part.write_to(&mut out, image::ImageFormat::Png).map_err(|e| e.to_string())?;
    Ok(out.into_inner())
}

#[cfg(not(windows))]
pub fn grab(_index: u32) -> Result<Vec<u8>, String> {
    Err("Clipping eliminations reads the screen on Windows".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spots_your_eliminations() {
        let w: Vec<String> = DEFAULT_WORDS.iter().map(|s| s.to_string()).collect();
        assert!(matches("ELIMINATED Player123\n2 Eliminations", &w));
        assert!(matches("You eliminated SomeOne", &w));
        assert!(matches("KNOCKED DOWN xX", &w));
        assert!(!matches("Eliminated by Player123", &w));
        assert!(!matches("You were eliminated", &w));
        assert!(!matches("Storm shrinking in 0:30", &w));
        // Your own words, in any language Windows reads.
        assert!(matches("ÉLIMINÉ Joueur", &["éliminé".to_string()]));
        assert!(!matches("ab", &["ab".to_string()]));
        assert_eq!(region(1920, 1080), (384, 540, 1152, 378));
    }
}
