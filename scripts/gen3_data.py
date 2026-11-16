#!/usr/bin/env python3
"""Build crates/omnihub-core/src/pkm/gen3.json, the Gen 3 (Ruby, Sapphire,
Emerald, FireRed, LeafGreen) tables the Pokémon save editor needs: species in
the games' own order (with their National Dex number, base stats, growth
rate, gender ratio and abilities), moves with their PP, items, abilities,
the experience tables and the games' text encoding.

Facts only (names, numbers), read from the pret/pokeemerald decompilation:

    python3 -I scripts/gen3_data.py [folder with the downloaded files]

Without a folder the files are downloaded from GitHub.
"""

import json
import os
import re
import sys
import urllib.request

BASE = "https://raw.githubusercontent.com/pret/pokeemerald/master/"
FILES = [
    "include/constants/species.h",
    "include/constants/pokedex.h",
    "src/data/pokemon/species_info.h",
    "src/data/text/species_names.h",
    "include/constants/moves.h",
    "src/data/text/move_names.h",
    "src/data/battle_moves.h",
    "include/constants/items.h",
    "src/data/items.h",
    "include/constants/abilities.h",
    "src/data/text/abilities.h",
    "charmap.txt",
]
OUT = os.path.join(os.path.dirname(__file__), "..", "crates", "omnihub-core", "src", "pkm", "gen3.json")


def read(folder, path):
    name = path.replace("/", "_")
    if folder:
        with open(os.path.join(folder, name), encoding="utf-8") as f:
            return f.read()
    with urllib.request.urlopen(BASE + path) as r:
        return r.read().decode("utf-8")


def title(s):
    """'MASTER BALL' -> 'Master Ball', 'HO-OH' -> 'Ho-Oh', 'TM01' stays."""
    s = s.replace("POKé", "Poké").replace("{PKMN}", "Pkmn")
    out = []
    for word in re.split(r"(\s+|-)", s):
        if not word or word.isspace() or word == "-":
            out.append(word)
        elif any(c.isdigit() for c in word) and word.upper() == word:
            out.append(word)
        else:
            out.append(word[:1].upper() + word[1:].lower())
    return "".join(out)


def defines(text, prefix):
    return {m.group(1): int(m.group(2), 0) for m in re.finditer(r"#define\s+(%s\w+)\s+(\d+|0x[0-9A-Fa-f]+)\b" % prefix, text)}


def enum_values(text, prefix):
    """Values of an enum's members in order (only plain members)."""
    vals, n = {}, 0
    for m in re.finditer(r"^\s*(%s\w+)\s*(?:=\s*([^,\n]+))?,?" % prefix, text, re.M):
        if m.group(2):
            n = int(m.group(2).strip(), 0) if re.fullmatch(r"\s*(\d+|0x[0-9A-Fa-f]+)\s*", m.group(2)) else vals.get(m.group(2).strip(), n)
        vals.setdefault(m.group(1), n)
        n = vals[m.group(1)] + 1
    return vals


def names(text):
    return {m.group(1): m.group(2) for m in re.finditer(r"\[(\w+)\]\s*=\s*_\(\"(.*?)\"\)", text)}


def main():
    folder = sys.argv[1] if len(sys.argv) > 1 else None
    f = {p: read(folder, p) for p in FILES}

    # Species, in the games' order.
    species_ids = defines(f["include/constants/species.h"], "SPECIES_")
    count = species_ids["SPECIES_EGG"]
    dex = enum_values(f["include/constants/pokedex.h"].split("enum {", 2)[1], "NATIONAL_DEX_")
    sp_names = names(f["src/data/text/species_names.h"])
    ability_ids = defines(f["include/constants/abilities.h"], "ABILITY_")
    ability_names = names(f["src/data/text/abilities.h"])
    ability_list = [""] * (max(ability_ids.values()) + 1)
    for k, v in ability_ids.items():
        if k != "ABILITIES_COUNT" and v < len(ability_list):
            ability_list[v] = "" if k == "ABILITY_NONE" else title(ability_names.get(k, ""))
    growth = {"GROWTH_MEDIUM_FAST": 0, "GROWTH_ERRATIC": 1, "GROWTH_FLUCTUATING": 2, "GROWTH_MEDIUM_SLOW": 3, "GROWTH_FAST": 4, "GROWTH_SLOW": 5}
    info = {}
    for chunk in f["src/data/pokemon/species_info.h"].split("[SPECIES_")[1:]:
        m = re.match(r"(\w+)\]\s*=\s*\{(.*)", chunk, re.S)
        if not m or ".baseHP" not in m.group(2):
            continue
        body = m.group(2)
        get = lambda key: re.search(r"\.%s\s*=\s*([^,\n]+)" % key, body)
        stats = [int(get(k).group(1)) for k in ("baseHP", "baseAttack", "baseDefense", "baseSpeed", "baseSpAttack", "baseSpDefense")]
        g = get("genderRatio").group(1).strip()
        pm = re.match(r"PERCENT_FEMALE\(([\d.]+)\)", g)
        gender = min(254, int(float(pm.group(1)) * 255 / 100)) if pm else {"MON_MALE": 0, "MON_FEMALE": 254, "MON_GENDERLESS": 255}[g]
        ab = re.search(r"\.abilities\s*=\s*\{\s*(\w+)\s*,\s*(\w+)\s*\}", body)
        info["SPECIES_" + m.group(1)] = {
            "base": stats,
            "growth": growth[get("growthRate").group(1).strip()],
            "gender": gender,
            "abilities": [ability_list[ability_ids[ab.group(1)]], ability_list[ability_ids[ab.group(2)]]],
        }
    by_id = {v: k for k, v in species_ids.items() if k.startswith("SPECIES_") and k not in ("NUM_SPECIES",)}
    species = []
    for i in range(count + 1):
        key = by_id.get(i)
        nm = sp_names.get(key, "?") if key else "?"
        if i == count:
            nm = "Egg"
        d = dex.get("NATIONAL_DEX_" + key[len("SPECIES_"):], 0) if key and not key.startswith("SPECIES_OLD_UNOWN") else 0
        d = d if d <= 386 else 0
        entry = {"name": title(nm) if nm not in ("?",) else "?", "dex": d if i < count else 0}
        entry.update(info.get(key, {"base": [0] * 6, "growth": 0, "gender": 255, "abilities": ["", ""]}))
        species.append(entry)
    for s in species:
        # The decompilation spells these in capitals with symbols.
        s["name"] = s["name"].replace("Nidoran♀", "Nidoran♀").replace("Nidoran♂", "Nidoran♂")

    # Moves and their PP.
    move_ids = defines(f["include/constants/moves.h"], "MOVE_")
    move_names = names(f["src/data/text/move_names.h"])
    pp = {m.group(1): int(m.group(2)) for m in re.finditer(r"\[(MOVE_\w+)\]\s*=\s*\{[^}]*?\.pp\s*=\s*(\d+)", f["src/data/battle_moves.h"], re.S)}
    n_moves = max(v for k, v in move_ids.items() if k not in ("MOVES_COUNT",) and not k.startswith("MOVE_UNAVAILABLE")) + 1
    by_move = {v: k for k, v in move_ids.items()}
    moves = []
    for i in range(n_moves):
        k = by_move.get(i, "")
        moves.append({"name": "" if i == 0 else title(move_names.get(k, "?")), "pp": pp.get(k, 0)})

    # Items.
    item_enum = f["include/constants/items.h"]
    item_enum = item_enum[item_enum.index("enum {"): item_enum.index("ITEMS_COUNT") + len("ITEMS_COUNT")]
    item_ids = enum_values(item_enum, "ITEM_")
    # Each entry names its own id (TMs are keyed by move name but carry ITEM_TMxx).
    item_names = {m.group(2): m.group(1) for m in re.finditer(r"\.name\s*=\s*_\(\"(.*?)\"\),\s*\.itemId\s*=\s*(ITEM_\w+)", f["src/data/items.h"])}
    n_items = item_ids.get("ITEMS_COUNT", max(item_ids.values()) + 1)
    by_item = {}
    for k, v in item_ids.items():
        by_item.setdefault(v, k)
    items = []
    for i in range(n_items):
        k = by_item.get(i, "")
        nm = item_names.get(k, "")
        items.append("" if i == 0 else (title(nm) if nm and nm != "????????" else ""))

    # Experience for each level, per growth rate (as the games compute it).
    def exp(g, n):
        if n <= 1:
            return n
        c, s = n ** 3, n * n
        if g == 0:
            return c
        if g == 1:
            return (100 - n) * c // 50 if n <= 50 else (150 - n) * c // 100 if n <= 68 else ((1911 - 10 * n) // 3) * c // 500 if n <= 98 else (160 - n) * c // 100
        if g == 2:
            return ((n + 1) // 3 + 24) * c // 50 if n <= 15 else (n + 14) * c // 50 if n <= 36 else ((n // 2) + 32) * c // 50
        if g == 3:
            return 6 * c // 5 - 15 * s + 100 * n - 140
        if g == 4:
            return 4 * c // 5
        return 5 * c // 4

    exp_tables = [[exp(g, n) for n in range(101)] for g in range(6)]

    # Text: byte -> character (English letters, digits and symbols).
    chars = {}
    for line in f["charmap.txt"].splitlines():
        m = re.match(r"^'(.)'\s*=\s*([0-9A-Fa-f]{2})\s*$", line)
        if not m:
            continue
        ch, b = m.group(1), int(m.group(2), 16)
        if 0x3040 <= ord(ch) <= 0x30FF or 0xFF00 <= ord(ch) <= 0xFFEF:
            continue
        chars.setdefault(b, ch)
    charmap = "".join(chars.get(b, "\0") for b in range(256))

    data = {
        "source": "pret/pokeemerald (facts: names and numbers)",
        "species": species,
        "moves": moves,
        "items": items,
        "abilities": ability_list,
        "exp": exp_tables,
        "charmap": charmap,
    }
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w", encoding="utf-8") as fh:
        json.dump(data, fh, ensure_ascii=False, separators=(",", ":"))
    print(f"species {len(species)}, moves {len(moves)}, items {len(items)}, abilities {len(ability_list)} -> {os.path.normpath(OUT)}")


if __name__ == "__main__":
    main()
