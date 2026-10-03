#!/usr/bin/env bash
# Build a large NTFS image for benchmarking the MFT scanner (needs root + ntfs-3g).
#   sudo scripts/make-bench-image.sh target/bench.img 250000
set -euo pipefail
img="${1:-target/bench.img}"
count="${2:-250000}"
mnt="$(mktemp -d)"
trap 'umount "$mnt" 2>/dev/null || true; rmdir "$mnt"' EXIT
rm -f "$img"
truncate -s 4G "$img"
mkntfs -F -Q -q "$img" >/dev/null 2>&1
ntfs-3g -o big_writes "$img" "$mnt"
python3 - "$mnt" "$count" <<'PY'
import os, sys, random
root, count = sys.argv[1], int(sys.argv[2])
random.seed(7)
made = 0
d = 0
while made < count:
    path = os.path.join(root, f"top{d % 40}", f"mid{d % 997}", f"leaf{d}")
    os.makedirs(path, exist_ok=True)
    for i in range(random.randint(20, 200)):
        with open(os.path.join(path, f"file-{i:04d}.dat"), "wb") as f:
            n = random.choice((0, 10, 600, 3000, 70000))
            if n:
                f.write(b"x" * n)
        made += 1
    d += 1
print(f"created {made} files in {d} folders")
PY
umount "$mnt"
