# Benchmarks

## MFT scan

Image built with `sudo scripts/make-bench-image.sh target/bench.img 250000`
(4 GiB NTFS volume, 250,107 files in 4,555 folders, written through
ntfs-3g), measured with `cargo run --release --example mftbench --
target/bench.img` in a 4-vCPU Linux container (virtio disk):

| | Cold cache | Warm cache |
|---|---|---|
| MFT records | 254,712 (5 runs) | |
| Read + parse | **195 ms** | 106 ms |
| Build tree (sort, totals, pre-order) | 63 ms | 63 ms |
| Tree memory | 12.8 MiB | |
| Wildcard search over all names (`*0042*`, 1,967 hits) | 15 ms | 12 ms |

That is ~1.3 million records per second cold, ~2.4 million warm, and the
work is split between one reader thread and a parsing pool, so on a real
NVMe drive the read side is not the limit. Extrapolated, a drive with 5
million files needs on the order of 3–5 s for the first scan and a fraction
of a second for each USN-journal refresh after that (proportional to what
changed, not to the drive size).

## On real Windows

The `windows` CI job runs the live tests against the runner's system drive
and then `cargo run --release -p omnihub-core --example mftbench -- C:`.
GitHub's `windows-latest` runner (cloud VM disk, not NVMe), October 2026:

| | |
|---|---|
| Volume | `C:`, NTFS, 1,384,704 MFT records in 7 runs |
| Contents | 1,153,559 files, 212,304 folders, 140.7 GiB |
| MFT read + parse (release) | **3.28 s** |
| Tree build | 247 ms |
| Tree memory | 83.9 MiB |
| Wildcard search over every name | 179 ms |
| Full scan incl. journal baseline and snapshot save (debug build) | 3.4 s |
| **Incremental refresh** through the USN journal (22 changed records) | **0.40 s** |

Run the same command as administrator on your own PC to measure your drive.
For comparison, a recursive `FindFirstFile` walk of the same drive is what
the "Standard scan" does (parallel, but bounded by per-directory system
calls) — typically tens of seconds to minutes for millions of files.
