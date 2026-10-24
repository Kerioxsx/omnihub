//! The world seen from above: OmniHub reads the world's region files (the
//! server keeps saving them as people play) and draws each 512×512-block
//! region as a picture, one pixel per block, shaded by height like the
//! game's own maps. Pictures are kept and redrawn when their region changes.
//! Worlds from Minecraft 1.18 on (the current chunk format) are drawn.

use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::nbt::{self, Tag};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Dim {
    Overworld,
    Nether,
    End,
}

impl Dim {
    pub fn from_id(s: &str) -> Option<Dim> {
        Some(match s.trim_start_matches("minecraft:") {
            "overworld" => Dim::Overworld,
            "the_nether" | "nether" => Dim::Nether,
            "the_end" | "end" => Dim::End,
            _ => return None,
        })
    }

    pub fn id(self) -> &'static str {
        match self {
            Dim::Overworld => "overworld",
            Dim::Nether => "the_nether",
            Dim::End => "the_end",
        }
    }
}

/// Where a dimension's region files are, in any of the layouts servers use.
pub fn region_dir(server: &Path, level: &str, dim: Dim) -> Option<PathBuf> {
    let world = server.join(level);
    let mut c = vec![world.join("dimensions").join("minecraft").join(dim.id()).join("region")];
    match dim {
        Dim::Overworld => c.push(world.join("region")),
        Dim::Nether => {
            c.push(world.join("DIM-1").join("region"));
            c.push(server.join(format!("{level}_nether")).join("DIM-1").join("region"));
        }
        Dim::End => {
            c.push(world.join("DIM1").join("region"));
            c.push(server.join(format!("{level}_the_end")).join("DIM1").join("region"));
        }
    }
    c.into_iter().find(|d| d.is_dir())
}

/// The regions a dimension has: (x, z), each 512 blocks square.
pub fn regions(dir: &Path) -> Vec<(i32, i32)> {
    let mut v: Vec<(i32, i32)> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.metadata().map(|m| m.len() > 8192).unwrap_or(false))
                .filter_map(|e| {
                    let n = e.file_name().to_string_lossy().into_owned();
                    let mut p = n.strip_prefix("r.")?.strip_suffix(".mca")?.split('.');
                    Some((p.next()?.parse().ok()?, p.next()?.parse().ok()?))
                })
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

/// Read one chunk's NBT out of a region file (None: never generated).
fn chunk(f: &mut std::fs::File, dir: &Path, cx: i32, cz: i32) -> Result<Option<Tag>, String> {
    let i = ((cx & 31) + (cz & 31) * 32) as u64;
    let mut loc = [0u8; 4];
    f.seek(SeekFrom::Start(i * 4)).map_err(|e| e.to_string())?;
    f.read_exact(&mut loc).map_err(|e| e.to_string())?;
    let offset = u32::from_be_bytes([0, loc[0], loc[1], loc[2]]) as u64;
    if offset < 2 || loc[3] == 0 {
        return Ok(None);
    }
    f.seek(SeekFrom::Start(offset * 4096)).map_err(|e| e.to_string())?;
    let mut head = [0u8; 5];
    f.read_exact(&mut head).map_err(|e| e.to_string())?;
    let len = u32::from_be_bytes([head[0], head[1], head[2], head[3]]) as usize;
    if !(1..=1 << 24).contains(&len) {
        return Ok(None);
    }
    let kind = head[4];
    let data = if kind & 0x80 != 0 {
        // Stored beside the region (very big chunks): c.<x>.<z>.mcc
        std::fs::read(dir.join(format!("c.{cx}.{cz}.mcc"))).map_err(|e| e.to_string())?
    } else {
        let mut b = vec![0u8; len - 1];
        f.read_exact(&mut b).map_err(|e| e.to_string())?;
        b
    };
    let raw = match kind & 0x7f {
        1 | 2 => nbt::inflate(&data)?,
        3 => data,
        4 => lz4_blocks(&data)?,
        _ => return Ok(None),
    };
    nbt::parse(&raw).map(Some)
}

/// Java's LZ4BlockOutputStream format (region-file-compression=lz4).
fn lz4_blocks(mut b: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    while b.len() >= 21 && &b[..8] == b"LZ4Block" {
        let token = b[8];
        let clen = u32::from_le_bytes([b[9], b[10], b[11], b[12]]) as usize;
        let dlen = u32::from_le_bytes([b[13], b[14], b[15], b[16]]) as usize;
        b = &b[21..];
        if dlen == 0 {
            break;
        }
        if clen > b.len() || dlen > 1 << 24 {
            return Err("A damaged chunk.".into());
        }
        if token & 0xf0 == 0x10 {
            out.extend_from_slice(&b[..clen]);
        } else {
            out.extend(lz4_flex::block::decompress(&b[..clen], dlen).map_err(|e| e.to_string())?);
        }
        b = &b[clen..];
    }
    Ok(out)
}

/// One chunk's blocks, as far as drawing needs.
struct Chunk<'a> {
    /// Section Y → (palette names, packed indices).
    sections: HashMap<i32, (Vec<&'a str>, &'a [i64])>,
    min_y: i32,
    max_y: i32,
}

impl<'a> Chunk<'a> {
    fn new(t: &'a Tag) -> Option<Chunk<'a>> {
        let status = t.get("Status").and_then(Tag::str).unwrap_or("full");
        if status.trim_start_matches("minecraft:") != "full" {
            return None;
        }
        let mut sections = HashMap::new();
        for s in t.get("sections")?.list() {
            let Some(y) = s.get("Y").and_then(Tag::int) else { continue };
            let Some(bs) = s.get("block_states") else { continue };
            let pal: Vec<&str> = bs.get("palette").map(|p| p.list().iter().map(|b| b.get("Name").and_then(Tag::str).unwrap_or("minecraft:air")).collect()).unwrap_or_default();
            if pal.is_empty() {
                continue;
            }
            sections.insert(y as i32, (pal, bs.get("data").map(Tag::longs).unwrap_or(&[])));
        }
        let min = *sections.keys().min()?;
        let max = *sections.keys().max()?;
        Some(Chunk { sections, min_y: min * 16, max_y: max * 16 + 15 })
    }

    fn block(&self, x: usize, y: i32, z: usize) -> &'a str {
        let Some((pal, data)) = self.sections.get(&y.div_euclid(16)) else { return "minecraft:air" };
        if pal.len() == 1 || data.is_empty() {
            return pal[0];
        }
        let bits = (usize::BITS - (pal.len() - 1).leading_zeros()).max(4) as usize;
        let per = 64 / bits;
        let i = (y.rem_euclid(16) as usize) * 256 + z * 16 + x;
        let Some(word) = data.get(i / per) else { return "minecraft:air" };
        let v = ((*word as u64) >> ((i % per) * bits)) & ((1u64 << bits) - 1);
        pal.get(v as usize).copied().unwrap_or("minecraft:air")
    }
}

fn air(b: &str) -> bool {
    matches!(b, "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air" | "minecraft:light" | "minecraft:barrier" | "minecraft:structure_void")
}

fn watery(b: &str) -> bool {
    matches!(b, "minecraft:water" | "minecraft:bubble_column" | "minecraft:kelp" | "minecraft:kelp_plant" | "minecraft:seagrass" | "minecraft:tall_seagrass")
}

/// A block's colour on the map (close to the game's own map colours).
pub fn colour(name: &str) -> [u8; 3] {
    let n = name.rsplit(':').next().unwrap_or(name);
    const DYES: &[(&str, [u8; 3])] = &[
        ("light_blue_", [102, 153, 216]), ("light_gray_", [153, 153, 153]), ("white_", [232, 232, 232]), ("orange_", [216, 127, 51]), ("magenta_", [178, 76, 216]),
        ("yellow_", [229, 229, 51]), ("lime_", [127, 204, 25]), ("pink_", [242, 127, 165]), ("gray_", [76, 76, 76]), ("cyan_", [76, 127, 153]),
        ("purple_", [127, 63, 178]), ("blue_", [51, 76, 178]), ("brown_", [102, 76, 51]), ("green_", [102, 127, 51]), ("red_", [153, 51, 51]), ("black_", [25, 25, 25]),
    ];
    const WOODS: &[(&str, [u8; 3])] = &[
        ("dark_oak", [102, 76, 51]), ("pale_oak", [214, 207, 196]), ("oak", [143, 119, 72]), ("spruce", [129, 86, 49]), ("birch", [215, 203, 141]), ("jungle", [151, 109, 77]),
        ("acacia", [216, 127, 51]), ("mangrove", [153, 51, 51]), ("cherry", [209, 177, 161]), ("bamboo", [229, 229, 51]), ("crimson", [148, 63, 97]), ("warped", [58, 142, 140]),
    ];
    match n {
        "water" | "bubble_column" => return [52, 92, 210],
        "lava" => return [230, 100, 20],
        "grass_block" => return [110, 170, 60],
        "short_grass" | "tall_grass" | "fern" | "large_fern" | "bush" | "sweet_berry_bush" | "sugar_cane" | "vine" | "lily_pad" | "cactus" | "moss_block" | "moss_carpet" | "big_dripleaf" | "small_dripleaf" => return [80, 140, 40],
        "sand" | "sandstone" | "smooth_sandstone" | "cut_sandstone" | "chiseled_sandstone" | "suspicious_sand" | "end_stone" | "end_stone_bricks" | "glowstone" | "bone_block" => return [219, 207, 163],
        "red_sand" | "red_sandstone" | "smooth_red_sandstone" | "cut_red_sandstone" | "terracotta" | "pumpkin" | "carved_pumpkin" | "jack_o_lantern" | "honey_block" | "shroomlight" => return [190, 110, 50],
        "snow" | "snow_block" | "powder_snow" | "white_wool" | "quartz_block" | "smooth_quartz" | "calcite" | "diorite" | "polished_diorite" => return [236, 238, 240],
        "ice" | "packed_ice" | "blue_ice" | "frosted_ice" => return [150, 170, 245],
        "dirt" | "coarse_dirt" | "rooted_dirt" | "farmland" | "dirt_path" | "mud" | "packed_mud" | "mud_bricks" | "brown_mushroom_block" => return [140, 100, 70],
        "podzol" => return [110, 78, 45],
        "mycelium" => return [120, 90, 120],
        "clay" => return [160, 166, 180],
        "gravel" | "suspicious_gravel" => return [130, 124, 120],
        "granite" | "polished_granite" => return [149, 103, 85],
        "tuff" | "tuff_bricks" | "polished_tuff" => return [100, 100, 90],
        "obsidian" | "crying_obsidian" => return [20, 18, 29],
        "netherrack" | "nether_bricks" | "nether_quartz_ore" | "nether_gold_ore" | "magma_block" | "nether_wart_block" | "red_nether_bricks" => return [112, 22, 18],
        "soul_sand" | "soul_soil" => return [84, 64, 51],
        "basalt" | "polished_basalt" | "smooth_basalt" | "blackstone" | "polished_blackstone" | "gilded_blackstone" | "coal_block" => return [40, 38, 42],
        "crimson_nylium" => return [150, 30, 40],
        "warped_nylium" | "warped_wart_block" => return [22, 126, 120],
        "purpur_block" | "purpur_pillar" | "chorus_plant" | "chorus_flower" => return [169, 125, 169],
        "glass" | "glass_pane" => return [200, 220, 230],
        "iron_block" | "anvil" | "cauldron" | "hopper" => return [167, 167, 167],
        "gold_block" => return [250, 238, 77],
        "diamond_block" => return [92, 219, 213],
        "emerald_block" => return [0, 217, 58],
        "redstone_block" => return [255, 0, 0],
        "lapis_block" => return [74, 128, 255],
        "deepslate" | "cobbled_deepslate" | "polished_deepslate" | "deepslate_bricks" | "deepslate_tiles" | "sculk" => return [80, 80, 86],
        "melon" | "slime_block" => return [127, 178, 56],
        "bedrock" => return [60, 60, 60],
        "hay_block" => return [229, 200, 51],
        "beacon" | "sea_lantern" | "prismarine" | "prismarine_bricks" | "dark_prismarine" => return [92, 170, 160],
        _ => {}
    }
    if n.ends_with("_leaves") {
        return match n {
            "cherry_leaves" => [235, 170, 200],
            "azalea_leaves" | "flowering_azalea_leaves" => [100, 140, 50],
            "pale_oak_leaves" => [160, 170, 150],
            _ => [56, 118, 32],
        };
    }
    if n.contains("coral") {
        return [200, 90, 140];
    }
    if n.ends_with("_ore") {
        return [112, 112, 112];
    }
    for (p, c) in DYES {
        if n.starts_with(p) {
            return *c;
        }
    }
    if n.ends_with("_terracotta") || n.ends_with("_glazed_terracotta") {
        return [152, 94, 67];
    }
    for (p, c) in WOODS {
        if n.starts_with(p) && !n.ends_with("_leaves") {
            return *c;
        }
    }
    if n.contains("copper") {
        return if n.contains("oxidized") || n.contains("weathered") { [86, 160, 140] } else { [190, 110, 80] };
    }
    if n.contains("brick") {
        return [150, 90, 70];
    }
    if n.ends_with("_flower") || n.ends_with("_tulip") || matches!(n, "dandelion" | "poppy" | "allium" | "azure_bluet" | "oxeye_daisy" | "cornflower" | "lily_of_the_valley" | "sunflower" | "lilac" | "rose_bush" | "peony" | "wildflowers" | "pink_petals") {
        return [100, 150, 50];
    }
    [118, 118, 118]
}

/// Water gets darker with depth; land by height against its northern neighbour.
fn shade(c: [u8; 3], f: f32) -> [u8; 3] {
    [(c[0] as f32 * f).min(255.0) as u8, (c[1] as f32 * f).min(255.0) as u8, (c[2] as f32 * f).min(255.0) as u8]
}

/// Draw one region (512×512 blocks) as an image; transparent where nothing was generated.
pub fn render(dir: &Path, rx: i32, rz: i32, dim: Dim) -> Result<image::RgbaImage, String> {
    let path = dir.join(format!("r.{rx}.{rz}.mca"));
    let mut f = std::fs::File::open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    const N: usize = 512;
    let mut heights = vec![i32::MIN; N * N];
    let mut colours = vec![[0u8; 4]; N * N];
    let mut cache: HashMap<String, [u8; 3]> = HashMap::new();
    for cz in 0..32 {
        for cx in 0..32 {
            let (gx, gz) = (rx * 32 + cx, rz * 32 + cz);
            let Ok(Some(t)) = chunk(&mut f, dir, gx, gz) else { continue };
            let Some(c) = Chunk::new(&t) else { continue };
            let hm = t.get("Heightmaps").and_then(|h| h.get("MOTION_BLOCKING").or(h.get("WORLD_SURFACE"))).map(Tag::longs).unwrap_or(&[]);
            let per = if hm.is_empty() { 0 } else { 256usize.div_ceil(hm.len()) };
            let bits = 64usize.checked_div(per).unwrap_or(0);
            for z in 0..16 {
                for x in 0..16 {
                    // The top block: from the heightmap, or (the Nether, under its roof) by looking down.
                    let mut y = if dim == Dim::Nether || bits == 0 {
                        let mut y = if dim == Dim::Nether { c.max_y.min(120) } else { c.max_y };
                        if dim == Dim::Nether {
                            while y > c.min_y && !air(c.block(x, y, z)) {
                                y -= 1;
                            }
                        }
                        while y > c.min_y && air(c.block(x, y, z)) {
                            y -= 1;
                        }
                        y
                    } else {
                        let i = z * 16 + x;
                        let v = ((hm.get(i / per).copied().unwrap_or(0) as u64) >> ((i % per) * bits)) & ((1u64 << bits) - 1);
                        c.min_y + v as i32 - 1
                    };
                    if y < c.min_y {
                        continue;
                    }
                    let mut b = c.block(x, y, z);
                    while air(b) && y > c.min_y {
                        y -= 1;
                        b = c.block(x, y, z);
                    }
                    if air(b) {
                        continue;
                    }
                    let mut depth = 0;
                    if watery(b) {
                        let mut yy = y;
                        while yy > c.min_y && watery(c.block(x, yy, z)) && depth < 24 {
                            yy -= 1;
                            depth += 1;
                        }
                    }
                    let col = *cache.entry(b.to_string()).or_insert_with(|| colour(if watery(b) { "minecraft:water" } else { b }));
                    let px = (cz as usize * 16 + z) * N + cx as usize * 16 + x;
                    heights[px] = y;
                    colours[px] = [col[0], col[1], col[2], if depth > 0 { 255 - depth.min(12) as u8 } else { 255 }];
                }
            }
        }
    }
    let mut img = image::RgbaImage::new(N as u32, N as u32);
    for z in 0..N {
        for x in 0..N {
            let i = z * N + x;
            if heights[i] == i32::MIN {
                continue;
            }
            let [r, g, b, a] = colours[i];
            let col = if a < 255 {
                // Water: deeper is darker.
                shade([r, g, b], 1.0 - (255 - a) as f32 * 0.03)
            } else {
                let north = if z > 0 { heights[i - N] } else { i32::MIN };
                let f = if north == i32::MIN || north == heights[i] { 0.92 } else if heights[i] > north { 1.06 } else { 0.78 };
                shade([r, g, b], f)
            };
            img.put_pixel(x as u32, z as u32, image::Rgba([col[0], col[1], col[2], 255]));
        }
    }
    Ok(img)
}

/// A region's picture as PNG, kept in `cache` and redrawn when the region changed.
pub fn tile(dir: &Path, cache: &Path, rx: i32, rz: i32, dim: Dim) -> Result<Vec<u8>, String> {
    let src = dir.join(format!("r.{rx}.{rz}.mca"));
    let changed = std::fs::metadata(&src).and_then(|m| m.modified()).map_err(|_| "That part of the world hasn't been explored.".to_string())?;
    let out = cache.join(dim.id()).join(format!("r.{rx}.{rz}.png"));
    if let Ok(m) = std::fs::metadata(&out).and_then(|m| m.modified()) {
        if m >= changed {
            if let Ok(b) = std::fs::read(&out) {
                return Ok(b);
            }
        }
    }
    let img = render(dir, rx, rz, dim)?;
    let png = crate::mods::cover::png(&img);
    let _ = std::fs::create_dir_all(out.parent().unwrap_or(cache));
    let _ = std::fs::write(&out, &png);
    Ok(png)
}

/// The world's spawn point from level.dat: (x, y, z).
pub fn spawn(server: &Path, level: &str) -> Option<[i32; 3]> {
    let t = nbt::read(&std::fs::read(server.join(level).join("level.dat")).ok()?).ok()?;
    let d = t.get("Data")?;
    if let (Some(x), Some(y), Some(z)) = (d.get("SpawnX").and_then(Tag::int), d.get("SpawnY").and_then(Tag::int), d.get("SpawnZ").and_then(Tag::int)) {
        return Some([x as i32, y as i32, z as i32]);
    }
    // 1.21.9+: Data.spawn.pos as [I; x, y, z].
    let pos = d.get("spawn")?.get("pos")?;
    match pos {
        Tag::Ints(v) if v.len() == 3 => Some([v[0], v[1], v[2]]),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcserver::nbt::tests::write;

    /// Blocks of a section: (name, where it is) — the last that matches wins.
    type Fill<'a> = &'a [(&'a str, &'a dyn Fn(usize, usize, usize) -> bool)];

    /// A region file with one chunk: grass at y=64 everywhere, a 3-deep pond, a stone pillar.
    pub fn region(dir: &Path) {
        let sec_data = |fill: Fill| -> Tag {
            let pal: Vec<Tag> = fill.iter().map(|(n, _)| Tag::Compound(vec![("Name".into(), Tag::Str(format!("minecraft:{n}")))])).collect();
            let bits = 4;
            let per = 64 / bits;
            let mut longs = vec![0i64; 4096 / per];
            for i in 0..4096 {
                let (x, z, y) = (i % 16, (i / 16) % 16, i / 256);
                let idx = fill.iter().rposition(|(_, f)| f(x, y, z)).unwrap_or(0);
                longs[i / per] |= (idx as i64) << ((i % per) * bits);
            }
            Tag::Compound(vec![("palette".into(), Tag::List(pal)), ("data".into(), Tag::Longs(longs))])
        };
        let pond = |x: usize, z: usize| (4..8).contains(&x) && (4..8).contains(&z);
        // Section Y=4 holds y 64..79: grass at 64 (or water 62..64 in the pond), stone pillar to 70 at (12,12).
        let s4 = sec_data(&[
            ("air", &|_, _, _| true),
            ("grass_block", &|x, y, z| y == 0 && !pond(x, z)),
            ("water", &|x, y, z| y == 0 && pond(x, z)),
            ("stone", &|x, y, z| x == 12 && z == 12 && y <= 6),
        ]);
        let s3 = sec_data(&[("stone", &|_, _, _| true), ("water", &|x, y, z| y >= 14 && pond(x, z))]);
        let mut hm = vec![0i64; 37];
        for i in 0..256 {
            let (x, z) = (i % 16, i / 16);
            let top = if x == 12 && z == 12 { 70 } else { 64 };
            let v = (top + 1 - 48) as i64; // min_y here is 48 (section 3)
            hm[i / 7] |= v << ((i % 7) * 9);
        }
        let chunk = Tag::Compound(vec![
            ("DataVersion".into(), Tag::Int(4786)),
            ("Status".into(), Tag::Str("minecraft:full".into())),
            ("sections".into(), Tag::List(vec![Tag::Compound(vec![("Y".into(), Tag::Byte(3)), ("block_states".into(), s3)]), Tag::Compound(vec![("Y".into(), Tag::Byte(4)), ("block_states".into(), s4)])])),
            ("Heightmaps".into(), Tag::Compound(vec![("MOTION_BLOCKING".into(), Tag::Longs(hm))])),
        ]);
        let raw = write(&chunk);
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
        std::io::Write::write_all(&mut z, &raw).unwrap();
        let data = z.finish().unwrap();
        let mut file = vec![0u8; 8192];
        // Chunk (1, 0) of region (0, 0) at sector 2.
        let idx = 1;
        let sectors = (data.len() + 5).div_ceil(4096);
        file[idx * 4..idx * 4 + 4].copy_from_slice(&[0, 0, 2, sectors as u8]);
        file.extend(((data.len() + 1) as u32).to_be_bytes());
        file.push(2);
        file.extend(&data);
        file.resize(8192 + sectors * 4096, 0);
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("r.0.0.mca"), file).unwrap();
    }

    #[test]
    fn draws_a_region_from_its_chunks() {
        let t = tempfile::tempdir().unwrap();
        let rd = t.path().join("world/dimensions/minecraft/overworld/region");
        region(&rd);
        assert_eq!(region_dir(t.path(), "world", Dim::Overworld).as_deref(), Some(rd.as_path()));
        assert_eq!(regions(&rd), [(0, 0)]);
        let img = render(&rd, 0, 0, Dim::Overworld).unwrap();
        let px = |x: u32, z: u32| img.get_pixel(x, z).0;
        assert_eq!(px(3, 3)[3], 0, "chunk (0,0) was never generated");
        let grass = px(16 + 1, 1);
        assert!(grass[1] > grass[0] && grass[1] > grass[2] && grass[3] == 255, "grass is green: {grass:?}");
        let water = px(16 + 5, 5);
        assert!(water[2] > water[0] && water[2] > water[1], "the pond is blue: {water:?}");
        let pillar = px(16 + 12, 12);
        assert!(pillar[0] == pillar[1] && pillar[1] == pillar[2], "the pillar is grey: {pillar:?}");
        // Taller than its northern neighbour: lit; the block south of it, lower: shaded.
        let (lit, after) = (px(16 + 12, 12), px(16 + 12, 13));
        assert!(lit[0] > colour("minecraft:stone")[0] && after[1] < grass[1], "{lit:?} {after:?}");
        // Kept, and only redrawn when the region changes.
        let cache = t.path().join("cache");
        let a = tile(&rd, &cache, 0, 0, Dim::Overworld).unwrap();
        assert!(cache.join("overworld/r.0.0.png").is_file());
        assert_eq!(tile(&rd, &cache, 0, 0, Dim::Overworld).unwrap(), a);
        assert!(tile(&rd, &cache, 5, 5, Dim::Overworld).is_err());
        assert_eq!(colour("minecraft:oak_leaves"), [56, 118, 32]);
        assert_eq!(colour("minecraft:red_wool"), [153, 51, 51]);
        assert_eq!(colour("minecraft:light_blue_concrete"), [102, 153, 216]);
        assert_eq!(colour("minecraft:spruce_planks"), [129, 86, 49]);
    }

    /// Draws a real server's world (OMNIHUB_MC_WORLD=<server folder>) into OMNIHUB_MC_MAP.
    #[test]
    #[ignore]
    fn real_world_map() {
        let (Ok(srv), Ok(out)) = (std::env::var("OMNIHUB_MC_WORLD"), std::env::var("OMNIHUB_MC_MAP")) else { return };
        let srv = PathBuf::from(srv);
        for dim in [Dim::Overworld, Dim::Nether, Dim::End] {
            let Some(rd) = region_dir(&srv, "world", dim) else { continue };
            for (x, z) in regions(&rd) {
                let t0 = std::time::Instant::now();
                let img = render(&rd, x, z, dim).unwrap();
                let drawn = img.pixels().filter(|p| p.0[3] == 255).count();
                println!("{} r.{x}.{z}: {drawn} blocks in {:?}", dim.id(), t0.elapsed());
                img.save(Path::new(&out).join(format!("{}-{x}-{z}.png", dim.id()))).unwrap();
            }
        }
        println!("spawn {:?}", spawn(&srv, "world"));
    }
}
