# Roadmap

<p align="justify">
The order is not random, and the cri­te­ri­on is one: think first, then
build. Every phase must bring non-trivial ex­am­ples with it into the FAQ
(docs/FAQ.md): if a feature has no real ques­tion jus­ti­fy­ing it, it does
not start.
</p>

## Phase 1 — CLI (done, 0.6.x)

- [x] Parallel walker with per-depth aggregation (`--depth`, grid,
      `-top`, `--order`)
- [x] `-f` filters with glob and `REG(...)`
- [x] Notes block at the bottom: NOTE of the `-top` cut, WARNING with
      count and truncation
- [x] `MFD_BENCH` contract for the benches
- [x] C# twin and Node prototype in the comparison

## Phase 2 — TUI (done, 0.7.0)

- [x] Design before implementation: docs/DESIGN-TUI.md, draft 3
      approved
- [x] `mfd-core` extraction: the second consumer taught the API
- [x] `mfd-tui`: interactive navigation of the tree, drill-down, live
      re-sorting
- [x] Two separate binaries (`mfd-cli` + `mfd-tui`) on the shared
      core (see docs/ARCHITECTURE.md)

## Phase 3 — the C# twin, on demand

Today the twin is a proof of concept, and there it stays: the walker,
complete and benchmarked, without the CLI and the TUI around it (the
READMEs' scorecard says why it is kept: the right compromise, a second
opinion at a diff's distance).

The promotion is not on the calendar — it is conditioned to real
interest: **1,000 stars**. Until that many people say they want it, the
POC is the experiment's evidence and nothing more. The accept test is
already written, for the day the stars arrive: the same redirected
ASCII, byte for byte. The C# TUI stays out of the path either way (the
twin's endgame would be the GUI of phase 4, where .NET is at home),
and the roads not taken stay open to the community: the bench's RESULT
lines and the redirected ASCII output are the judge of every
contribution (CONTRIBUTING.md).

- [ ] `Manto.FreeDisk.Cli` promoted from walker to real CLI — gated
      by 1k stars: same help, same grid, same tree, same `-top` as
      the Rust twin
- [ ] The Rust-vs-C# comparison becomes a product comparison, not just
      a walker comparison

## Phase 4 — GUI and app

- [ ] `gui\`: desktop graphical interface on the engine
- [ ] `app\`: distribution to the end user

## Always open

- FAQ-driven development: the entries of docs/FAQ.md drive the features
  (`--json` for before/after diffs: done in 0.7.0, FAQ entry 9)
- The bench results get regenerated and archived in `bench\results\`
