#!/usr/bin/env python3
"""Rebuild public/emoji/3d: the 3D emoji pictures beside Aurora's lyrics.

Every emoji in src/desktop/lib/aurora/emoji.ts is looked up in Microsoft's
Fluent Emoji (MIT licence), its 3D picture downloaded and made a 160x160 WebP
named by its code points (no FE0F), e.g. 1f441.webp for 👁️.

Needs git, curl, node 22+ and ffmpeg with libwebp:

    python3 scripts/emoji-3d.py
"""

import glob
import json
import os
import subprocess
import sys
import tempfile
import urllib.parse

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "public", "emoji", "3d")
REPO = "https://github.com/microsoft/fluentui-emoji.git"
RAW = "https://raw.githubusercontent.com/microsoft/fluentui-emoji/main/assets/"


def run(args, **kw):
    return subprocess.run(args, check=True, **kw)


def emojis(work):
    """[emoji, file] for every emoji the lyrics can show."""
    src = open(os.path.join(ROOT, "src", "desktop", "lib", "aurora", "emoji.ts"), encoding="utf-8").read()
    src = src.replace("import type { EmojiAmount } from '@shared/types';", "type EmojiAmount = 'off' | 'some' | 'more';")
    open(os.path.join(work, "emoji.ts"), "w", encoding="utf-8").write(src)
    open(os.path.join(work, "list.ts"), "w").write("import { ALL_EMOJIS, emojiFile } from './emoji.ts';\nconsole.log(JSON.stringify(ALL_EMOJIS.map((e) => [e, emojiFile(e)])));\n")
    out = run(["node", "--experimental-strip-types", "--no-warnings", os.path.join(work, "list.ts")], capture_output=True, text=True).stdout
    return json.loads(out)


def main():
    work = tempfile.mkdtemp(prefix="omnihub-emoji-")
    fluent = os.path.join(work, "fluent")
    # Only the metadata (folder names and code points), not the pictures.
    run(["git", "clone", "-q", "--filter=blob:none", "--no-checkout", "--depth", "1", REPO, fluent])
    run(["git", "-C", fluent, "sparse-checkout", "set", "--no-cone", "/assets/*/metadata.json", "/LICENSE"])
    run(["git", "-C", fluent, "checkout", "-q"])
    folders = {}
    for f in glob.glob(os.path.join(fluent, "assets", "*", "metadata.json")):
        d = json.load(open(f, encoding="utf-8"))
        folders["-".join(c for c in d.get("unicode", "").split() if c != "fe0f")] = os.path.basename(os.path.dirname(f))
    os.makedirs(OUT, exist_ok=True)
    failed = []
    for emoji, file in emojis(work):
        key = os.path.basename(file)[:-5]
        folder = folders.get(key)
        if not folder:
            failed.append(emoji)
            continue
        listing = run(["git", "-C", fluent, "ls-tree", "-r", "--name-only", "HEAD", "--", f"assets/{folder}"], capture_output=True, text=True).stdout.split("\n")
        # Skin-tone emojis keep the default (yellow) one under Default/.
        pics = [p for p in listing if p.endswith("_3d.png") or p.endswith("_3d_default.png")]
        pics.sort(key=lambda p: ("/Default/" not in p and "_default" in p, len(p)))
        if not pics:
            failed.append(emoji)
            continue
        png = os.path.join(work, key + ".png")
        run(["curl", "-sS", "-f", "-m", "60", "-o", png, RAW + urllib.parse.quote(pics[0][len("assets/"):])])
        run(["ffmpeg", "-loglevel", "error", "-y", "-i", png, "-vf", "scale=160:160:flags=lanczos", "-c:v", "libwebp", "-quality", "86", "-compression_level", "6", "-pix_fmt", "yuva420p", os.path.join(OUT, key + ".webp")])
    with open(os.path.join(OUT, "LICENSE.txt"), "w", encoding="utf-8") as fh:
        fh.write("These 3D emoji pictures are Microsoft's Fluent Emoji (https://github.com/microsoft/fluentui-emoji),\nresized to 160x160 WebP. They are used under the MIT licence below.\n\n")
        fh.write(open(os.path.join(fluent, "LICENSE"), encoding="utf-8").read())
    print("done;", "missing: " + " ".join(failed) if failed else "nothing missing")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
