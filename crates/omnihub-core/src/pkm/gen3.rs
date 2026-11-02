//! Gen 3 saves (Ruby, Sapphire, Emerald, FireRed, LeafGreen) and their
//! Pokémon (the 80/100-byte "pk3" structure).
//!
//! ```text
//! save (128 KiB): two slots of 14 sections × 4 KiB at 0x0000 and 0xE000;
//!   the slot with the higher save index is the current one. Each section:
//!   data | … | id u16 @FF4 | checksum u16 @FF6 | 0x08012025 @FF8 | index u32 @FFC
//!   checksum: the data's u32 words summed, folded to 16 bits.
//!   0: trainer (name, gender, IDs, play time, Emerald key), 1: party and
//!   money, 5–13: the PC (box 1–14, 30 × 80 bytes each, then box names).
//! pk3: PID | OT ID | nickname | language | flags | OT name | markings |
//!   checksum | 48 bytes in four 12-byte blocks (Growth, Attacks, EVs,
//!   Misc), shuffled by PID % 24 and XOR-ed with PID ^ OT ID; the party
//!   form adds status, level and the stats.
//! ```

use super::{tables, MonEdit, MonView, Trainer};

pub const SAVE_SIZE: usize = 0x20000;
const SECTION: usize = 0x1000;
const SLOT: usize = 14 * SECTION;
const SIGNATURE: u32 = 0x0801_2025;
const DATA_SIZES: [usize; 14] = [0xF2C, 0xF80, 0xF80, 0xF80, 0xF08, 0xF80, 0xF80, 0xF80, 0xF80, 0xF80, 0xF80, 0xF80, 0xF80, 0x7D0];
pub const BOX_MON: usize = 80;
pub const PARTY_MON: usize = 100;
pub const BOXES: usize = 14;
pub const BOX_SLOTS: usize = 30;
const BOX_NAMES: usize = 4 + BOXES * BOX_SLOTS * BOX_MON;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Game {
    RubySapphire,
    Emerald,
    FireRedLeafGreen,
}

impl Game {
    pub fn label(self) -> &'static str {
        match self {
            Game::RubySapphire => "Ruby / Sapphire",
            Game::Emerald => "Emerald",
            Game::FireRedLeafGreen => "FireRed / LeafGreen",
        }
    }
    fn party_offset(self) -> usize {
        if self == Game::FireRedLeafGreen { 0x34 } else { 0x234 }
    }
    fn money_offset(self) -> usize {
        if self == Game::FireRedLeafGreen { 0x290 } else { 0x490 }
    }
    /// The origin game written into new Pokémon.
    fn origin(self) -> u16 {
        match self {
            Game::RubySapphire => 2,
            Game::Emerald => 3,
            Game::FireRedLeafGreen => 4,
        }
    }
}

fn u16_at(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}
fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
fn put16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn put32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}

/// A section's checksum: its data's 32-bit words summed, folded to 16 bits.
pub fn section_checksum(data: &[u8]) -> u16 {
    let sum = data.chunks_exact(4).fold(0u32, |s, w| s.wrapping_add(u32::from_le_bytes([w[0], w[1], w[2], w[3]])));
    ((sum >> 16).wrapping_add(sum & 0xFFFF)) as u16
}

// ---------- text ----------

/// Game text to a string (stops at 0xFF).
pub fn decode_text(b: &[u8]) -> String {
    let map = &tables().chars;
    let s: String = b.iter().take_while(|&&c| c != 0xFF).filter_map(|&c| Some(map[c as usize]).filter(|&ch| ch != '\0')).collect();
    s.trim_end().to_string()
}

/// A string to game text of `len` bytes (unknown characters are left out, the rest is 0xFF).
pub fn encode_text(s: &str, len: usize) -> Vec<u8> {
    let map = &tables().chars;
    let mut out: Vec<u8> = s.chars().filter_map(|ch| map.iter().position(|&c| c == ch && c != '\0').map(|p| p as u8)).take(len).collect();
    out.resize(len, 0xFF);
    out
}

// ---------- pk3 ----------

/// The block order for each PID % 24 (G, A, E, M = 0..3): the 24 orders in sequence.
fn order(pid: u32) -> [usize; 4] {
    let mut perms = Vec::with_capacity(24);
    for a in 0..4 {
        for b in 0..4 {
            for c in 0..4 {
                for d in 0..4 {
                    let p = [a, b, c, d];
                    if (0..4).all(|x| p.contains(&x)) {
                        perms.push(p);
                    }
                }
            }
        }
    }
    perms[(pid % 24) as usize]
}

pub const NATURES: [&str; 25] = ["Hardy", "Lonely", "Brave", "Adamant", "Naughty", "Bold", "Docile", "Relaxed", "Impish", "Lax", "Timid", "Hasty", "Serious", "Jolly", "Naive", "Modest", "Mild", "Quiet", "Bashful", "Rash", "Calm", "Gentle", "Sassy", "Careful", "Quirky"];
pub const BALLS: [&str; 13] = ["", "Master Ball", "Ultra Ball", "Great Ball", "Poké Ball", "Safari Ball", "Net Ball", "Dive Ball", "Nest Ball", "Repeat Ball", "Timer Ball", "Luxury Ball", "Premier Ball"];

/// One Pokémon, decrypted, its blocks in G A E M order at 0x20.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Pk3 {
    pub data: [u8; PARTY_MON],
    /// The stored checksum matched.
    pub valid: bool,
}

const G: usize = 0x20;
const A: usize = 0x2C;
const E: usize = 0x38;
const M: usize = 0x44;

impl Pk3 {
    pub fn empty() -> Pk3 {
        Pk3 { data: [0; PARTY_MON], valid: true }
    }

    /// From the stored (encrypted, shuffled) form: 80 or 100 bytes.
    pub fn decode(raw: &[u8]) -> Pk3 {
        let mut data = [0u8; PARTY_MON];
        let n = raw.len().min(PARTY_MON);
        data[..n].copy_from_slice(&raw[..n]);
        let pid = u32_at(&data, 0);
        let key = pid ^ u32_at(&data, 4);
        let mut plain = [0u8; 48];
        for i in (0..48).step_by(4) {
            put32(&mut plain, i, u32_at(raw, 0x20 + i) ^ key);
        }
        let ord = order(pid);
        for (pos, &block) in ord.iter().enumerate() {
            data[G + block * 12..G + block * 12 + 12].copy_from_slice(&plain[pos * 12..pos * 12 + 12]);
        }
        let mut p = Pk3 { data, valid: true };
        p.valid = p.checksum() == u16_at(&p.data, 0x1C);
        p
    }

    /// From a decrypted file (a .pk3 as other editors save it), or an encrypted one.
    pub fn from_file(raw: &[u8]) -> Option<Pk3> {
        if raw.len() != BOX_MON && raw.len() != PARTY_MON {
            return None;
        }
        let mut data = [0u8; PARTY_MON];
        data[..raw.len()].copy_from_slice(raw);
        let plain = Pk3 { data, valid: true };
        if plain.checksum() == u16_at(&plain.data, 0x1C) && plain.species() != 0 {
            return Some(plain);
        }
        let enc = Pk3::decode(raw);
        (enc.valid && enc.species() != 0).then_some(enc)
    }

    /// The stored form (`party`: 100 bytes, else 80).
    pub fn encode(&self, party: bool) -> Vec<u8> {
        let mut d = self.data;
        if self.is_empty() {
            return vec![0; if party { PARTY_MON } else { BOX_MON }];
        }
        put16(&mut d, 0x1C, self.checksum());
        let pid = u32_at(&d, 0);
        let key = pid ^ u32_at(&d, 4);
        let ord = order(pid);
        let mut shuffled = [0u8; 48];
        for (pos, &block) in ord.iter().enumerate() {
            shuffled[pos * 12..pos * 12 + 12].copy_from_slice(&d[G + block * 12..G + block * 12 + 12]);
        }
        for i in (0..48).step_by(4) {
            put32(&mut d, 0x20 + i, u32_at(&shuffled, i) ^ key);
        }
        d[..if party { PARTY_MON } else { BOX_MON }].to_vec()
    }

    /// The decrypted form, for a .pk3 file (box size).
    pub fn file_bytes(&self) -> Vec<u8> {
        let mut d = self.data;
        put16(&mut d, 0x1C, self.checksum());
        d[..BOX_MON].to_vec()
    }

    pub fn checksum(&self) -> u16 {
        (0..48).step_by(2).fold(0u16, |s, i| s.wrapping_add(u16_at(&self.data, G + i)))
    }

    pub fn is_empty(&self) -> bool {
        self.species() == 0 && self.pid() == 0
    }

    pub fn pid(&self) -> u32 {
        u32_at(&self.data, 0)
    }
    pub fn tid(&self) -> u16 {
        u16_at(&self.data, 4)
    }
    pub fn sid(&self) -> u16 {
        u16_at(&self.data, 6)
    }
    pub fn species(&self) -> u16 {
        u16_at(&self.data, G)
    }
    pub fn item(&self) -> u16 {
        u16_at(&self.data, G + 2)
    }
    pub fn exp(&self) -> u32 {
        u32_at(&self.data, G + 4)
    }
    pub fn pp_ups(&self) -> [u8; 4] {
        let b = self.data[G + 8];
        [b & 3, (b >> 2) & 3, (b >> 4) & 3, (b >> 6) & 3]
    }
    pub fn friendship(&self) -> u8 {
        self.data[G + 9]
    }
    pub fn moves(&self) -> [u16; 4] {
        [0, 1, 2, 3].map(|i| u16_at(&self.data, A + i * 2))
    }
    pub fn pp(&self) -> [u8; 4] {
        [0, 1, 2, 3].map(|i| self.data[A + 8 + i])
    }
    pub fn evs(&self) -> [u8; 6] {
        [0, 1, 2, 3, 4, 5].map(|i| self.data[E + i])
    }
    fn origins(&self) -> u16 {
        u16_at(&self.data, M + 2)
    }
    fn iv_word(&self) -> u32 {
        u32_at(&self.data, M + 4)
    }
    pub fn ivs(&self) -> [u8; 6] {
        let w = self.iv_word();
        [0, 1, 2, 3, 4, 5].map(|i| ((w >> (i * 5)) & 31) as u8)
    }
    pub fn is_egg(&self) -> bool {
        self.iv_word() >> 30 & 1 == 1
    }
    pub fn ability_slot(&self) -> u8 {
        (self.iv_word() >> 31) as u8
    }
    pub fn nature(&self) -> u8 {
        (self.pid() % 25) as u8
    }
    pub fn shiny(&self) -> bool {
        shiny_value(self.pid(), self.tid(), self.sid()) < 8
    }
    pub fn level(&self) -> u8 {
        level_for(self.growth(), self.exp())
    }
    fn growth(&self) -> u8 {
        tables().species.get(self.species() as usize).map_or(0, |s| s.growth)
    }

    fn set_iv_word(&mut self, w: u32) {
        put32(&mut self.data, M + 4, w);
    }

    /// The party block: status, level and stats from the rest.
    pub fn refresh_party(&mut self) {
        let level = self.level();
        let stats = self.stats();
        self.data[0x54] = level;
        put16(&mut self.data, 0x56, stats[0]);
        for (i, s) in stats.iter().enumerate() {
            put16(&mut self.data, 0x58 + i * 2, *s);
        }
    }

    /// HP, Attack, Defense, Speed, Sp. Atk, Sp. Def at the current level.
    pub fn stats(&self) -> [u16; 6] {
        let Some(sp) = tables().species.get(self.species() as usize) else { return [0; 6] };
        let (l, ivs, evs, nat) = (self.level() as u32, self.ivs(), self.evs(), self.nature() as usize);
        let mut out = [0u16; 6];
        for i in 0..6 {
            let core = (2 * sp.base[i] as u32 + ivs[i] as u32 + evs[i] as u32 / 4) * l / 100;
            out[i] = if i == 0 {
                // Shedinja always has 1 HP.
                if self.species() == 303 { 1 } else { (core + l + 10) as u16 }
            } else {
                let v = core + 5;
                let (up, down) = (nat / 5 + 1, nat % 5 + 1);
                (if up == down { v } else if i == up { v * 110 / 100 } else if i == down { v * 90 / 100 } else { v }) as u16
            };
        }
        out
    }

    pub fn view(&self) -> MonView {
        let t = tables();
        let sp = t.species.get(self.species() as usize);
        let gender = match sp.map_or(255, |s| s.gender) {
            255 => "none",
            254 => "female",
            0 => "male",
            r => {
                if (self.pid() & 0xFF) < r as u32 {
                    "female"
                } else {
                    "male"
                }
            }
        };
        let origins = self.origins();
        MonView {
            species: self.species(),
            dex: sp.map_or(0, |s| s.dex),
            species_name: sp.map_or_else(String::new, |s| s.name.clone()),
            nickname: decode_text(&self.data[8..18]),
            level: self.level(),
            exp: self.exp(),
            nature: self.nature(),
            shiny: self.shiny(),
            gender: gender.into(),
            ability_slot: self.ability_slot(),
            ability: sp.map(|s| if self.ability_slot() == 1 && !s.abilities[1].is_empty() { s.abilities[1].clone() } else { s.abilities[0].clone() }).unwrap_or_default(),
            item: self.item(),
            friendship: self.friendship(),
            moves: self.moves(),
            pp: self.pp(),
            pp_ups: self.pp_ups(),
            ivs: self.ivs(),
            evs: self.evs(),
            stats: self.stats(),
            ot_name: decode_text(&self.data[0x14..0x1B]),
            tid: self.tid(),
            sid: self.sid(),
            ball: ((origins >> 11) & 0xF) as u8,
            met_level: (origins & 0x7F) as u8,
            egg: self.is_egg(),
            pid: self.pid(),
            valid: self.valid,
        }
    }

    /// Apply an edit. `trainer` fills in a new Pokémon's original trainer.
    pub fn apply(&mut self, e: &MonEdit, trainer: &Trainer, game: Game) -> Result<(), String> {
        let t = tables();
        let new = self.is_empty();
        let species = e.species.unwrap_or(self.species());
        let sp = t.species.get(species as usize).filter(|s| s.dex != 0).ok_or("Pick a Pokémon")?;
        let level = e.level.unwrap_or(if new { 5 } else { self.level() }).clamp(1, 100);
        if new {
            self.data = [0; PARTY_MON];
            let pid = rand_u32();
            put32(&mut self.data, 0, pid);
            put16(&mut self.data, 4, trainer.tid);
            put16(&mut self.data, 6, trainer.sid);
            self.data[0x12] = 2; // English
            self.data[0x14..0x1B].copy_from_slice(&encode_text(&trainer.name, 7));
            self.data[G + 9] = 70;
            put16(&mut self.data, M + 2, level as u16 | game.origin() << 7 | 4 << 11 | (trainer.gender as u16 & 1) << 15);
            let mut w = 0u32;
            for i in 0..6 {
                w |= (rand_u32() % 32) << (i * 5);
            }
            self.set_iv_word(w);
        }
        let old_name = t.species.get(self.species() as usize).map(|s| s.name.to_uppercase()).unwrap_or_default();
        let (old_level, old_growth) = (self.level(), self.growth());
        let nick = decode_text(&self.data[8..18]);
        put16(&mut self.data, G, species);
        let nickname = match &e.nickname {
            Some(n) if !n.trim().is_empty() && n.trim().to_uppercase() != sp.name.to_uppercase() => n.trim().to_string(),
            Some(_) => sp.name.to_uppercase(),
            None if new || nick == old_name => sp.name.to_uppercase(),
            None => nick,
        };
        self.data[8..18].copy_from_slice(&encode_text(&nickname, 10));
        self.data[0x13] |= 0b10; // has species
        if let Some(n) = &e.ot_name {
            self.data[0x14..0x1B].copy_from_slice(&encode_text(n.trim(), 7));
        }
        if let Some(tid) = e.tid {
            put16(&mut self.data, 4, tid);
        }
        if let Some(sid) = e.sid {
            put16(&mut self.data, 6, sid);
        }
        if let Some(item) = e.item {
            if item as usize >= t.items.len() {
                return Err("Unknown item".into());
            }
            put16(&mut self.data, G + 2, item);
        }
        if let Some(f) = e.friendship {
            self.data[G + 9] = f;
        }
        if let Some(ups) = e.pp_ups {
            self.data[G + 8] = ups.iter().enumerate().fold(0u8, |b, (i, u)| b | (u.min(&3) << (i * 2)));
        }
        let mut moves = e.moves.unwrap_or(self.moves());
        if moves.iter().all(|&m| m == 0) {
            moves[0] = 33; // Tackle: every Pokémon needs a move
        }
        let old_moves = self.moves();
        let ups = self.pp_ups();
        for (i, &m) in moves.iter().enumerate() {
            if m as usize >= t.moves.len() {
                return Err("Unknown move".into());
            }
            put16(&mut self.data, A + i * 2, m);
            if m != old_moves[i] || new || e.pp_ups.is_some() {
                let base = t.moves[m as usize].pp as u32;
                self.data[A + 8 + i] = (base * (5 + ups[i] as u32) / 5) as u8;
            }
        }
        if let Some(evs) = e.evs {
            if evs.iter().map(|&v| v as u32).sum::<u32>() > 510 {
                return Err("EVs add up to more than 510".into());
            }
            self.data[E..E + 6].copy_from_slice(&evs);
        }
        let mut w = self.iv_word();
        if let Some(ivs) = e.ivs {
            w &= 0xC000_0000;
            for (i, &v) in ivs.iter().enumerate() {
                w |= (v.min(31) as u32) << (i * 5);
            }
        }
        if let Some(a) = e.ability_slot {
            w = (w & 0x7FFF_FFFF) | ((a.min(1) as u32 & u32::from(!sp.abilities[1].is_empty())) << 31);
        }
        self.set_iv_word(w);
        if let Some(ball) = e.ball {
            if !(1..=12).contains(&ball) {
                return Err("Unknown ball".into());
            }
            let o = self.origins();
            put16(&mut self.data, M + 2, (o & !(0xF << 11)) | (ball as u16) << 11);
        }
        // Nature and shininess live in the PID: find one that gives both,
        // keeping its lowest byte (which decides gender).
        let nature = e.nature.unwrap_or(self.nature()).min(24);
        let shiny = e.shiny.unwrap_or(self.shiny());
        if nature != self.nature() || shiny != self.shiny() {
            let pid = find_pid(self.pid(), self.tid(), self.sid(), nature, shiny).ok_or("No PID found for that nature")?;
            put32(&mut self.data, 0, pid);
        }
        // Experience: only reset to the level's start when the level (or the
        // species' growth rate) changed, so progress towards the next stays.
        if new || level != old_level || sp.growth != old_growth {
            put32(&mut self.data, G + 4, t.exp[sp.growth as usize][level as usize]);
        }
        self.valid = true;
        self.refresh_party();
        Ok(())
    }
}

fn shiny_value(pid: u32, tid: u16, sid: u16) -> u32 {
    (tid ^ sid ^ (pid >> 16) as u16 ^ (pid & 0xFFFF) as u16) as u32
}

/// A PID with this nature and shininess, keeping the lowest byte of `from`.
pub fn find_pid(from: u32, tid: u16, sid: u16, nature: u8, shiny: bool) -> Option<u32> {
    let low = from & 0xFF;
    let start = from >> 8;
    (0..(1u32 << 24)).map(|i| ((start.wrapping_add(i) & 0xFF_FFFF) << 8) | low).find(|&pid| pid % 25 == nature as u32 && (shiny_value(pid, tid, sid) < 8) == shiny)
}

fn level_for(growth: u8, exp: u32) -> u8 {
    let table = &tables().exp[growth.min(5) as usize];
    (1..=100u8).rev().find(|&l| table[l as usize] <= exp).unwrap_or(1)
}

fn rand_u32() -> u32 {
    rand::random()
}

// ---------- the save ----------

#[derive(Clone)]
pub struct Save3 {
    pub raw: Vec<u8>,
    pub game: Game,
    /// Where each section id (0–13) is in `raw`, in the current slot.
    sections: [usize; 14],
    key: u32,
}

impl Save3 {
    pub fn parse(raw: Vec<u8>) -> Result<Save3, String> {
        if raw.len() < SAVE_SIZE {
            return Err(format!("That isn't a Game Boy Advance Pokémon save (it's {} KB; Gen 3 saves are 128 KB).", raw.len() / 1024));
        }
        let mut best: Option<(u32, [usize; 14])> = None;
        for slot in [0, SLOT] {
            let mut found = [usize::MAX; 14];
            let mut index = 0;
            for s in 0..14 {
                let at = slot + s * SECTION;
                if u32_at(&raw, at + 0xFF8) != SIGNATURE {
                    continue;
                }
                let id = u16_at(&raw, at + 0xFF4) as usize;
                if id < 14 {
                    found[id] = at;
                    index = u32_at(&raw, at + 0xFFC);
                }
            }
            if found.iter().all(|&f| f != usize::MAX) && best.is_none_or(|(i, _)| index > i) {
                best = Some((index, found));
            }
        }
        let (_, sections) = best.ok_or("No Ruby, Sapphire, Emerald, FireRed or LeafGreen save found in that file.")?;
        let code = u32_at(&raw, sections[0] + 0xAC);
        let (game, key) = match code {
            0 => (Game::RubySapphire, 0),
            1 => (Game::FireRedLeafGreen, u32_at(&raw, sections[0] + 0xAF8)),
            k => (Game::Emerald, k),
        };
        Ok(Save3 { raw, game, sections, key })
    }

    fn section(&self, id: usize) -> &[u8] {
        &self.raw[self.sections[id]..self.sections[id] + DATA_SIZES[id]]
    }
    fn section_mut(&mut self, id: usize) -> &mut [u8] {
        let at = self.sections[id];
        &mut self.raw[at..at + DATA_SIZES[id]]
    }

    /// Every section's checksum is right.
    pub fn checksums_ok(&self) -> bool {
        (0..14).all(|id| section_checksum(self.section(id)) == u16_at(&self.raw, self.sections[id] + 0xFF6))
    }

    pub fn trainer(&self) -> Trainer {
        let s0 = self.section(0);
        let s1 = self.section(1);
        let money = u32_at(s1, self.game.money_offset()) ^ self.key;
        Trainer {
            name: decode_text(&s0[0..7]),
            gender: s0[8],
            tid: u16_at(s0, 0x0A),
            sid: u16_at(s0, 0x0C),
            money,
            hours: u16_at(s0, 0x0E),
            minutes: s0[0x10],
        }
    }

    pub fn set_trainer(&mut self, t: &Trainer) -> Result<(), String> {
        if t.name.trim().is_empty() {
            return Err("The trainer needs a name".into());
        }
        if t.money > 999_999 {
            return Err("Money goes up to 999,999".into());
        }
        let key = self.key;
        let money_at = self.game.money_offset();
        let s0 = self.section_mut(0);
        s0[0..7].copy_from_slice(&encode_text(t.name.trim(), 7));
        s0[7] = 0xFF;
        s0[8] = t.gender.min(1);
        put16(s0, 0x0A, t.tid);
        put16(s0, 0x0C, t.sid);
        put16(s0, 0x0E, t.hours.min(999));
        s0[0x10] = t.minutes.min(59);
        put32(self.section_mut(1), money_at, t.money ^ key);
        Ok(())
    }

    pub fn party_count(&self) -> usize {
        (u32_at(self.section(1), self.game.party_offset()) as usize).min(6)
    }

    pub fn party(&self) -> Vec<Pk3> {
        let at = self.game.party_offset() + 4;
        let s1 = self.section(1);
        (0..self.party_count()).map(|i| Pk3::decode(&s1[at + i * PARTY_MON..at + (i + 1) * PARTY_MON])).collect()
    }

    /// Replace the party (at most 6, empty ones dropped).
    pub fn set_party(&mut self, mons: &[Pk3]) {
        let at = self.game.party_offset();
        let mons: Vec<&Pk3> = mons.iter().filter(|m| !m.is_empty()).take(6).collect();
        let s1 = self.section_mut(1);
        put32(s1, at, mons.len() as u32);
        for i in 0..6 {
            let bytes = mons.get(i).map_or(vec![0; PARTY_MON], |m| m.encode(true));
            s1[at + 4 + i * PARTY_MON..at + 4 + (i + 1) * PARTY_MON].copy_from_slice(&bytes);
        }
    }

    /// The PC's bytes (sections 5–13 joined).
    fn pc(&self) -> Vec<u8> {
        (5..14).flat_map(|id| self.section(id).to_vec()).collect()
    }
    fn set_pc(&mut self, pc: &[u8]) {
        let mut at = 0;
        for (id, &n) in DATA_SIZES.iter().enumerate().skip(5) {
            self.section_mut(id).copy_from_slice(&pc[at..at + n]);
            at += n;
        }
    }

    pub fn box_mon(&self, b: usize, i: usize) -> Pk3 {
        let pc = self.pc();
        let at = 4 + (b * BOX_SLOTS + i) * BOX_MON;
        Pk3::decode(&pc[at..at + BOX_MON])
    }

    pub fn set_box_mon(&mut self, b: usize, i: usize, mon: &Pk3) {
        let mut pc = self.pc();
        let at = 4 + (b * BOX_SLOTS + i) * BOX_MON;
        pc[at..at + BOX_MON].copy_from_slice(&mon.encode(false));
        self.set_pc(&pc);
    }

    pub fn boxes(&self) -> Vec<(String, Vec<Pk3>)> {
        let pc = self.pc();
        (0..BOXES)
            .map(|b| {
                let name = decode_text(&pc[BOX_NAMES + b * 9..BOX_NAMES + b * 9 + 9]);
                let mons = (0..BOX_SLOTS).map(|i| Pk3::decode(&pc[4 + (b * BOX_SLOTS + i) * BOX_MON..4 + (b * BOX_SLOTS + i + 1) * BOX_MON])).collect();
                (if name.is_empty() { format!("Box {}", b + 1) } else { name }, mons)
            })
            .collect()
    }

    /// The file to write: every section's checksum brought up to date.
    pub fn bytes(&mut self) -> Vec<u8> {
        for id in 0..14 {
            let sum = section_checksum(self.section(id));
            let at = self.sections[id];
            put16(&mut self.raw, at + 0xFF6, sum);
        }
        self.raw.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A blank Emerald save: slot A, every section signed, with a trainer.
    pub fn blank(game_code: u32) -> Vec<u8> {
        let mut raw = vec![0u8; SAVE_SIZE];
        for id in 0..14 {
            let at = id * SECTION;
            put16(&mut raw, at + 0xFF4, id as u16);
            put32(&mut raw, at + 0xFF8, SIGNATURE);
            put32(&mut raw, at + 0xFFC, 7);
        }
        raw[0..7].copy_from_slice(&encode_text("MAY", 7));
        put16(&mut raw, 0x0A, 12345);
        put16(&mut raw, 0x0C, 54321);
        put32(&mut raw, 0xAC, game_code);
        let mut s = Save3::parse(raw).unwrap();
        s.bytes()
    }

    fn trainer() -> Trainer {
        Trainer { name: "MAY".into(), gender: 1, tid: 12345, sid: 54321, money: 0, hours: 0, minutes: 0 }
    }

    #[test]
    fn checksum_folds() {
        let mut d = vec![0u8; 8];
        put32(&mut d, 0, 0xFFFF_0001);
        put32(&mut d, 4, 0x0001_0002);
        // 0x0000_0003 after wrapping: (0x0000) + (0x0003)
        assert_eq!(section_checksum(&d), 3);
    }

    #[test]
    fn text_round_trip() {
        assert_eq!(decode_text(&encode_text("Pikachu!", 10)), "Pikachu!");
        assert_eq!(encode_text("AB", 4), vec![0xBB, 0xBC, 0xFF, 0xFF]);
    }

    #[test]
    fn orders_are_the_24_permutations() {
        assert_eq!(order(0), [0, 1, 2, 3]); // GAEM
        assert_eq!(order(1), [0, 1, 3, 2]); // GAME
        assert_eq!(order(23), [3, 2, 1, 0]); // MEAG
        assert_eq!(order(24), order(0));
    }

    #[test]
    fn pokemon_encode_decode_and_edits() {
        let t = trainer();
        let mut p = Pk3::empty();
        p.apply(&MonEdit { species: Some(25), level: Some(50), nature: Some(3), shiny: Some(true), ivs: Some([31; 6]), moves: Some([85, 98, 0, 0]), ..Default::default() }, &t, Game::Emerald).unwrap();
        let v = p.view();
        assert_eq!((v.species_name.as_str(), v.level, v.nature, v.shiny), ("Pikachu", 50, 3, true));
        assert_eq!(v.nickname, "PIKACHU");
        assert_eq!(v.pp[0], 15, "Thunderbolt has 15 PP");
        // Stored form is encrypted: decoding gives the same Pokémon, checksum valid.
        let stored = p.encode(true);
        assert_eq!(stored.len(), PARTY_MON);
        assert_ne!(&stored[0x20..0x50], &p.data[0x20..0x50]);
        let back = Pk3::decode(&stored);
        assert!(back.valid);
        assert_eq!(back.view(), Pk3 { valid: true, ..back.clone() }.view());
        assert_eq!(back.view().species, 25);
        // .pk3 files are decrypted; both forms import.
        assert_eq!(Pk3::from_file(&p.file_bytes()).unwrap().view().pid, v.pid);
        assert_eq!(Pk3::from_file(&p.encode(false)).unwrap().view().pid, v.pid);
        // Not shiny any more, same nature and gender.
        let gender = v.gender.clone();
        let mut q = back.clone();
        q.apply(&MonEdit { shiny: Some(false), ..Default::default() }, &t, Game::Emerald).unwrap();
        assert!(!q.shiny() && q.nature() == 3 && q.view().gender == gender);
        assert!(p.apply(&MonEdit { evs: Some([255, 255, 1, 0, 0, 0]), ..Default::default() }, &t, Game::Emerald).is_err());
    }

    #[test]
    fn stats_match_the_games() {
        let t = trainer();
        let mut p = Pk3::empty();
        // Mewtwo, level 100, 31 IVs, no EVs, Hardy: 353 HP, 319 Speed.
        p.apply(&MonEdit { species: Some(150), level: Some(100), ivs: Some([31; 6]), evs: Some([0; 6]), nature: Some(0), ..Default::default() }, &t, Game::Emerald).unwrap();
        let s = p.stats();
        assert_eq!(s[0], 353);
        assert_eq!(s[3], 2 * 130 + 31 + 5);
        assert_eq!(p.exp(), 1_250_000);
    }

    #[test]
    fn save_round_trip_party_boxes_trainer() {
        for code in [0u32, 1, 0xDEAD_BEEF] {
            let mut s = Save3::parse(blank(code)).unwrap();
            assert!(s.checksums_ok());
            let t = s.trainer();
            assert_eq!((t.name.as_str(), t.tid, t.sid), ("MAY", 12345, 54321));
            let mut mon = Pk3::empty();
            mon.apply(&MonEdit { species: Some(277), level: Some(5), ..Default::default() }, &t, s.game).unwrap();
            s.set_party(&[mon.clone()]);
            s.set_box_mon(13, 29, &mon);
            s.set_trainer(&Trainer { money: 123_456, ..t.clone() }).unwrap();
            let bytes = s.bytes();
            let again = Save3::parse(bytes).unwrap();
            assert!(again.checksums_ok());
            assert_eq!(again.trainer().money, 123_456);
            assert_eq!(again.party().len(), 1);
            assert_eq!(again.party()[0].view().species_name, "Treecko");
            assert_eq!(again.box_mon(13, 29).view().species_name, "Treecko");
            assert!(again.box_mon(0, 0).is_empty());
            assert_eq!(again.boxes()[0].0, "Box 1");
        }
    }

    #[test]
    fn newest_slot_wins() {
        let mut raw = blank(0);
        // Copy slot A to slot B with a higher index and another trainer name.
        let a = raw[..SLOT].to_vec();
        raw[SLOT..2 * SLOT].copy_from_slice(&a);
        for id in 0..14 {
            put32(&mut raw, SLOT + id * SECTION + 0xFFC, 8);
        }
        raw[SLOT..SLOT + 7].copy_from_slice(&encode_text("BRENDAN", 7));
        let s = Save3::parse(raw).unwrap();
        assert_eq!(s.trainer().name, "BRENDAN");
        assert!(Save3::parse(vec![0; 1000]).is_err());
        assert!(Save3::parse(vec![0; SAVE_SIZE]).is_err());
    }
}
