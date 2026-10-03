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
(they print the record count, total size and scan time) and then:

```
cargo run --release -p omnihub-core --example mftbench -- C:
```

Run the same command as administrator on your own PC to measure your drive.
For comparison, a recursive `FindFirstFile` walk of the same drive is what
the "Standard scan" does (parallel, but bounded by per-directory system
calls) — typically tens of seconds to minutes for millions of files.
