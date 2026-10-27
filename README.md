<table>
  <tr>
    <td width="75%" valign="middle">
      <h1>Manto.FreeDisk</h1>
      <h3>Where did my disk space go?</h3>
      <p><strong><em>One engine, two command-line frontends — and an open experiment: the same parallel walker in Rust and C#, measured against each other.</em></strong></p>
      <p>
        <img src="https://img.shields.io/badge/0-STAT%20CALLS-e3b341?style=for-the-badge" alt="0 stat calls" />
        <img src="https://img.shields.io/badge/WORKERS-ONE%20PER%20CORE-e3b341?style=for-the-badge" alt="one worker per core" />
        <img src="https://img.shields.io/badge/OUTPUT-DUF%20%2B%20ASCII-e3b341?style=for-the-badge" alt="duf look plus the ASCII contract" />
      </p>
      <p>
        <img src="https://img.shields.io/badge/walk-181.513%20files%20in%201%2C4%20s-brightgreen" alt="181.513 files in 1,4 s" />
        <img src="https://img.shields.io/badge/frontends-mfd--cli%200.8.0%20%C2%B7%20mfd--tui%200.2.1-blue" alt="mfd-cli 0.8.0 and mfd-tui 0.2.1" />
        <img src="https://img.shields.io/badge/languages-7-blue" alt="seven languages" />
        <img src="https://img.shields.io/badge/license-AGPL--3.0-brightgreen" alt="AGPL-3.0" />
      </p>
      <p><sub>📖 <a href="docs/FAQ.md"><b>The manual</b></a> · 🔧 <a href="#quickstart"><b>Quickstart</b></a> · 🇮🇹 <a href="README.it.md"><b>Leggilo in italiano</b></a></sub></p>
    </td>
    <td width="25%" valign="middle" align="center">
      <img src="docs/img/MNT_C1.png" alt="Manto.FreeDisk" width="150" />
    </td>
  </tr>
</table>

---

The idea was born from a real annoyance. At a time when storage cost
dearly — and a bigger drive was not an option — I kept wondering why my
disk was always full, even after cleaning up with every well-known
tool. None of them gave me an answer at a glance, or with one direct
command. Tired of poor results, I decided to build it myself. I could
have written it in C# — my favorite language, one I have known for
twenty years — but I wanted to try some vibe coding and go with Rust.
GLM 5.3, open weight, on-premise, helped me along. And it worked: with
my own tool I immediately found huge forgotten files — backups, virtual
machines I no longer used — and spotted a good 30% of stale data I
could delete. The experiment below tells the rest of the story.

**Manto.FreeDisk** is a disk-space analyzer built around one question:
*where did my gigabytes go*. It scans a tree with one worker per core,
reads file sizes straight from directory enumeration (no per-file `stat`
on Windows), and turns them into a colored grid, a ranking of the biggest
files, a navigable tree — or a JSON picture machines can diff.

It was born as an open experiment: the same parallel walker written three
times — Node (the phase-0 prototype, now frozen), Rust, and a C# proof of
concept — same folders, same protocol, cross-checked totals:

> Rust and C# run at the same speed — this session even has the C# JIT
> slightly ahead of Rust. Node sits ~6× behind. RAM is where they split:
> Rust walks the tree on 12,5 MB, the C# walker on ~59, Node on ~251 —
> one fifth, one twentieth. That, and a product to ship, is why Rust is
> the engine; the C# twin is a proof of concept today — the walker,
> benchmarked, without the CLI and the TUI around it: its promotion is
> the roadmap's next chapter. Node stays frozen as the phase-0
> prototype. One machine, one session, the raw outputs in
> [bench/results/](bench/results); the method in
> [docs/BENCHMARKS.md](docs/BENCHMARKS.md).

```text
> powershell -ExecutionPolicy Bypass -File bench\run-bench.ps1 -Target C:\Windows -Runs 5
> powershell -ExecutionPolicy Bypass -File bench\mem-bench.ps1 -Target C:\Windows -Runs 5
```

| Implementation | Walk, median (ms) | RAM, peak working set |
| --- | ---: | ---: |
| c# JIT (sizes from the enumeration, no stat) | 1378 | 59,3 MB |
| c# R2R (ReadyToRun, zero JIT) | 1464 | 59,2 MB |
| **rust (find-data via std::fs, no stat)** | **1462** | **12,5 MB** |
| node (readdir+stat per file, libuv) | 8531 | 251,3 MB |

181.513 files, 102.998 folders, totals cross-checked `[OK]` on every row:
the walk is a tie, the memory is not — and that is the verdict of the
the details below.

## Get it

Nothing to install: extract and run — that's it. Both binaries work
right away, in portable mode: English is embedded, and the other six
languages are found in `lang/` next to the binaries.

[![Windows x64](https://img.shields.io/badge/%E2%AC%87-Windows%20x64-2ea44f?style=for-the-badge)](../../releases/download/v0.8.0/mantofreedisk-0.8.0-win-x64.zip)
[![Linux x64](https://img.shields.io/badge/%E2%AC%87-Linux%20x64-2ea44f?style=for-the-badge)](../../releases/download/v0.8.0-linux-x64.tar.gz)
[![macOS arm64](https://img.shields.io/badge/%E2%AC%87-macOS%20arm64-2ea44f?style=for-the-badge)](../../releases/download/v0.8.0/mantofreedisk-0.8.0-macos-aarch64.tar.gz)

**Optional**: if you want the commands available everywhere (not just
from the extracted folder), run `install.ps1` (Windows) or `install.sh`
(Linux). One command, two seconds, done: `mfd-cli` and `mfd-tui` work
from any terminal, with all the languages. Without running it,
everything still works — you just `cd` to the folder first. The SHA256
rides next to each file.

## Quickstart

You need [Rust](https://rustup.rs) (`cargo --version`) and a clone of this
repository. If Rust is not there yet: `winget install Rustlang.Rustup`, then
open a **new** terminal — the ones already open don't see the PATH that
rustup just wrote. Everything else is two commands:

```powershell
cd rust
cargo build --release     # about a minute → mfd-cli.exe and mfd-tui.exe in target\release\
.\target\release\mfd-cli.exe -big 10 C:\Windows
```

Then ask the question this tool exists for — where did the space go? —
and read the answer: ranked, full paths, fixed size-descending order. That is
Windows seen from the inside: one installer package at 348,5 MB on top, and
`msedge.dll` four times, because WinSxS keeps every component version. Add
`-json` and the same picture comes as JSON (`mfd-cli -big 10 -json > big.json`).

<div align="center">
<br />
<img src="docs/img/cli-big.png" alt="The ten biggest files of C:\Windows, ranked with full paths, as they dress on a real terminal" width="100%" />
<sub>🔍 <b>The answer, ranked</b> — mfd-cli -big 10 C:\Windows: full paths, fixed order, and the WARNING block reporting what the scan could not read. (Screenshot taken on Windows, console host.)</sub>
</div>

<details>
<summary><b>Optional:</b> publish the commands (and the seven languages)</summary>

`.\install.ps1` in `rust\` drops the exes into `%USERPROFILE%\.local\bin`
(make sure that folder is in your PATH) and the language files into
`%APPDATA%\mfd\lang`, the registry CLI and TUI discover at runtime —
English is embedded in the binaries, so they work even without that folder.
`install.ps1` never touches your `PATH`. Without installing, run
`rust\target\release\mfd-cli.exe` directly. Prebuilt binaries via GitHub
Releases are planned.

</details>

---

## Two frontends, one engine

| `mfd-cli` — the answer, read | `mfd-tui` — the answer, navigated |
|:---:|:---:|
| ![The grid on a real terminal: mfd-cli -top 10](docs/img/cli-grid.png) | ![mfd-tui: the two panels after the scan](docs/img/tui-start.png) |

| | `mfd-cli` | `mfd-tui` |
| --- | --- | --- |
| **What it is** | the breakdown grid, `duf` style, at any depth | a two-panel navigator over the same scan |
| **Start it** | `mfd-cli <folder>` | `mfd-tui <folder>` |
| **The lens** | `-f` (glob or `REG(...)`), `-o`, `-top` | the same lens: `Shift+S` sort, `Shift+F` filter |
| **The key moves** | `-big N` ranks the biggest files, `-json` photographs | `Shift+X` opens the tree, `Shift+R` the report, `F1` the keyboard |
| **The rest** | `-d a` full tree, 7 languages, the ASCII contract | 4 themes, `Shift+P` JSON print, `F2` saves, the mouse wears keys |
| **Manual** | [FAQ](docs/FAQ.md), entries 1-9 and 11-16 · `mfd-cli -h` | [FAQ](docs/FAQ.md#10-is-there-a-version-to-navigate-not-just-to-read), entry 10 · `F1` in the app |

### `mfd-cli` in action

The colored grid opened this page; here is the rest of the family, as it
looks on the same terminal. (The outputs speak the session's language; the
OS messages inside the WARNING blocks, like "Accesso negato", are passed
through verbatim by the Italian Windows that ran the scans.)

![The whole tree: mfd-cli -d a -top 12](docs/img/cli-tree.png)

![The grid speaks the session's language: -lang it](docs/img/cli-lang.png)

![The filter lens, declared next to the root: -f REG(...)](docs/img/cli-filter.png)

Full reference: `mfd-cli -h`. Real-world examples explained line by line:
[docs/FAQ.md](docs/FAQ.md) — the FAQ is a pillar of this project: every entry
is a real question, a real command, and its real output.

### `mfd-tui` in action

One scan, then the arrows. Each `Shift+X` opens one more branch — the tree
grows a level at a time:

![mfd-tui: branches opened one by one with Shift+X](docs/img/tui-expand.png)

`Shift+A` opens the whole subtree, and `TAB` puts a reading cursor in the
detail:

![mfd-tui: the tree open and the reading cursor in the detail](docs/img/tui-tree.png)

The report window (`Shift+R` / F9) ranks the biggest files of the current
folder — the top is editable with the digits:

![mfd-tui: the report window](docs/img/tui-report.png)

And `F1` documents the whole keyboard in one window:

![mfd-tui: the F1 help](docs/img/tui-help.png)

`F2` saves the settings (theme, lens, bar) to the user profile — the flash
says where:

![mfd-tui: F2 saves the settings](docs/img/tui-save.png)

`Shift+P` / F11 prints the JSON picture of what you are looking at — the
main screen or the report window, the flash says the file:

![mfd-tui: the print from the main screen](docs/img/tui-print-home.png)

The same print from inside the report window:

![mfd-tui: the print from the report window](docs/img/tui-print-report.png)

Four themes, chosen with `Shift+T` / F3 — night blue, Debian installer,
console (the default), CLI:

![The theme menu of the TUI (Shift+T / F3)](docs/img/tui-theme-menu.png)

![The Night theme](docs/img/tui-theme-night.png)

![The Debian theme](docs/img/tui-theme-debian.png)

The filter prompt (`Shift+F` / F7), the language menu (`Shift+L` / F12), and
the scan loader:

![The filter prompt (Shift+F / F7)](docs/img/tui-filter.png)

![The language menu (Shift+L / F12)](docs/img/tui-lang.png)

![The scan loader](docs/img/tui-scan.png)

## The JSON contract

Every picture comes from the same serializer, promoted into the engine
(mfd-core): pure ASCII, deterministic order, byte-for-byte diffable.

| Picture | What it photographs | How |
| --- | --- | --- |
| the grid | the level-1 rows you are looking at | `mfd-cli -json <folder>` |
| the whole tree | every folder, every file — what a diff needs | `mfd-cli -d a -json <folder>` |
| the report | the N biggest files, ranking and full paths | `mfd-cli -big N -json <folder>` |
| the view | both TUI panes, as you left them | `mfd-tui`, `Shift+P` |
| the report window | the open report, top and big files | `Shift+R`, then `Shift+P` |

A real print — the seven-file demo folder, branches closed, the selection
on `alpha` (`mfd-stampa-demo-tui.json`):

```json
{
  "mfd": "0.2.1",
  "root": "C:\\Users\\Public\\mfd-captures\\demo-tui",
  "filter": null,
  "depth": 4294967295,
  "totals": {"bytes": 3530, "files": 7, "dirs": 5, "warnings": 0, "elapsed_ms": 3.7},
  "warnings": [],
  "tree": {
    "bytes": 3530,
    "files": 7,
    "dirs": 5,
    "children": [
      {"name": "alpha", "bytes": 3000, "files": 2, "dirs": 0, "children": null, "leaves": null},
      {"name": "gamma", "bytes": 310, "files": 2, "dirs": 1, "children": null, "leaves": null},
      {"name": "beta", "bytes": 120, "files": 2, "dirs": 1, "children": null, "leaves": null}
    ],
    "leaves": [
      {"name": "readme.txt", "bytes": 100}
    ]
  },
  "detail": {
    "selected": "C:\\Users\\Public\\mfd-captures\\demo-tui\\alpha",
    "bytes": 3000,
    "files": 2,
    "dirs": 0,
    "rows": [
      {"name": "data.dat", "kind": "file", "bytes": 2000},
      {"name": "lib.dll", "kind": "file", "bytes": 1000}
    ]
  }
}
```

The files land in the folder `mfd-tui` was launched from — `mfd-stampa-<name>.json`
for the view, `mfd-report-<name>.json` for the report, numbered on collision:
no silent overwrites, two pictures are worth a diff. The before/after recipe
and the TUI prints are in the FAQ (entries 9 and 12).

## The experiment: same walker, three languages

This repository was born as a stack-selection experiment: the same parallel
walker — Node, C#, Rust — over the same folders, warmup of everything,
interleaved rounds with rotated order, medians, and totals cross-checked
before printing anything. Three benches, three questions
([docs/BENCHMARKS.md](docs/BENCHMARKS.md)): how fast is the walk, how much
RAM does it use, how much does starting cost. The sessions are pasted
whole in [bench/results/](bench/results); the table at the top of this
page is today's. The verdict, on three axes:

### 1 — Performance: CPU and memory

The walk is a tie: C# JIT 1378 ms of median, C# R2R 1464, Rust 1462 —
this session even has the C# a hair ahead, well inside the noise. On a
small folder the C# JIT pays its warmup and looks 4× slower — an
artifact, not a verdict (FAQ, entry 8).

Memory is where they part ways: Rust peaks at 12,5 MB of working set,
the C# walker at ~59, Node at ~251 — one fifth, one twentieth. The
reason is the runtime, not the code: the walker allocates almost nothing
(sizes come straight from the enumeration), but C# carries the .NET
runtime and its garbage collector — a managed heap, JIT structures, GC
generations that trade RAM for allocation speed. Rust has no GC:
ownership frees memory at scope exit, deterministically, and the 12,5 MB
are nearly all buffers. The startup bench tells the same story from
another angle: a native exe starts in ~10 ms, loading the CLR or V8
costs ~60. **The language matters less than the runtime around it.**

### 2 — Robustness, maturity, development

Here the comparison flips. C#/.NET is a twenty-year-old platform: an
immense class library, best-in-class tooling and debugger, a talent pool
everywhere — developing in it is fast and forgiving. Rust is younger
(1.0 in 2015) and steeper: the borrow checker fights you for weeks
before it starts fighting for you — and what it buys is robustness by
construction: data races, use-after-free and null dereferences are
compile errors here, not runtime crashes. Both walkers are rock solid
(the bench cross-checks their totals on every run); the difference is
when you pay for that safety: Rust upfront, in the compiler; C# later,
in tests.

### 3 — Portability

Two different games. .NET has a capability Rust does not: cross-publish
from one machine — from Windows, `dotnet publish -r linux-x64
--self-contained true` produces the Linux executable, any host to any
target, one command (the SDK pulls the runtime pack of the target). Rust
cross-compiles in theory, but like C++ it wants the target's toolchain:
the GNU targets need a cross-linker that Windows does not carry, so in
practice you build on two systems — or on two CI runners, or through
workarounds like musl and zigbuild — and pay the compatibility frictions
each time. What Rust gives back is the artifact: `mfd-cli.exe` is a
single 2 MB file, native, with no runtime to ship or install — .NET's
self-contained binaries carry the runtime with them, tens of MB, and the
R2R publish wants it installed. The process is .NET's game; the artifact
is Rust's. Node wants its runtime too, and carries the heaviest startup
of the three.

### The final standings

| Axis | Rust | C# | Node |
| --- | :---: | :---: | :---: |
| CPU | 🥇 5 | 🥇 5 | 2 |
| RAM | 🥇 5 | 3 | 1 |
| Maturity & ecosystem | 3 | 🥇 5 | 4 |
| Ease of development | 2 | 🥇 5 | 4 |
| Portability | 1 | 4 | 🥇 5 |
| **Total** | **16/25** | **22/25** | **16/25** |
| **The winner, if…** | you are a performance maniac | you want the weighed way | you use it now and then, from one command line |

Nobody wins in absolute terms — every axis has its champion: **Rust
takes the performance crown** (CPU tie, one fifth of the RAM, a single
2 MB binary that starts in ~10 ms), **Node the portability crown** (one
codebase that runs wherever its runtime is — everywhere), and **C# the
best sum: the right compromise** — parity on CPU, a quarter of Node's
memory, a twenty-year-old platform, cross-publishing every target from
one machine. The sum crowns the compromise, not a dominator: the two
purists — the performance one and the portability one — even tie at
16/25.

The first two rows are measured (the sessions in
[bench/results/](bench/results)); the last three are engineering
judgment, declared as such.

The product chose its champion by its own brief: a disk analyzer that
must feel instant and ship as one small binary — so **Rust is the engine
and the product** (mfd-core, mfd-cli, mfd-tui). **The C# twin is a proof
of concept** — the walker, complete and benchmarked, without the CLI and
the TUI around it: its promotion is the roadmap's next chapter
([docs/ROADMAP.md](docs/ROADMAP.md)), and the compromise it embodies is
exactly what a second opinion is for. Node stays frozen as the phase-0
prototype: its portability could not save its 251 MB.

### The road from here

The experiment decided the family's road, and it is explicit:

- **Rust ships first**: the product gets published and stable — that is
  the step that unfreezes everything else.
- **The C# twin grows through the CLI** (roadmap phase 3), when the Rust
  contract is frozen — its acceptance test is already written: the same
  redirected ASCII, byte for byte. **The C# TUI is deliberately out of
  the path**: the twin's endgame is the GUI of phase 4, where .NET is
  at home — the CLI grows the engine, the GUI harvests it.
- **Node stays the frozen patriarch**: walker for the bench, prototype
  for the history. Nothing more is planned for it.

These are the maintainer's choices, not closed doors: **the community is
welcome to walk the roads not taken** — a Node CLI, a C# TUI, ports to
other platforms. The contracts make every contribution verifiable: the
bench's RESULT lines and the redirected ASCII output are the judge, and
the FAQ is the place where a real question becomes a feature
([CONTRIBUTING.md](CONTRIBUTING.md)).

## Known limitations

- **Tested on one machine, one session.** The bench compares the
  implementations with each other; it does not rank computers.
- **Windows and Linux.** Windows carries the no-`stat` fast path (a
  Win32 contract); on Linux the walker falls back to `stat`. macOS is
  next. The bench harness is PowerShell (Windows only).
- Locked folders are warnings, not errors: the grid still shows them (0 B),
  and the totals say how many reports are missing.
- The duf look needs an ANSI terminal; redirected, the output is the
  flat ASCII grid of always (`MFD_DUF=1` forces the look even through a
  pipe, for `less -R`).
- Prebuilt binaries via GitHub Releases are planned: today you build with
  `cargo` — one command, about a minute.
- The TUI speaks 16 colors, by design: the console's own palette.

## Development

```powershell
cargo test                                        # the workspace's tests
powershell -ExecutionPolicy Bypass -File bench\run-bench.ps1 -Target . -Runs 2
```

Architecture notes — engine vs. frontends, the walker, the contracts:
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Repository layout

```text
rust\     Rust family (Cargo workspace): crates\mfd-core (the engine),
          crates\mfd-cli (the CLI), crates\mfd-tui (the interactive TUI)
net\      .NET family: the C# proof of concept — the walker, bench-verified (full twin: roadmap)
node\     phase-0 Node prototype (frozen: history, and one bench implementation)
bench\    benchmark scripts (run / startup / mem) and their results
docs\     ARCHITECTURE - BENCHMARKS - ROADMAP - FAQ
gui\ app\ reserved for the future GUI and app distribution
```

## Documentation

- [docs/FAQ.md](docs/FAQ.md) — real questions, real commands, real outputs
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — engine vs. frontend, the walker, the contracts
- [docs/BENCHMARKS.md](docs/BENCHMARKS.md) — method, scripts, the RESULT contract
- [docs/ROADMAP.md](docs/ROADMAP.md) — TUI, the C# twin promotion, GUI, app
- [docs/DESIGN-TUI.md](docs/DESIGN-TUI.md) — the approved TUI design: two panels, one lens, and since 0.7.1 the tree and the reading cursor

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). The short version: the bench contract
is sacred, naked means the current directory, output is pure ASCII, and every
example in the docs is real. Development is FAQ-driven: if an example feels
awkward, that is a feature request.

## License

Copyright (C) 2026 Francesco Mantovanelli.

This program is free software: you can redistribute it and/or modify it under
the terms of the **GNU Affero General Public License v3.0** as published by the
Free Software Foundation — see [LICENSE](LICENSE). Commercial licensing is
available on request for products that cannot ship under the AGPL.

---

<div align="center">

**Author** — Francesco Mantovanelli · **Project** — Manto.FreeDisk

</div>