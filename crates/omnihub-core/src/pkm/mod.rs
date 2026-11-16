//! Pokémon save editor for the Game Boy Advance games (Ruby, Sapphire,
//! Emerald, FireRed, LeafGreen): open a save (or a .pk3), edit the trainer
//! and any Pokémon in the party or the PC, and write it back with every
//! checksum right. A copy of the file is kept before each write. The game
//! tables (species, moves, items...) are facts from the pret decompilation,
//! built by scripts/gen3_data.py.

pub mod gen3;

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use gen3::{Pk3, Save3, BALLS, BOXES, BOX_SLOTS, NATURES};

#[derive(Deserialize)]
pub struct SpeciesInfo {
    pub name: String,
    /// National Pokédex number (0: an unused slot or the egg).
    pub dex: u16,
    pub base: [u16; 6],
    pub growth: u8,
    /// 0 male only, 254 female only, 255 genderless, else the female threshold.
    pub gender: u8,
    pub abilities: [String; 2],
}

#[derive(Deserialize)]
pub struct MoveInfo {
    pub name: String,
    pub pp: u8,
}

#[derive(Deserialize)]
struct RawTables {
    species: Vec<SpeciesInfo>,
    moves: Vec<MoveInfo>,
    items: Vec<String>,
    abilities: Vec<String>,
    exp: Vec<Vec<u32>>,
    charmap: String,
}

pub struct Tables {
    pub species: Vec<SpeciesInfo>,
    pub moves: Vec<MoveInfo>,
    pub items: Vec<String>,
    pub abilities: Vec<String>,
    pub exp: Vec<Vec<u32>>,
    /// Byte → character ('\0': none).
    pub chars: Vec<char>,
}

pub fn tables() -> &'static Tables {
    static T: OnceLock<Tables> = OnceLock::new();
    T.get_or_init(|| {
        let r: RawTables = serde_json::from_str(include_str!("gen3.json")).expect("gen3 tables");
        Tables { species: r.species, moves: r.moves, items: r.items, abilities: r.abilities, exp: r.exp, chars: r.charmap.chars().collect() }
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Trainer {
    pub name: String,
    /// 0 boy, 1 girl.
    pub gender: u8,
    pub tid: u16,
    pub sid: u16,
    pub money: u32,
    pub hours: u16,
    pub minutes: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MonView {
    /// The games' own species number.
    pub species: u16,
    pub dex: u16,
    pub species_name: String,
    pub nickname: String,
    pub level: u8,
    pub exp: u32,
    pub nature: u8,
    pub shiny: bool,
    /// "male", "female" or "none".
    pub gender: String,
    pub ability_slot: u8,
    pub ability: String,
    pub item: u16,
    pub friendship: u8,
    pub moves: [u16; 4],
    pub pp: [u8; 4],
    pub pp_ups: [u8; 4],
    pub ivs: [u8; 6],
    pub evs: [u8; 6],
    pub stats: [u16; 6],
    pub ot_name: String,
    pub tid: u16,
    pub sid: u16,
    pub ball: u8,
    pub met_level: u8,
    pub egg: bool,
    pub pid: u32,
    /// Its checksum was right (a "bad egg" otherwise).
    pub valid: bool,
}

/// What to change (unset fields stay as they are).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct MonEdit {
    pub species: Option<u16>,
    pub nickname: Option<String>,
    pub level: Option<u8>,
    pub nature: Option<u8>,
    pub shiny: Option<bool>,
    pub ability_slot: Option<u8>,
    pub item: Option<u16>,
    pub friendship: Option<u8>,
    pub moves: Option<[u16; 4]>,
    pub pp_ups: Option<[u8; 4]>,
    pub ivs: Option<[u8; 6]>,
    pub evs: Option<[u8; 6]>,
    pub ot_name: Option<String>,
    pub tid: Option<u16>,
    pub sid: Option<u16>,
    pub ball: Option<u8>,
}

/// A place for a Pokémon: the party (`box` unset, index 0–5) or a PC box.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Slot {
    #[serde(rename = "box")]
    pub box_: Option<u8>,
    pub index: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BoxView {
    pub name: String,
    pub mons: Vec<Option<MonView>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SaveView {
    pub path: String,
    pub game: String,
    pub trainer: Trainer,
    pub party: Vec<MonView>,
    pub boxes: Vec<BoxView>,
    /// Edits not written to the file yet.
    pub changed: bool,
    /// The file's checksums were all right when opened.
    pub checksums_ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NamedId {
    pub id: u16,
    pub name: String,
    pub dex: u16,
}

/// The lists the editor's pickers show.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PkmTables {
    /// Real species only, in National Dex order.
    pub species: Vec<NamedId>,
    pub moves: Vec<NamedId>,
    pub items: Vec<NamedId>,
    pub natures: Vec<String>,
    pub balls: Vec<String>,
    /// For each species id, its two abilities.
    pub abilities: Vec<[String; 2]>,
}

pub fn pkm_tables() -> PkmTables {
    let t = tables();
    let mut species: Vec<NamedId> = t.species.iter().enumerate().filter(|(_, s)| s.dex != 0).map(|(i, s)| NamedId { id: i as u16, name: s.name.clone(), dex: s.dex }).collect();
    species.sort_by_key(|s| s.dex);
    PkmTables {
        species,
        moves: t.moves.iter().enumerate().filter(|(i, m)| *i == 0 || !m.name.is_empty()).map(|(i, m)| NamedId { id: i as u16, name: if i == 0 { "—".into() } else { m.name.clone() }, dex: m.pp as u16 }).collect(),
        items: t.items.iter().enumerate().filter(|(i, n)| *i == 0 || !n.is_empty()).map(|(i, n)| NamedId { id: i as u16, name: if i == 0 { "None".into() } else { n.clone() }, dex: 0 }).collect(),
        natures: NATURES.iter().map(|s| s.to_string()).collect(),
        balls: BALLS.iter().map(|s| s.to_string()).collect(),
        abilities: t.species.iter().map(|s| s.abilities.clone()).collect(),
    }
}

struct Open {
    path: PathBuf,
    save: Save3,
    changed: bool,
    checksums_ok: bool,
}

/// The save being edited (one at a time).
#[derive(Default)]
pub struct Editor {
    open: Mutex<Option<Open>>,
}

impl Editor {
    pub fn open(&self, path: &Path) -> Result<SaveView, String> {
        let raw = std::fs::read(path).map_err(|e| format!("Couldn't read the save: {e}"))?;
        let save = Save3::parse(raw)?;
        let checksums_ok = save.checksums_ok();
        *self.open.lock() = Some(Open { path: path.to_path_buf(), save, changed: false, checksums_ok });
        self.view()
    }

    pub fn close(&self) {
        *self.open.lock() = None;
    }

    pub fn view(&self) -> Result<SaveView, String> {
        let g = self.open.lock();
        let o = g.as_ref().ok_or("Open a save first")?;
        Ok(SaveView {
            path: o.path.display().to_string(),
            game: o.save.game.label().into(),
            trainer: o.save.trainer(),
            party: o.save.party().iter().map(Pk3::view).collect(),
            boxes: o.save.boxes().into_iter().map(|(name, mons)| BoxView { name, mons: mons.iter().map(|m| (!m.is_empty()).then(|| m.view())).collect() }).collect(),
            changed: o.changed,
            checksums_ok: o.checksums_ok,
        })
    }

    fn with<T>(&self, f: impl FnOnce(&mut Open) -> Result<T, String>) -> Result<T, String> {
        let mut g = self.open.lock();
        let o = g.as_mut().ok_or("Open a save first")?;
        f(o)
    }

    fn get(save: &Save3, slot: Slot) -> Result<Pk3, String> {
        match slot.box_ {
            None => Ok(save.party().get(slot.index as usize).cloned().unwrap_or_else(Pk3::empty)),
            Some(b) if (b as usize) < BOXES && (slot.index as usize) < BOX_SLOTS => Ok(save.box_mon(b as usize, slot.index as usize)),
            _ => Err("No such place".into()),
        }
    }

    fn put(save: &mut Save3, slot: Slot, mon: Pk3) -> Result<(), String> {
        match slot.box_ {
            None => {
                let mut party = save.party();
                let i = slot.index as usize;
                if i > 5 {
                    return Err("The party has 6 places".into());
                }
                if i < party.len() {
                    party[i] = mon;
                } else {
                    party.push(mon);
                }
                save.set_party(&party);
            }
            Some(b) => save.set_box_mon(b as usize, slot.index as usize, &mon),
        }
        Ok(())
    }

    /// Change (or create, in an empty place) a Pokémon.
    pub fn edit(&self, slot: Slot, edit: &MonEdit) -> Result<SaveView, String> {
        self.with(|o| {
            let mut mon = Self::get(&o.save, slot)?;
            if mon.data[0x13] & 1 == 1 || (!mon.is_empty() && !mon.valid) {
                return Err("That's a bad egg (its data is damaged): delete it instead.".into());
            }
            let trainer = o.save.trainer();
            mon.apply(edit, &trainer, o.save.game)?;
            Self::put(&mut o.save, slot, mon)?;
            o.changed = true;
            Ok(())
        })?;
        self.view()
    }

    pub fn delete(&self, slot: Slot) -> Result<SaveView, String> {
        self.with(|o| {
            if slot.box_.is_none() && o.save.party_count() <= 1 {
                return Err("The party needs at least one Pokémon".into());
            }
            match slot.box_ {
                None => {
                    let mut party = o.save.party();
                    if (slot.index as usize) < party.len() {
                        party.remove(slot.index as usize);
                    }
                    o.save.set_party(&party);
                }
                Some(b) => o.save.set_box_mon(b as usize, slot.index as usize, &Pk3::empty()),
            }
            o.changed = true;
            Ok(())
        })?;
        self.view()
    }

    pub fn set_trainer(&self, t: &Trainer) -> Result<SaveView, String> {
        self.with(|o| {
            o.save.set_trainer(t)?;
            o.changed = true;
            Ok(())
        })?;
        self.view()
    }

    /// Bring in a .pk3 (or .3gpkm) file.
    pub fn import(&self, slot: Slot, file: &Path) -> Result<SaveView, String> {
        let raw = std::fs::read(file).map_err(|e| e.to_string())?;
        let mut mon = Pk3::from_file(&raw).ok_or("That isn't a Gen 3 Pokémon file (.pk3)")?;
        mon.refresh_party();
        self.with(|o| {
            Self::put(&mut o.save, slot, mon)?;
            o.changed = true;
            Ok(())
        })?;
        self.view()
    }

    /// Save one Pokémon as a .pk3 file.
    pub fn export(&self, slot: Slot, dest: &Path) -> Result<(), String> {
        let mon = self.with(|o| Self::get(&o.save, slot))?;
        if mon.is_empty() {
            return Err("That place is empty".into());
        }
        crate::settings::write_atomic(dest, &mon.file_bytes()).map_err(|e| e.to_string())
    }

    /// Write the save. The file as it was is kept next to it first
    /// (`name.sav.bak-YYYYMMDD-HHMMSS`); returns that copy's path.
    pub fn write(&self) -> Result<String, String> {
        self.with(|o| {
            let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
            let name = o.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "save.sav".into());
            let backup = o.path.with_file_name(format!("{name}.bak-{stamp}"));
            std::fs::copy(&o.path, &backup).map_err(|e| format!("Couldn't keep a copy of the old save, so nothing was written: {e}"))?;
            let bytes = o.save.bytes();
            crate::settings::write_atomic(&o.path, &bytes).map_err(|e| e.to_string())?;
            o.changed = false;
            o.checksums_ok = true;
            Ok(backup.display().to_string())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_load() {
        let t = tables();
        assert_eq!(t.species[1].name, "Bulbasaur");
        assert_eq!(t.species[277].dex, 252);
        assert_eq!(t.moves[85].name, "Thunderbolt");
        assert_eq!(t.items[68], "Rare Candy");
        assert_eq!(t.chars[0xBB], 'A');
        let p = pkm_tables();
        assert_eq!(p.species.len(), 386);
        assert_eq!(p.species[251].name, "Treecko");
    }

    #[test]
    fn editor_open_edit_write_keeps_a_copy() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("emerald.sav");
        let mut raw = vec![0u8; gen3::SAVE_SIZE];
        for id in 0..14usize {
            let at = id * 0x1000;
            raw[at + 0xFF4..at + 0xFF6].copy_from_slice(&(id as u16).to_le_bytes());
            raw[at + 0xFF8..at + 0xFFC].copy_from_slice(&0x0801_2025u32.to_le_bytes());
        }
        raw[0..3].copy_from_slice(&gen3::encode_text("ASH", 3));
        raw[0xAC..0xB0].copy_from_slice(&0x1234_5678u32.to_le_bytes());
        std::fs::write(&path, &raw).unwrap();

        let ed = Editor::default();
        let v = ed.open(&path).unwrap();
        assert_eq!((v.game.as_str(), v.trainer.name.as_str(), v.party.len()), ("Emerald", "ASH", 0));
        assert!(!v.checksums_ok, "the blank file was never checksummed");
        let party = Slot { box_: None, index: 0 };
        let v = ed.edit(party, &MonEdit { species: Some(6), level: Some(36), shiny: Some(true), ..Default::default() }).unwrap();
        assert!(v.changed && v.party[0].shiny && v.party[0].species_name == "Charizard");
        assert!(ed.delete(party).is_err(), "the last party Pokémon stays");
        let in_box = Slot { box_: Some(2), index: 4 };
        ed.edit(in_box, &MonEdit { species: Some(406), level: Some(70), ..Default::default() }).unwrap();
        let backup = ed.write().unwrap();
        assert!(Path::new(&backup).exists());
        assert_eq!(std::fs::read(&backup).unwrap(), raw, "the copy is the file as it was");

        let again = Editor::default();
        let v = again.open(&path).unwrap();
        assert!(v.checksums_ok && !v.changed);
        assert_eq!(v.party[0].level, 36);
        assert_eq!(v.boxes[2].mons[4].as_ref().unwrap().species_name, "Rayquaza");
        let pk3 = dir.path().join("zard.pk3");
        again.export(party, &pk3).unwrap();
        let v = again.import(Slot { box_: Some(0), index: 0 }, &pk3).unwrap();
        assert_eq!(v.boxes[0].mons[0].as_ref().unwrap().pid, v.party[0].pid);
    }
}
