//! Similar photos: pictures that look the same even when they aren't the
//! same file (a resized copy, a re-saved JPEG, burst shots, the same
//! screenshot twice). Each picture gets a 64-bit "difference hash" of a tiny
//! greyscale version; pictures whose hashes differ in only a few bits are
//! grouped, and the biggest, sharpest one is suggested to keep. Nothing is
//! deleted here: the page sends the ones you pick to the Recycle Bin.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use rayon::prelude::*;
use serde::Serialize;

const EXTS: &[&str] = &["jpg", "jpeg", "png", "webp", "bmp", "gif"];
pub const MAX_PHOTOS: usize = 6000;
/// Bits that may differ for two pictures to count as the same.
pub const NEAR: u32 = 6;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Photo {
    pub path: String,
    pub bytes: u64,
    pub width: u32,
    pub height: u32,
    pub modified: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SimilarGroup {
    pub photos: Vec<Photo>,
    /// Which one to keep (the most pixels, then the biggest file).
    pub keep: usize,
}

/// The difference hash of a picture: each bit says whether a pixel of the
/// 9×8 greyscale thumbnail is brighter than its right neighbour.
pub fn dhash(img: &image::DynamicImage) -> u64 {
    let small = img.resize_exact(9, 8, image::imageops::FilterType::Triangle).to_luma8();
    let mut h = 0u64;
    for y in 0..8 {
        for x in 0..8 {
            let a = small.get_pixel(x, y)[0];
            let b = small.get_pixel(x + 1, y)[0];
            h = (h << 1) | u64::from(a > b);
        }
    }
    h
}

/// Groups of indexes whose hashes are within `near` bits of each other (connected).
pub fn group(hashes: &[u64], near: u32) -> Vec<Vec<usize>> {
    let n = hashes.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(p: &mut [usize], mut i: usize) -> usize {
        while p[i] != i {
            p[i] = p[p[i]];
            i = p[i];
        }
        i
    }
    for i in 0..n {
        for j in i + 1..n {
            if (hashes[i] ^ hashes[j]).count_ones() <= near {
                let (a, b) = (find(&mut parent, i), find(&mut parent, j));
                if a != b {
                    parent[b] = a;
                }
            }
        }
    }
    let mut groups: std::collections::BTreeMap<usize, Vec<usize>> = std::collections::BTreeMap::new();
    for i in 0..n {
        let r = find(&mut parent, i);
        groups.entry(r).or_default().push(i);
    }
    groups.into_values().filter(|g| g.len() > 1).collect()
}

fn pictures(dir: &Path) -> Vec<PathBuf> {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().is_some_and(|x| EXTS.contains(&x.to_string_lossy().to_lowercase().as_str())))
        // Icons and tiny images aren't photos.
        .filter(|e| e.metadata().is_ok_and(|m| m.len() >= 16 * 1024))
        .map(|e| e.into_path())
        .take(MAX_PHOTOS)
        .collect()
}

/// Similar pictures in a folder (and its folders), biggest groups first.
pub fn find(dir: &Path, cancel: &AtomicBool, progress: &(dyn Fn(usize, usize) + Sync)) -> Result<Vec<SimilarGroup>, String> {
    if !dir.is_dir() {
        return Err("Pick a folder".into());
    }
    let files = pictures(dir);
    let total = files.len();
    let done = std::sync::atomic::AtomicUsize::new(0);
    let read: Vec<(Photo, u64)> = files
        .par_iter()
        .filter_map(|p| {
            if cancel.load(Ordering::Relaxed) {
                return None;
            }
            let img = image::ImageReader::open(p).ok()?.with_guessed_format().ok()?.decode().ok();
            let n = done.fetch_add(1, Ordering::Relaxed) + 1;
            if n % 25 == 0 || n == total {
                progress(n, total);
            }
            let img = img?;
            let meta = std::fs::metadata(p).ok()?;
            let modified = meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_millis() as i64);
            Some((Photo { path: p.to_string_lossy().into_owned(), bytes: meta.len(), width: img.width(), height: img.height(), modified }, dhash(&img)))
        })
        .collect();
    if cancel.load(Ordering::Relaxed) {
        return Err("Stopped".into());
    }
    let hashes: Vec<u64> = read.iter().map(|r| r.1).collect();
    let mut out: Vec<SimilarGroup> = group(&hashes, NEAR)
        .into_iter()
        .map(|g| {
            let photos: Vec<Photo> = g.into_iter().map(|i| read[i].0.clone()).collect();
            let keep = photos.iter().enumerate().max_by_key(|(_, p)| (u64::from(p.width) * u64::from(p.height), p.bytes)).map_or(0, |(i, _)| i);
            SimilarGroup { photos, keep }
        })
        .collect();
    out.sort_by_key(|g| std::cmp::Reverse(g.photos.iter().map(|p| p.bytes).sum::<u64>()));
    Ok(out)
}

/// Send pictures to the Recycle Bin; says how many went.
pub fn recycle(paths: &[String]) -> usize {
    paths.iter().filter(|p| Path::new(p).is_file() && trash::delete(p).is_ok()).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture(seed: u32, w: u32, h: u32) -> image::DynamicImage {
        image::DynamicImage::ImageRgb8(image::RgbImage::from_fn(w, h, |x, y| {
            let v = ((x * 255 / w) ^ (y * 7 * seed / h)) as u8;
            image::Rgb([v, v.wrapping_mul(3), (seed * 40) as u8])
        }))
    }

    #[test]
    fn groups_look_alikes() {
        let a = picture(1, 640, 480);
        let resized = a.resize_exact(320, 240, image::imageops::FilterType::Triangle);
        let other = picture(9, 640, 480);
        let (ha, hr, ho) = (dhash(&a), dhash(&resized), dhash(&other));
        assert!((ha ^ hr).count_ones() <= NEAR, "a smaller copy looks the same");
        assert!((ha ^ ho).count_ones() > NEAR, "a different picture doesn't");
        assert_eq!(group(&[ha, ho, hr], NEAR), vec![vec![0, 2]]);

        let t = tempfile::tempdir().unwrap();
        // Noise keeps the files above the small-image cut-off.
        let noisy = |img: &image::DynamicImage| {
            let mut i = img.to_rgb8();
            for (k, p) in i.pixels_mut().enumerate() {
                p[2] = p[2].wrapping_add((k * 2654435761 % 7) as u8);
            }
            image::DynamicImage::ImageRgb8(i)
        };
        noisy(&a).save(t.path().join("big.png")).unwrap();
        std::fs::create_dir(t.path().join("sub")).unwrap();
        noisy(&resized).save(t.path().join("sub").join("small.png")).unwrap();
        noisy(&other).save(t.path().join("other.png")).unwrap();
        let groups = find(t.path(), &AtomicBool::new(false), &|_, _| {}).unwrap();
        assert_eq!(groups.len(), 1, "{groups:?}");
        let g = &groups[0];
        assert_eq!(g.photos.len(), 2);
        assert!(g.photos[g.keep].path.ends_with("big.png"), "keep the bigger one");
    }
}
