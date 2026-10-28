# Contributing to Manto.FreeDisk

Thank you for the interest. The project lives on two things: an honest,
measured engine, and a documentation that shows real outputs. Before
opening a PR, read the points below: they are few, and all practical.

## How to build

Rust family (the CLI, `mfd-cli`):

```powershell
cd rust
cargo build --release          # the workspace builds every crate:
                               #   rust\target\release\mfd-cli.exe (the CLI)
                               #   rust\target\release\mfd-tui.exe (the TUI)
.\install.ps1                  # optional: publishes mfd-cli and mfd-tui as user
                               # commands, and the language registry to %APPDATA%\mfd\lang
```

A note for those using the rustup **windows-gnu** toolchain (without a
full MinGW): the `mfd-tui` build pins `parking_lot` at 0.12.4 and
`parking_lot_core` at 0.9.11, because later versions switch to
`windows-link`, which generates the import libraries with `dlltool` on
that target (and the assembler it needs is not in the minimal
toolchain). The pin is commented in `rust\crates\mfd-tui\Cargo.toml`.

.NET family (the C# twin of the comparison):

```powershell
cd net
dotnet build -c Release        # net\src\Manto.FreeDisk.Cli\bin\Release\net10.0\Manto.FreeDisk.Cli.dll
```

The micro-benchmarks (same folder, same logic, three languages):

```powershell
cargo build --release        # in rust\
dotnet build -c Release      # in net\
powershell -ExecutionPolicy Bypass -File bench\run-bench.ps1 -Target <folder> -Runs 3
```

## Issues

Three doors, one workflow: **bug**, **question**, **feature request** —
the templates carry the context the project needs (the command, the
redirected output, the version), and the manual is linked as the first
stop. Questions are the engine of the FAQ-driven development (rule 4
below): the real ones become entries in `docs\FAQ.md`, quoted with their
real output.

A note on expectations: this is a hobby project with one maintainer —
no response times are promised. Everything gets read; answers arrive
when life allows.

## The rules of the game

1. **The bench contract is sacred.** With `MFD_BENCH=1` the output is
   only the `RESULT ms=... bytes=... files=... dirs=... errors=...` line
   (or the `ITER` lines with loops). Any change to that format gets
   discussed first: `bench\run-bench.ps1`, `startup-bench.ps1` and
   `mem-bench.ps1` sit on it.
2. **No arguments = the current folder.** Without arguments `mfd-cli`
   and `mfd-tui` start from the current folder (like `du` and `ncdu`);
   the help stays on `-h`.
3. **Pure ASCII output.** No emission depends on the console codepage:
   the CLI must behave identically in every Windows terminal.
4. **FAQ-driven development.** If an example feels awkward to explain,
   or an option to make it work is missing, that is a feature signal:
   open an issue with the command you wish you could write (see
   `docs\FAQ.md`).
5. **Every example in the documentation was really run.** If you change
   an output, update the documentation that quotes it (README, FAQ).
6. **The documentation speaks English** — one language for everybody,
   with `README.it.md` as the single Italian counterpart and no other
   doc translations in the repo. Code comments, test names and the bench
   scripts stay in the maintainer's voice (Italian): they are
   maintainer-facing, not user-facing.

## License of the contributions

The project is distributed under **GNU AGPLv3** (see `LICENSE`). By
contributing:

- you grant that your contribution is released under the same
  AGPL-3.0-only license as the project;
- you confirm the contribution is your original work, or material you
  have the right to submit under these terms.

No copyright assignment: the contribution stays yours. The confirmation
is needed because the project owner reserves the right to offer
commercial licenses on the code in the future (dual-licensing model,
like iText), which is only possible if contributions arrive with clear
rights.
