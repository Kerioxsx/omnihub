//! What the PC is built from, read the way Windows itself shows it and
//! nothing more intrusive: the firmware's SMBIOS tables (motherboard, case
//! type, every memory stick), the registry (processor, disks), DXGI
//! (graphics cards) and the list of connected USB devices (coolers, fan and
//! RGB hubs). No driver, no administrator rights, nothing written.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RamStick {
    /// Slot name ("DIMM_A2").
    pub slot: String,
    pub size_gb: f64,
    /// MT/s as configured (falls back to the rated speed).
    pub speed: u32,
    /// MT/s the stick reports as its own: the standard speed, which for
    /// EXPO and XMP kits is the slower one they run at until it is turned on.
    #[serde(default)]
    pub rated: u32,
    pub maker: String,
    pub part: String,
    /// "DDR4", "DDR5"…
    pub kind: String,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Chassis {
    #[default]
    Unknown,
    Desktop,
    Tower,
    MiniTower,
    SmallFormFactor,
    AllInOne,
    Laptop,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum BoardSize {
    EAtx,
    #[default]
    Atx,
    MicroAtx,
    MiniItx,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CoolerKind {
    #[default]
    Unknown,
    Air,
    /// All-in-one liquid cooler.
    Aio,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DeviceRole {
    #[default]
    Other,
    /// A liquid cooler's pump.
    Cooler,
    /// Fan and/or RGB controller.
    Hub,
    /// A case's own screen or a cooler's LCD.
    Screen,
}

/// A cooler, hub or screen recognised among the USB devices.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KnownDevice {
    pub vendor_id: u16,
    pub product_id: u16,
    pub name: String,
    pub role: DeviceRole,
    /// Radiator length for coolers, when the model says it.
    pub radiator_mm: Option<u16>,
    /// It has an LCD screen.
    pub lcd: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Disk {
    pub name: String,
    pub nvme: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Specs {
    pub cpu: String,
    pub cores: usize,
    pub threads: usize,
    pub gpus: Vec<String>,
    pub ram: Vec<RamStick>,
    pub ram_gb: f64,
    pub board_maker: String,
    pub board: String,
    pub board_size: BoardSize,
    /// The PC maker's own name for it (prebuilts and laptops).
    pub system: String,
    pub chassis: Chassis,
    pub cooler: CoolerKind,
    pub cooler_name: String,
    pub radiator_mm: Option<u16>,
    pub devices: Vec<KnownDevice>,
    pub disks: Vec<Disk>,
    pub displays: Vec<crate::games::pc::Display>,
}

// ---------- SMBIOS ----------

/// One SMBIOS structure: its formatted bytes and its strings.
#[derive(Debug, Clone)]
pub struct Structure<'a> {
    pub kind: u8,
    pub data: &'a [u8],
    pub strings: Vec<&'a [u8]>,
}

impl Structure<'_> {
    pub fn byte(&self, at: usize) -> Option<u8> {
        self.data.get(at).copied()
    }
    pub fn word(&self, at: usize) -> Option<u16> {
        Some(u16::from_le_bytes([*self.data.get(at)?, *self.data.get(at + 1)?]))
    }
    pub fn dword(&self, at: usize) -> Option<u32> {
        Some(u32::from_le_bytes(self.data.get(at..at + 4)?.try_into().ok()?))
    }
    /// The string a byte at `at` points to (1-based; 0 means none).
    pub fn string(&self, at: usize) -> String {
        let i = self.byte(at).unwrap_or(0) as usize;
        if i == 0 {
            return String::new();
        }
        self.strings.get(i - 1).map(|s| String::from_utf8_lossy(s).trim().to_string()).unwrap_or_default()
    }
}

/// Split the table SMBIOS hands over (`RawSMBIOSData`'s table part) into
/// its structures.
pub fn structures(table: &[u8]) -> Vec<Structure<'_>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 4 <= table.len() {
        let kind = table[i];
        let len = table[i + 1] as usize;
        if len < 4 || i + len > table.len() {
            break;
        }
        let data = &table[i..i + len];
        // Strings: NUL-terminated, the set ends with an empty one.
        let mut j = i + len;
        let mut strings = Vec::new();
        loop {
            let Some(end) = table[j..].iter().position(|&b| b == 0) else { return out };
            if end == 0 {
                j += 1;
                break;
            }
            strings.push(&table[j..j + end]);
            j += end + 1;
        }
        // A structure without strings still ends with two NULs.
        if strings.is_empty() && table.get(j) == Some(&0) {
            j += 1;
        }
        out.push(Structure { kind, data, strings });
        if kind == 127 {
            break;
        }
        i = j;
    }
    out
}

fn junk(s: &str) -> bool {
    let l = s.to_ascii_lowercase();
    l.is_empty() || l.contains("to be filled") || l.contains("default string") || l == "system product name" || l == "system manufacturer" || l == "not specified" || l == "unknown" || l.chars().all(|c| c == '0' || c == ' ')
}

fn clean(s: String) -> String {
    if junk(&s) {
        String::new()
    } else {
        s
    }
}

/// What the SMBIOS tables say.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Firmware {
    pub system: String,
    pub board_maker: String,
    pub board: String,
    pub chassis: Chassis,
    pub cpu: String,
    pub ram: Vec<RamStick>,
}

fn memory_kind(code: u8) -> &'static str {
    match code {
        0x12 => "DDR",
        0x13 => "DDR2",
        0x18 => "DDR3",
        0x1A => "DDR4",
        0x1B => "LPDDR",
        0x1C => "LPDDR2",
        0x1D => "LPDDR3",
        0x1E => "LPDDR4",
        0x22 => "DDR5",
        0x23 => "LPDDR5",
        _ => "",
    }
}

pub fn firmware(table: &[u8]) -> Firmware {
    let mut f = Firmware::default();
    let mut system_maker = String::new();
    for s in structures(table) {
        match s.kind {
            1 => {
                system_maker = clean(s.string(4));
                f.system = clean(s.string(5));
            }
            2 if f.board.is_empty() => {
                f.board_maker = clean(s.string(4));
                f.board = clean(s.string(5));
            }
            3 => {
                f.chassis = match s.byte(5).unwrap_or(0) & 0x7f {
                    3..=5 => Chassis::Desktop,
                    7 | 0x11 | 0x17 => Chassis::Tower,
                    6 => Chassis::MiniTower,
                    0x0F | 0x10 | 0x23 | 0x24 => Chassis::SmallFormFactor,
                    0x0D => Chassis::AllInOne,
                    8..=0x0E | 0x1E | 0x1F | 0x20 => Chassis::Laptop,
                    _ => Chassis::Unknown,
                };
            }
            4 if f.cpu.is_empty() => f.cpu = clean(s.string(0x10)),
            17 => {
                let size = s.word(0x0C).unwrap_or(0);
                if size == 0 || size == 0xFFFF {
                    continue;
                }
                let mb = if size == 0x7FFF {
                    s.dword(0x1C).unwrap_or(0) as f64
                } else if size & 0x8000 != 0 {
                    (size & 0x7FFF) as f64 / 1024.0
                } else {
                    size as f64
                };
                let rated = match s.word(0x15) {
                    Some(0xFFFF) => s.dword(0x54).unwrap_or(0),
                    Some(v) => v as u32,
                    None => 0,
                };
                let configured = match s.word(0x20) {
                    Some(0xFFFF) => s.dword(0x58).unwrap_or(0),
                    Some(v) => v as u32,
                    None => 0,
                };
                f.ram.push(RamStick {
                    slot: s.string(0x10),
                    size_gb: (mb / 1024.0 * 10.0).round() / 10.0,
                    speed: if configured > 0 { configured } else { rated },
                    rated,
                    maker: clean(s.string(0x17)),
                    part: clean(s.string(0x1A)),
                    kind: memory_kind(s.byte(0x12).unwrap_or(0)).to_string(),
                });
            }
            _ => {}
        }
    }
    if !f.system.is_empty() && !system_maker.is_empty() && !f.system.to_ascii_lowercase().contains(&system_maker.to_ascii_lowercase()) {
        f.system = format!("{system_maker} {}", f.system);
    }
    f
}

/// A motherboard's size, from how makers name them ("B650M", "Z790-I",
/// "mATX", "ITX").
/// Memory speeds kits are sold at, MT/s.
const KIT_SPEEDS: &[u32] = &[
    2133, 2400, 2666, 2933, 3000, 3200, 3466, 3600, 3733, 3866, 4000, 4133, 4266, 4400, 4600, 4800, 5200, 5600, 6000, 6200, 6400, 6600, 6800, 7000, 7200, 7600, 7800, 8000, 8200, 8400, 8600, 8800,
];

/// The speed a memory kit is sold at, from its part number: G.Skill
/// "F5-6000J3038F16G", Corsair "CMK32GX5M2B6000C36" and TeamGroup
/// "FF3D532G6000HC38" spell it out; Kingston "KF560C36" and Crucial
/// "CP16G60C36" give it in hundreds before the CAS latency. None for
/// part numbers that don't say (most standard-speed memory).
pub fn kit_speed(part: &str) -> Option<u32> {
    let b = part.as_bytes();
    let mut best = None;
    let mut i = 0;
    while i < b.len() {
        if !b[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        let run = &part[start..i];
        let before_cl = b.get(i) == Some(&b'C') && b.get(i + 1).is_some_and(u8::is_ascii_digit);
        let speed = match run.len() {
            4 => run.parse::<u32>().ok().filter(|s| KIT_SPEEDS.contains(s)),
            // Hundreds, after the DDR generation for Kingston ("KF560C36").
            2 | 3 if before_cl && (run.len() == 2 || run.starts_with(['4', '5'])) => run[run.len() - 2..].parse::<u32>().ok().and_then(|code| KIT_SPEEDS.iter().copied().find(|s| s / 100 == code)),
            _ => None,
        };
        best = best.max(speed);
    }
    best
}

/// Every memory stick, from the firmware's tables alone (quicker than
/// `detect`).
pub fn memory() -> Vec<RamStick> {
    imp::smbios().map(|t| firmware(&t).ram).unwrap_or_default()
}

pub fn board_size(name: &str) -> BoardSize {
    let n = name.to_ascii_uppercase();
    if n.contains("ITX") || n.contains("-I ") || n.ends_with("-I") || n.contains("-I GAMING") || n.contains(" I GAMING") {
        return BoardSize::MiniItx;
    }
    if n.contains("E-ATX") || n.contains("EATX") || n.contains("EXTREME") {
        return BoardSize::EAtx;
    }
    if n.contains("MATX") || n.contains("MICRO") {
        return BoardSize::MicroAtx;
    }
    // A chipset with M after it: B650M, B760M-A, A620M, H610M.
    let b = n.as_bytes();
    for i in 0..b.len().saturating_sub(4) {
        let chipset = matches!(b[i], b'A' | b'B' | b'H' | b'Q' | b'X' | b'Z') && b[i + 1..i + 4].iter().all(u8::is_ascii_digit);
        let before = i == 0 || !b[i - 1].is_ascii_alphanumeric();
        let after = |k: usize| b.get(i + 5 + k).is_none_or(|c| !c.is_ascii_alphabetic());
        if chipset && before {
            // B650M, B760M-A / B650I, Z790I.
            match b.get(i + 4) {
                Some(b'M') if after(0) => return BoardSize::MicroAtx,
                Some(b'I') if after(0) => return BoardSize::MiniItx,
                _ => {}
            }
        }
    }
    BoardSize::Atx
}

// ---------- USB devices ----------

/// (vendor, product, name, role, radiator mm, has an LCD).
type Known = (u16, u16, &'static str, DeviceRole, Option<u16>, bool);

/// Coolers, hubs and screens worth drawing (and, for some, controlling).
const KNOWN: &[Known] = &[
    (0x1E71, 0x170E, "NZXT Kraken X (X42–X72)", DeviceRole::Cooler, None, false),
    (0x1E71, 0x1715, "NZXT Kraken M22", DeviceRole::Cooler, Some(120), false),
    (0x1E71, 0x2007, "NZXT Kraken X53/X63/X73", DeviceRole::Cooler, None, false),
    (0x1E71, 0x2014, "NZXT Kraken X53/X63/X73", DeviceRole::Cooler, None, false),
    (0x1E71, 0x3008, "NZXT Kraken Z53/Z63/Z73", DeviceRole::Cooler, None, true),
    (0x1E71, 0x300C, "NZXT Kraken Elite (2023)", DeviceRole::Cooler, None, true),
    (0x1E71, 0x300E, "NZXT Kraken (2023)", DeviceRole::Cooler, None, true),
    (0x1E71, 0x3012, "NZXT Kraken Elite RGB (2024)", DeviceRole::Cooler, None, true),
    (0x1E71, 0x3014, "NZXT Kraken Plus (2024)", DeviceRole::Cooler, None, false),
    (0x1E71, 0x1714, "NZXT Smart Device", DeviceRole::Hub, None, false),
    (0x1E71, 0x1711, "NZXT Grid+ V3", DeviceRole::Hub, None, false),
    (0x1E71, 0x2006, "NZXT Smart Device V2", DeviceRole::Hub, None, false),
    (0x1E71, 0x200D, "NZXT Smart Device V2", DeviceRole::Hub, None, false),
    (0x1E71, 0x200F, "NZXT Smart Device V2", DeviceRole::Hub, None, false),
    (0x1E71, 0x2001, "NZXT HUE 2", DeviceRole::Hub, None, false),
    (0x1E71, 0x2002, "NZXT HUE 2 Ambient", DeviceRole::Hub, None, false),
    (0x1E71, 0x2009, "NZXT RGB & Fan Controller", DeviceRole::Hub, None, false),
    (0x1E71, 0x200E, "NZXT RGB & Fan Controller", DeviceRole::Hub, None, false),
    (0x1E71, 0x2010, "NZXT RGB & Fan Controller", DeviceRole::Hub, None, false),
    (0x1E71, 0x2011, "NZXT RGB & Fan Controller", DeviceRole::Hub, None, false),
    (0x1E71, 0x2019, "NZXT RGB & Fan Controller", DeviceRole::Hub, None, false),
    (0x1E71, 0x201F, "NZXT RGB & Fan Controller", DeviceRole::Hub, None, false),
    (0x1E71, 0x2020, "NZXT RGB & Fan Controller", DeviceRole::Hub, None, false),
    (0x1E71, 0x2012, "NZXT RGB Controller (2023)", DeviceRole::Hub, None, false),
    (0x1E71, 0x2021, "NZXT RGB Controller (2023)", DeviceRole::Hub, None, false),
    (0x1B1C, 0x0C10, "Corsair Commander Pro", DeviceRole::Hub, None, false),
    (0x1B1C, 0x0C1C, "Corsair Commander Core", DeviceRole::Hub, None, false),
    (0x1B1C, 0x0C2A, "Corsair Commander Core XT", DeviceRole::Hub, None, false),
    (0x1B1C, 0x0C32, "Corsair Commander ST", DeviceRole::Hub, None, false),
    (0x1B1C, 0x0C0B, "Corsair Lighting Node Pro", DeviceRole::Hub, None, false),
    (0x1B1C, 0x0C1A, "Corsair Lighting Node Core", DeviceRole::Hub, None, false),
    (0x1B1C, 0x0C12, "Corsair Hydro H150i Pro", DeviceRole::Cooler, Some(360), false),
    (0x1B1C, 0x0C13, "Corsair Hydro H115i Pro", DeviceRole::Cooler, Some(280), false),
    (0x1B1C, 0x0C15, "Corsair Hydro H100i Pro", DeviceRole::Cooler, Some(240), false),
    (0x1B1C, 0x0C17, "Corsair Hydro H115i Platinum", DeviceRole::Cooler, Some(280), false),
    (0x1B1C, 0x0C18, "Corsair Hydro H100i Platinum", DeviceRole::Cooler, Some(240), false),
    (0x1B1C, 0x0C19, "Corsair Hydro H100i Platinum SE", DeviceRole::Cooler, Some(240), false),
    (0x1B1C, 0x0C20, "Corsair Hydro H100i Pro XT", DeviceRole::Cooler, Some(240), false),
    (0x1B1C, 0x0C21, "Corsair Hydro H115i Pro XT", DeviceRole::Cooler, Some(280), false),
    (0x1B1C, 0x0C22, "Corsair Hydro H150i Pro XT", DeviceRole::Cooler, Some(360), false),
    (0x1B1C, 0x0C29, "Corsair Hydro H60i Pro XT", DeviceRole::Cooler, Some(120), false),
    (0x1B1C, 0x0C35, "Corsair iCUE H100i Elite RGB", DeviceRole::Cooler, Some(240), false),
    (0x1B1C, 0x0C36, "Corsair iCUE H115i Elite RGB", DeviceRole::Cooler, Some(280), false),
    (0x1B1C, 0x0C37, "Corsair iCUE H150i Elite RGB", DeviceRole::Cooler, Some(360), false),
    (0x1B1C, 0x0C40, "Corsair iCUE H100i Elite RGB", DeviceRole::Cooler, Some(240), false),
    (0x1B1C, 0x0C41, "Corsair iCUE H150i Elite RGB", DeviceRole::Cooler, Some(360), false),
    (0x0B05, 0x1988, "ASUS ROG Ryujin II 360", DeviceRole::Cooler, Some(360), true),
    (0x0B05, 0x1AA2, "ASUS ROG Ryujin III 360", DeviceRole::Cooler, Some(360), true),
    (0x0B05, 0x1ADA, "ASUS ROG Ryujin III", DeviceRole::Cooler, Some(360), true),
    (0x0B05, 0x1ADE, "ASUS ROG Ryujin III EVA", DeviceRole::Cooler, Some(360), true),
    (0x0B05, 0x1BCB, "ASUS ROG Ryujin III Extreme", DeviceRole::Cooler, Some(360), true),
    (0x0DB0, 0xB130, "MSI MPG Coreliquid K360", DeviceRole::Cooler, Some(360), true),
    (0x0416, 0x7395, "Lian Li Galahad II LCD", DeviceRole::Cooler, None, true),
    (0x0CF2, 0x7750, "Lian Li Uni Hub SL", DeviceRole::Hub, None, false),
    (0x0CF2, 0xA100, "Lian Li Uni Hub SL", DeviceRole::Hub, None, false),
    (0x0CF2, 0xA101, "Lian Li Uni Hub AL", DeviceRole::Hub, None, false),
    (0x0CF2, 0xA102, "Lian Li Uni Hub SL-Infinity", DeviceRole::Hub, None, false),
    (0x0CF2, 0xA103, "Lian Li Uni Hub SL V2", DeviceRole::Hub, None, false),
    (0x0CF2, 0xA104, "Lian Li Uni Hub AL V2", DeviceRole::Hub, None, false),
    (0x0CF2, 0xA105, "Lian Li Uni Hub SL V2", DeviceRole::Hub, None, false),
];

/// Vendor and product from a device instance id ("USB\VID_1E71&PID_300E\…").
pub fn vid_pid(id: &str) -> Option<(u16, u16)> {
    let u = id.to_ascii_uppercase();
    let v = u.find("VID_")?;
    let p = u.find("PID_")?;
    let hex = |at: usize| u.get(at..at + 4).and_then(|h| u16::from_str_radix(h, 16).ok());
    Some((hex(v + 4)?, hex(p + 4)?))
}

/// The known devices among connected USB/HID device ids, once each.
pub fn known_devices<'a>(ids: impl IntoIterator<Item = &'a str>) -> Vec<KnownDevice> {
    let mut out: Vec<KnownDevice> = Vec::new();
    for id in ids {
        let Some((v, p)) = vid_pid(id) else { continue };
        let Some(&(vendor_id, product_id, name, role, radiator_mm, lcd)) = KNOWN.iter().find(|k| k.0 == v && k.1 == p) else { continue };
        if out.iter().any(|d| d.vendor_id == v && d.product_id == p) {
            continue;
        }
        out.push(KnownDevice { vendor_id, product_id, name: name.into(), role, radiator_mm, lcd });
    }
    out
}

/// Radiator size from a cooler's name ("H150i" 360, "X63" 280, "Z73" 360).
pub fn radiator_from_name(name: &str) -> Option<u16> {
    let n = name.to_ascii_uppercase();
    for (k, mm) in [("420", 420), ("360", 360), ("280", 280), ("240", 240), ("H150", 360), ("H115", 280), ("H100", 240), ("H60", 120), ("X73", 360), ("Z73", 360), ("X63", 280), ("Z63", 280), ("X53", 240), ("Z53", 240)] {
        if n.contains(k) {
            return Some(mm);
        }
    }
    None
}

/// Put the parts together.
pub fn assemble(machine: crate::games::pc::Machine, fw: Firmware, devices: Vec<KnownDevice>, disks: Vec<Disk>) -> Specs {
    let cooler = devices.iter().find(|d| d.role == DeviceRole::Cooler);
    let ram_gb = if fw.ram.is_empty() { machine.ram_gb } else { (fw.ram.iter().map(|r| r.size_gb).sum::<f64>() * 10.0).round() / 10.0 };
    Specs {
        cpu: if machine.cpu.is_empty() { fw.cpu.clone() } else { machine.cpu.clone() },
        cores: machine.cores,
        threads: machine.threads,
        gpus: machine.gpus.into_iter().filter(|g| !g.contains("Microsoft Basic") && !g.contains("Remote Display")).collect(),
        ram: fw.ram,
        ram_gb,
        board_size: board_size(&fw.board),
        board_maker: fw.board_maker,
        board: fw.board,
        system: fw.system,
        chassis: fw.chassis,
        cooler: if cooler.is_some() { CoolerKind::Aio } else { CoolerKind::Unknown },
        cooler_name: cooler.map(|c| c.name.clone()).unwrap_or_default(),
        radiator_mm: cooler.and_then(|c| c.radiator_mm.or_else(|| radiator_from_name(&c.name))),
        devices,
        disks,
        displays: machine.displays,
    }
}

/// This PC's specs.
pub fn detect() -> Specs {
    let machine = crate::games::pc::machine();
    let fw = imp::smbios().map(|t| firmware(&t)).unwrap_or_default();
    let ids = imp::device_ids();
    let devices = known_devices(ids.iter().map(String::as_str));
    assemble(machine, fw, devices, imp::disks())
}

#[cfg(windows)]
mod imp {
    use super::Disk;

    /// The SMBIOS table (without `RawSMBIOSData`'s 8-byte header).
    pub fn smbios() -> Option<Vec<u8>> {
        use windows::Win32::System::SystemInformation::{GetSystemFirmwareTable, FIRMWARE_TABLE_PROVIDER};
        let rsmb = FIRMWARE_TABLE_PROVIDER(u32::from_be_bytes(*b"RSMB"));
        let n = unsafe { GetSystemFirmwareTable(rsmb, 0, None) };
        if n < 8 {
            return None;
        }
        let mut buf = vec![0u8; n as usize];
        let got = unsafe { GetSystemFirmwareTable(rsmb, 0, Some(&mut buf)) };
        if got < 8 {
            return None;
        }
        buf.truncate(got as usize);
        let len = u32::from_le_bytes(buf[4..8].try_into().ok()?) as usize;
        Some(buf[8..(8 + len).min(buf.len())].to_vec())
    }

    /// Instance ids of the USB and HID devices plugged in now.
    pub fn device_ids() -> Vec<String> {
        use windows::core::PCWSTR;
        use windows::Win32::Devices::DeviceAndDriverInstallation::{CM_Get_Device_ID_ListW, CM_Get_Device_ID_List_SizeW, CM_GETIDLIST_FILTER_ENUMERATOR, CM_GETIDLIST_FILTER_PRESENT, CR_SUCCESS};
        let mut out = Vec::new();
        for enumerator in ["USB", "HID"] {
            let filter: Vec<u16> = enumerator.encode_utf16().chain(std::iter::once(0)).collect();
            let flags = CM_GETIDLIST_FILTER_ENUMERATOR | CM_GETIDLIST_FILTER_PRESENT;
            let mut len = 0u32;
            if unsafe { CM_Get_Device_ID_List_SizeW(&mut len, PCWSTR(filter.as_ptr()), flags) } != CR_SUCCESS || len == 0 {
                continue;
            }
            let mut buf = vec![0u16; len as usize];
            if unsafe { CM_Get_Device_ID_ListW(PCWSTR(filter.as_ptr()), &mut buf, flags) } != CR_SUCCESS {
                continue;
            }
            out.extend(buf.split(|&c| c == 0).filter(|s| !s.is_empty()).map(String::from_utf16_lossy));
        }
        out
    }

    /// Disk models, from the storage map Windows keeps in the registry.
    pub fn disks() -> Vec<Disk> {
        use winreg::enums::HKEY_LOCAL_MACHINE;
        use winreg::RegKey;
        let mut out: Vec<Disk> = Vec::new();
        let Ok(scsi) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(r"HARDWARE\DEVICEMAP\Scsi") else { return out };
        // Scsi Port N \ Scsi Bus B \ Target Id T \ Logical Unit Id L
        for port in scsi.enum_keys().flatten() {
            let Ok(pk) = scsi.open_subkey(&port) else { continue };
            let driver: String = pk.get_value("Driver").unwrap_or_default();
            for bus in pk.enum_keys().flatten() {
                let Ok(bk) = pk.open_subkey(&bus) else { continue };
                for target in bk.enum_keys().flatten().filter(|t| t.starts_with("Target Id")) {
                    let Ok(tk) = bk.open_subkey(&target) else { continue };
                    for lun in tk.enum_keys().flatten() {
                        let Ok(lk) = tk.open_subkey(&lun) else { continue };
                        let kind: String = lk.get_value("Type").unwrap_or_default();
                        if kind != "DiskPeripheral" {
                            continue;
                        }
                        let id: String = lk.get_value("Identifier").unwrap_or_default();
                        let name = id.trim().trim_start_matches("NVMe").trim().to_string();
                        if name.is_empty() || out.iter().any(|d| d.name == name) {
                            continue;
                        }
                        out.push(Disk { nvme: driver.eq_ignore_ascii_case("stornvme") || id.starts_with("NVMe"), name });
                    }
                }
            }
        }
        out
    }
}

#[cfg(not(windows))]
mod imp {
    use super::Disk;

    pub fn smbios() -> Option<Vec<u8>> {
        None
    }
    pub fn device_ids() -> Vec<String> {
        Vec::new()
    }
    pub fn disks() -> Vec<Disk> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One SMBIOS structure: header, formatted bytes after it, strings.
    fn s(kind: u8, body: &[u8], strings: &[&str]) -> Vec<u8> {
        let mut v = vec![kind, (4 + body.len()) as u8, 0, 0];
        v.extend_from_slice(body);
        if strings.is_empty() {
            v.extend_from_slice(&[0, 0]);
        } else {
            for st in strings {
                v.extend_from_slice(st.as_bytes());
                v.push(0);
            }
            v.push(0);
        }
        v
    }

    fn memory(slot: u8, size: u16, speed: u16, configured: u16, kind: u8) -> Vec<u8> {
        // Type 17 bytes from offset 4 up to 0x22.
        let mut b = vec![0u8; 0x22 - 4];
        let at = |o: usize| o - 4;
        b[at(0x0C)..at(0x0C) + 2].copy_from_slice(&size.to_le_bytes());
        b[at(0x10)] = slot;
        b[at(0x12)] = kind;
        b[at(0x15)..at(0x15) + 2].copy_from_slice(&speed.to_le_bytes());
        b[at(0x17)] = 2;
        b[at(0x1A)] = 3;
        b[at(0x20)..at(0x20) + 2].copy_from_slice(&configured.to_le_bytes());
        b
    }

    #[test]
    fn kit_speeds_from_part_numbers() {
        for (part, speed) in [
            ("F5-6000J3038F16G", Some(6000)),
            ("F4-3600C16-16GTZNC", Some(3600)),
            ("CMK32GX5M2B6000C36", Some(6000)),
            ("CMW32GX4M2E3200C16", Some(3200)),
            ("FF3D532G6000HC38ADC01", Some(6000)),
            ("KF560C36BBEK2-32", Some(6000)),
            ("KF426C16BB/16", Some(2666)),
            ("KF548C38BBE-16", Some(4800)),
            ("CP16G60C36U5B.M8D1", Some(6000)),
            ("CT16G48C40U5.M8A1", Some(4800)),
            ("M323R2GA3BB0-CQKOL", None),
            ("HMCG78AGBUA081N", None),
            ("", None),
        ] {
            assert_eq!(kit_speed(part), speed, "{part}");
        }
    }

    #[test]
    fn reads_the_board_case_and_every_stick() {
        let mut t = Vec::new();
        t.extend(s(1, &[1, 2, 0, 0], &["To Be Filled By O.E.M.", "To Be Filled By O.E.M."]));
        t.extend(s(2, &[1, 2, 0, 0], &["ASUSTeK COMPUTER INC.", "ROG STRIX B650E-F GAMING WIFI"]));
        t.extend(s(3, &[1, 0x03], &["Default string"]));
        t.extend(s(17, &memory(1, 16384, 4800, 6000, 0x22), &["DIMM_A2", "G.Skill", "F5-6000J3038F16G"]));
        // SMBIOS strings are never empty: an empty slot still names itself.
        t.extend(s(17, &memory(1, 0, 0, 0, 0), &["DIMM_A1", "Unknown", "Unknown"]));
        t.extend(s(17, &memory(1, 16384, 4800, 6000, 0x22), &["DIMM_B2", "G.Skill", "F5-6000J3038F16G"]));
        t.extend(s(127, &[], &[]));
        let f = firmware(&t);
        assert_eq!(f.board_maker, "ASUSTeK COMPUTER INC.");
        assert_eq!(f.board, "ROG STRIX B650E-F GAMING WIFI");
        assert_eq!(f.system, "", "placeholder names are dropped");
        assert_eq!(f.chassis, Chassis::Desktop);
        assert_eq!(f.ram.len(), 2, "empty slots are skipped");
        assert_eq!(f.ram[0], RamStick { slot: "DIMM_A2".into(), size_gb: 16.0, speed: 6000, rated: 4800, maker: "G.Skill".into(), part: "F5-6000J3038F16G".into(), kind: "DDR5".into() });
    }

    #[test]
    fn knows_board_sizes_from_their_names() {
        assert_eq!(board_size("ROG STRIX B650E-F GAMING WIFI"), BoardSize::Atx);
        assert_eq!(board_size("TUF GAMING B650M-PLUS"), BoardSize::MicroAtx);
        assert_eq!(board_size("B760M DS3H"), BoardSize::MicroAtx);
        assert_eq!(board_size("ROG STRIX X670E-I GAMING WIFI"), BoardSize::MiniItx);
        assert_eq!(board_size("B650I AORUS ULTRA"), BoardSize::MiniItx);
        assert_eq!(board_size("X870E AORUS MASTER"), BoardSize::Atx);
        assert_eq!(board_size("MAG B650 TOMAHAWK WIFI"), BoardSize::Atx);
        assert_eq!(board_size("ROG CROSSHAIR X670E EXTREME"), BoardSize::EAtx);
    }

    #[test]
    fn finds_coolers_and_hubs_among_usb_devices() {
        let ids = ["USB\\VID_1E71&PID_300E\\7&2F1C&0&3", "HID\\VID_1E71&PID_300E&MI_01\\8&1", "USB\\VID_046D&PID_C539\\5&1", "USB\\VID_1E71&PID_2019\\9&3"];
        let d = known_devices(ids);
        assert_eq!(d.len(), 2);
        assert_eq!((d[0].name.as_str(), d[0].role, d[0].lcd), ("NZXT Kraken (2023)", DeviceRole::Cooler, true));
        assert_eq!(d[1].role, DeviceRole::Hub);
        assert_eq!(vid_pid("usb\\vid_0b05&pid_1aa2"), Some((0x0B05, 0x1AA2)));
        assert_eq!(radiator_from_name("NZXT Kraken Z73"), Some(360));
        let specs = assemble(crate::games::pc::Machine { cpu: "AMD Ryzen 7 7800X3D 8-Core Processor".into(), gpus: vec!["NVIDIA GeForce RTX 4070 SUPER".into(), "Microsoft Basic Render Driver".into()], ..Default::default() }, Firmware::default(), d, Vec::new());
        assert_eq!(specs.cooler, CoolerKind::Aio);
        assert_eq!(specs.gpus, vec!["NVIDIA GeForce RTX 4070 SUPER".to_string()]);
    }
}
