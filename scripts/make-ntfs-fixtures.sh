#!/usr/bin/env bash
# Build small NTFS disk images with known contents for the MFT scanner tests.
#
# Needs root (for the FUSE mount), mkntfs and ntfs-3g:
#   sudo apt-get install ntfs-3g
#   sudo scripts/make-ntfs-fixtures.sh target/ntfs-fixtures
#   OMNIHUB_NTFS_FIXTURES=target/ntfs-fixtures cargo test -p omnihub-core --test ntfs_images
#
# Each image gets a JSON manifest written by walking the mounted file system,
# which the test compares against what the MFT parser reports.
set -euo pipefail

out="${1:-target/ntfs-fixtures}"
mkdir -p "$out"
out="$(cd "$out" && pwd)"
mnt="$(mktemp -d)"
trap 'umount "$mnt" 2>/dev/null || true; rmdir "$mnt" 2>/dev/null || true' EXIT

manifest() {
  python3 - "$1" "$2" <<'PY'
import json, os, stat, sys
root, dest = sys.argv[1], sys.argv[2]
entries = []
for dirpath, dirnames, filenames in os.walk(root):
    rel_dir = os.path.relpath(dirpath, root)
    for name in dirnames + filenames:
        full = os.path.join(dirpath, name)
        st = os.lstat(full)
        rel = name if rel_dir == "." else os.path.join(rel_dir, name)
        entries.append({
            "path": rel.replace(os.sep, "\\"),
            "dir": stat.S_ISDIR(st.st_mode),
            "size": 0 if stat.S_ISDIR(st.st_mode) else st.st_size,
            "blocks": st.st_blocks * 512,
            "nlink": st.st_nlink,
            "ino": st.st_ino,
        })
json.dump({"entries": entries}, open(dest, "w"), indent=1)
print(f"{dest}: {len(entries)} entries")
PY
}

make_image() {
  local name="$1" size="$2" cluster="$3" populate="$4"
  local img="$out/$name.img"
  rm -f "$img"
  truncate -s "$size" "$img"
  mkntfs -F -Q -q -c "$cluster" -L "$name" "$img" >/dev/null 2>&1
  ntfs-3g -o streams_interface=windows "$img" "$mnt"
  "$populate" "$mnt"
  sync
  manifest "$mnt" "$out/$name.json"
  umount "$mnt"
}

populate_basic() {
  local m="$1"
  mkdir -p "$m/Users/alice/Documents/Projects/omni" "$m/Users/alice/Downloads" "$m/Program Files/Tool" "$m/Empty Dir"
  printf 'tiny' > "$m/Users/alice/Documents/tiny.txt"                       # resident data
  head -c 300000 /dev/urandom > "$m/Users/alice/Documents/report.pdf"         # non-resident
  head -c 5000000 /dev/urandom > "$m/Users/alice/Downloads/setup.exe"
  head -c 123457 /dev/urandom > "$m/Program Files/Tool/tool.dll"
  printf 'x%.0s' $(seq 1 700) > "$m/Users/alice/Documents/Projects/omni/notes.md"
  echo "unicode" > "$m/Users/alice/Documents/Ünïcødé – 名前 😀.txt"
  echo "long" > "$m/Users/alice/Documents/a_rather_long_file_name_that_needs_more_than_eight_dot_three.extension"
  truncate -s 50M "$m/Users/alice/Downloads/sparse.vhd"                      # sparse
  head -c 20000 /dev/urandom > "$m/Users/alice/Documents/linked.bin"
  ln "$m/Users/alice/Documents/linked.bin" "$m/Users/alice/Downloads/hardlink.bin"
  echo "main" > "$m/Users/alice/Documents/with-stream.txt"
  head -c 200000 /dev/urandom > "$m/Users/alice/Documents/with-stream.txt:Zone.Big"
  # A directory with enough entries to need an index allocation.
  mkdir -p "$m/Users/alice/many"
  for i in $(seq 1 400); do printf '%05d' "$i" > "$m/Users/alice/many/file-$i.log"; done
}

populate_fragmented() {
  # Fill the volume (including the reserved MFT zone) with small data files,
  # free every other one, then create thousands of records: the $MFT has to
  # grow into the scattered holes, so its data stream ends up in many runs.
  local m="$1"
  mkdir -p "$m/data" "$m/records"
  for i in $(seq 1 2500); do head -c 8192 /dev/zero > "$m/data/d$i" 2>/dev/null || break; done
  head -c 200000000 /dev/zero > "$m/filler.bin" 2>/dev/null || true
  for i in $(seq 1 2 2500); do rm -f "$m/data/d$i"; done
  for i in $(seq 1 6000); do : > "$m/records/r$i" 2>/dev/null || break; done
  rm -f "$m/filler.bin"
  for i in $(seq 1 120); do head -c $((i * 37)) /dev/urandom > "$m/records/data$i.bin"; done
}

populate_small_clusters() {
  # 512-byte clusters with 1 KiB records: every record spans two clusters.
  local m="$1"
  mkdir -p "$m/a/b/c/d/e/f/g"
  head -c 77777 /dev/urandom > "$m/a/b/c/d/e/f/g/deep.bin"
  for i in $(seq 1 3000); do : > "$m/a/n$i"; done
  head -c 1048577 /dev/urandom > "$m/a/one-meg-plus-one"
}

make_image basic 96M 4096 populate_basic
make_image fragmented 48M 4096 populate_fragmented
make_image smallclusters 64M 512 populate_small_clusters
echo "fixtures in $out"
