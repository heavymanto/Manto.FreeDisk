# Architecture

<p align="justify">
Manto.FreeDisk was born as an ex­per­i­ment: <b>the same on-disk walker, three
im­ple­men­ta­tions</b> (Node, C#, Rust) to choose the stack with numbers in
hand. The verdict is a score­card where nobody wins in ab­so­lute terms:
<b>Rust the per­for­mance crown</b> (memory above all), <b>Node the porta­bil­i­ty
crown</b>, <b>C# the best sum — the right com­pro­mise</b>; the whole anal­y­sis,
with the ses­sions, is in the READMEs. The C# twin is a proof of concept
today — the walker, com­plete and bench­marked, without the CLI and the TUI
around it (its pro­mo­tion is the roadmap's phase 3); Node is the his­tor­i­cal
phase-0 pro­to­type. The com­par­i­son between the two worlds is part of the
product, not a left­over.
</p>

## The map

```
rust\                the Rust family (Cargo workspace)
  crates\mfd-core\     the engine: walker, tree, filters, warnings
  crates\mfd-cli\      the CLI: parsing, grid, -json, RESULT/ITER lines
  crates\mfd-tui\      the TUI: panels, lens, ratatui event loop
net\                the .NET family: the C# proof of concept (Manto.FreeDisk.Cli, the walker; full twin: roadmap phase 3)
node\mfd-scan\      the phase-0 Node prototype (frozen, historical value)
bench\              micro-benchmark scripts and their results
gui\  app\          reserved for the later phases (see docs/ROADMAP.md)
```

## Engine and skin

The split criterion is sharp: **engine** is everything that understands
the filesystem (parallel walker, per-depth aggregation, filters, warning
collection); **skin** is everything the user sees (argument parsing,
grid, totals line, exit code, RESULT line). The engine prints nothing;
the skins do not touch the filesystem.

The extraction happened in 0.7.0, when the TUI started: it is the
second consumer that teaches the right API to extract, not the other way
around. The engine (`mfd-core`) does not format — numbers and structure
— and the skins (`mfd-cli`, `mfd-tui`) do not touch the filesystem.

## The walker

One shared folder queue (`DirQueue`) and one worker per logical core:

- **push/pop with a Condvar**: workers sleep on an empty queue and wake
  up on notify; the `pending` protocol (an atomic counter of the open
  folders) closes the scan when it drops back to zero — the same
  protocol as the Node and C# walkers.
- **No stat per file**: on Windows `DirEntry::metadata()` reads the data
  that FindFirstFile/FindNextFile already reported during enumeration.
  It is the fast path in userspace, before going down to the $MFT
  engine.
- **Reparse points skipped**: junctions and symlinks are not crossed
  (no double counts, no cycles).
- **Credit aggregation**: every file credits its size and count to the
  chain of its ancestor folders within the requested depth (`--depth`).
  Folders beyond the depth reuse the father's chain: zero allocations
  per folder.

## The contracts not to break

1. **No arguments = the current folder.** `mfd-cli` and `mfd-tui`
   without arguments start from the current folder (like `du`); the help
   stays on `-h`.
2. **`MFD_BENCH=1` = RESULT/ITER only.** `RESULT ms=... bytes=...
   files=... dirs=... errors=...` (and `ITER k/n ...` with loops). The
   scripts in `bench\` parse those lines: changing their format breaks
   the benches.
3. **Pure ASCII output.** No emission depends on the console codepage.
4. **The totals match** across Rust, C# and Node on the same folder: it
   is the cross-check of correctness of `run-bench.ps1`.

## Why two binaries (CLI + TUI) and not one

Since 0.7.0 they are really two binaries on the same `mfd-core`, not
one executable with an interactive default:

- the CLI contract (no arguments = ., pipe, `MFD_BENCH`) stays intact;
- the TUI dependencies (terminal, event loop) stay out of the CLI;
- whoever installs `mfd-cli` gets only the CLI: the measurement on the
  community stays clean.

A possible later phase: an `mfd` launcher that chooses the interface.
To be decided with data.
