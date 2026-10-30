# FAQ — real questions, real commands, real outputs

The rule of this file (and of the project): **every entry is a real question,
the command was really run, the output is pasted whole from a real run**. No
invented examples, nothing "similar to". If an answer takes more imagination
than code, it is not an answer: it is a feature waiting to happen (see the
queue at the bottom).

The outputs are from mfd-cli 0.7.1 on Windows (mfd-tui 0.2.1 for the TUI
entries), in the factory language, English. One detail you will notice right
away: the numbers keep the Italian format — dot for thousands, comma for
decimals, `41,4 GB` — in every language. That is a declared contract, not an
oversight: entry 15 says why.

## Index

1. [Where did my disk space go?](#1-where-did-my-disk-space-go)
2. [What are the notices at the bottom? Did I break something?](#2-what-are-the-notices-at-the-bottom-did-i-break-something)
3. [The first scan is slow, the next ones are fast: why?](#3-the-first-scan-is-slow-the-next-ones-are-fast-why)
4. [I just want the total, no table](#4-i-just-want-the-total-no-table)
5. [How much space do only certain files take? (filters)](#5-how-much-space-do-only-certain-files-take-filters)
6. [The table is long: can I see just a slice?](#6-the-table-is-long-can-i-see-just-a-slice)
7. [How do I use it inside a script?](#7-how-do-i-use-it-inside-a-script)
8. [Rust vs C#: how much does the language really matter?](#8-rust-vs-c-how-much-does-the-language-really-matter)
9. [I want a before/after report of a cleanup](#9-i-want-a-beforeafter-report-of-a-cleanup)
10. [Is there a version to navigate, not just to read?](#10-is-there-a-version-to-navigate-not-just-to-read)
11. [I want the biggest files, not the map](#11-i-want-the-biggest-files-not-the-map)
12. [Can the TUI take the JSON picture, for reporting?](#12-can-the-tui-take-the-json-picture-for-reporting)
13. [What languages do the CLI and the TUI speak?](#13-what-languages-do-the-cli-and-the-tui-speak)
14. [Glob or REG? A field guide to filters](#14-glob-or-reg-a-field-guide-to-filters)
15. [Why do the numbers use Italian separators?](#15-why-do-the-numbers-use-italian-separators)
16. [A full disk hunt, end to end](#16-a-full-disk-hunt-end-to-end)

---

## 1. Where did my disk space go?

The question mfd-cli exists for. Point it at a folder and read the grid:

```
> mfd-cli C:\Windows

--- C:\Windows ----------------------------------------------------------------
       SIZE     %                                  FILES    FOLDERS  NAME
-------------------------------------------------------------------------------
    18,1 GB   43%  ##########..............      122.666     92.331  |-- WinSxS\
     7,1 GB   17%  ####....................       20.068      1.650  |-- System32\
     6,8 GB   16%  ####....................        4.441      3.745  |-- assembly\
     4,4 GB   10%  ###.....................          927        101  |-- Installer\
     1,4 GB    3%  #.......................        7.704      1.943  |-- SystemApps\
     1,2 GB    3%  #.......................        4.909        387  |-- SysWOW64\
   691,5 MB    1%  ........................        2.800      1.929  |-- Microsoft.NET\
   461,0 MB    1%  ........................          658          0  |-- Fonts\

(107 rows in all; here the first ones)
-------------------------------------------------------------------------------
    41,4 GB                                      178.972    103.007  TOTAL: 107 entries
--- THE 10 BIGGEST FILES ------------------------------------------------------
   350,5 MB  C:\Windows\Installer\46390ac1.msi
   348,5 MB  C:\Windows\Installer\3b7172f7.msi
   327,7 MB  C:\Windows\System32\Microsoft-Edge-WebView\msedge.dll
   ...
-------------------------------------------------------------------------------
    WARNING  - opening folder C:\Windows\CSC: Accesso negato.
             ...
             ... and 46 more reports
```

Row by row:

- **The title band on top says where you are** (`--- C:\Windows ---`):
  the scanned path, the only thing the root row used to say. The root is
  not a row anymore — its 100% told nothing the TOTAL doesn't already
  tell. The % of every row is born from that total: the whole tree is
  the 100%, the zero point of the bar.
- **The `|--` rows are the subfolders** (connectors like `tree`): WinSxS
  holds 18,1 GB, that is 43% of the total. The size is RECURSIVE: everything
  inside it, at any depth.
- **The bar is proportional**: 24 characters, `#` full and `.` empty. WinSxS
  at 43% has 10 full characters out of 24: it reads at a glance, without
  looking at the numbers.
- **Below the TOTAL, the ten biggest files** of the whole tree, full
  paths, fixed size-descending order: the grid says where the space
  sits, the ranking says which files hold it — one command, both
  answers. `-report N` prints just this section, `-grid` just the grid
  above.
- **On a real terminal the grid dresses like duf**
  (github.com/muesli/duf) — rounded box, bars and % colored by thresholds,
  blue names, the bold TOTAL row closing the box. A photograph says it
  better than a description:

  ![The grid on a real terminal: mfd-cli -grid -top 10](docs/img/cli-grid.png)

  The names stay whole: the box is as wide as it takes, like the ASCII grid.
  `NO_COLOR` turns the colors off but keeps the box, `MFD_ASCII=1` goes back
  to the flat grid, `MFD_DUF=1` forces the duf look even redirected (for
  `less -R`). Redirected to a file or a pipe, the output is always the flat
  ASCII grid above: the contract of logs, scripts and captures does not
  change — the language follows `-lang` and `MFD_LANG`, the format does not.
- **FILES and FOLDERS are recursive**: System32 holds 20.437 files in 1.651
  folders beneath it. Direct files appear as leaves, with `-` in place of the
  counts (further down in the same table: `|-- setuperr.log`).
- **The default order is size, descending** (`T.D`): the biggest on top,
  which is what you want when you are hunting for space.
- **The TOTAL closes the grid, aligned**: behind a thread of separator,
  every total under its own column — `41,4 GB` under SIZE, `178.972` under
  FILES, `103.007` under FOLDERS — and in the NAME column the label with the
  entry count (`TOTAL: 107 entries`). These are the same numbers the TUI
  detail pane (entry 10) puts on top of the right panel; when the table is
  long, the first rows are already off-screen, and the total stays within
  reach. `-top` leaves it alone too (entry 6): rows are cut, totals are not.
- **The WARNING rows close the whole output**, after the ranking: the first cell
  is the label, the other columns merge into one text cell — entry 2 is about
  them.

To go one level deeper you ask for more depth (`-d 2` and beyond): the rows
carry `|--` for the children and `` `-- `` for the last of each group,
exactly like the `tree` command.

## 2. What are the notices at the bottom? Did I break something?

No. The folders Windows does not let you open are **warnings, not errors**:
the scan goes on and ends with exit code 0. They close the whole output —
after the TOTAL and the biggest-files ranking — one report per row with its
dash. A real example from the scan above:

```
 -------------------------------------------------------------------------------
    41,4 GB                                      178.972    103.007  TOTAL: 107 entries
--- THE 10 BIGGEST FILES ------------------------------------------------------
   350,5 MB  C:\Windows\Installer\46390ac1.msi
   ...
-------------------------------------------------------------------------------
    WARNING  - opening folder C:\Windows\CSC: Accesso negato.
             - opening folder C:\Windows\LiveKernelReports: Accesso negato.
             - opening folder C:\Windows\ServiceState: Accesso negato.
             - opening folder C:\Windows\WUModels: Accesso negato.
             - opening folder C:\Windows\appcompat\UA: Accesso negato.
             - opening folder C:\Windows\Temp: Accesso negato.
             - opening folder C:\Windows\SystemTemp: Accesso negato.
             - opening folder C:\Windows\SysWOW64\config: Accesso negato.
             ... and 46 more reports
```

Worth knowing:

- An inaccessible folder **still appears in the grid**, with whatever it
  could read: look at `-- WUModels` in the table of entry 1 — 0 B and no
  files, because inside it could not be entered. The entry is there, the
  contents are not.
- The list is **cut at 10 entries**: when there are more, the last row
  reports how many are left out (`... and 46 more reports`): what matters
  is the magnitude, not the list.
- The singular is handled in the report too: `... and 1 more report` when
  exactly one is left out — and the same care shapes the plurals of the
  rest of the output ("1 warning" in the totals line with `-d 0`).
- One honest subtlety: in the WARNING block the prefix ("opening folder") is
  ours and follows the language, but the text after the colon ("Accesso
  negato.") is the operating system's answer, passed through untouched: on
  an Italian Windows it stays Italian, on an English one it reads "Access
  is denied." — it is the authentic message, not our translation.

## 3. The first scan is slow, the next ones are fast: why?

Same folder, same machine, seconds apart — the numbers come from the bench
contract of entry 7 (two walks of `C:\Windows`, back to back):

```
> $env:MFD_BENCH = '1'
> mfd-cli C:\Windows 2
ITER 1/2 ms=1398 bytes=44451019623 files=181496 dirs=102997 errors=56
ITER 2/2 ms=1509 bytes=44451019623 files=181496 dirs=102997 errors=56
```

Two walks, two seconds apart, same speed and same totals: the metadata was
already in RAM. The first walk of the day is another story: at the first
read the metadata is in no cache (NTFS takes it from disk, and the
antivirus inspects the files the first time they are touched), so the first
scan of a big tree pays for it all at once. On the machine this FAQ was
born on, a 188.387-file tree measured 21,5 s at the first walk of the day
and 0,4 s on the next one: a factor of ~55 between cold and warm — the
walk did not change, the disk did.

(And yes: the totals can drift by a handful of files between two captures,
if the machine writes meanwhile. A tree is a living thing; entry 9 says
what that means for diffs.)

Practical consequence: when you measure times (or run the benches), the
first run is thrown away — it only serves to warm the machine up. That is
why the scripts in `bench\` warm up every implementation before measuring.

## 4. I just want the total, no table

`-d 0` turns the grid off and leaves the totals line:

```
> mfd-cli -d 0 C:\Windows
mfd-cli: C:\Windows - 181.496 files, 102.997 folders, 41,4 GB in 3849 ms, 56 warnings
```

The numbers are grouped by thousands, the size picks its own unit (B, KB,
MB, GB...) and the milliseconds use the decimal comma below 10 (the declared
format, entry 15 — like the rest of the output). If something could not be
read, the count shows up at the end: `..., 1 warning`.

## 5. How much space do only certain files take? (filters)

`-f` counts only the files whose name matches the pattern; the folders are
all traversed anyway. With a glob:

```
> mfd-cli -f *.dll C:\Windows

       SIZE     %                                  FILES    FOLDERS  NAME
-------------------------------------------------------------------------------
    26,3 GB  100%  ########################       38.668    102.992  C:\Windows (filter: *.dll)
    12,3 GB   46%  ###########.............      26.476     92.330  |-- WinSxS\
     6,8 GB   25%  ######..................        2.161      3.745  |-- assembly\
     4,3 GB   16%  ####....................        4.741      1.651  |-- System32\
     1,1 GB    4%  #.......................        2.707        387  |-- SysWOW64\
     1,1 GB    4%  #.......................          498      1.943  |-- SystemApps\
   645,8 MB    2%  #.......................        1.969      1.929  |-- Microsoft.NET\
    20,5 MB    0%  ........................           30           4  |-- UUS\
    18,1 MB    0%  ........................            6          10  |-- ImmersiveControlPanel\

(20 rows in all; here the first ones — the command prints them all)
-------------------------------------------------------------------------------
    26,3 GB                                       38.668    102.992  TOTAL: 20 entries
    WARNING  - opening folder C:\Windows\WUModels: Accesso negato.
             ...
             ... and 46 more reports

NOTE: 67 folders without a match hidden by the filter
```

Worth noting:

- **The filter changes the meaning of the numbers**, and that is why it
  appears next to the root: `(filter: *.dll)`. The 26,3 GB are the sum of
  the .dll files only — and the TOTAL at the bottom repeats them, because
  it follows the lens like the root does.
- **The FOLDERS column is not filtered** (102.992, about the same as
  without the filter): folders are all traversed, or nothing would be found.
- **Folders without a match are not in the grid**: C:\Windows contains 67
  folders without a single .dll inside, and since 0.7.0 those rows (all
  0 B and 0 files) are not printed — but the filter does not hide in
  silence: the NOTE at the bottom reports how many there are.
- With more expressive patterns comes the regex, in the `REG(...)` form,
  unanchored and case-insensitive: `-f 'REG(\.(zip|7z)$)'` makes the table
  of the archives. Entry 14 is the complete field guide.

## 6. The table is long: can I see just a slice?

`-top N` keeps the first N rows (root included) and says so, because a
truncated count without a notice would be a lie:

```
> mfd-cli -top 5 C:\Windows

       SIZE     %                                  FILES    FOLDERS  NAME
-------------------------------------------------------------------------------
    41,4 GB  100%  ########################      181.507    102.992  C:\Windows
    18,5 GB   44%  ###########.............      124.088     92.330  |-- WinSxS\
     7,1 GB   17%  ####....................       20.437      1.651  |-- System32\
     6,8 GB   16%  ####....................        4.441      3.745  |-- assembly\
     4,0 GB    9%  ##......................          922        100  |-- Installer\
-------------------------------------------------------------------------------
    41,4 GB                                      181.507    102.992  TOTAL: 108 entries

NOTE: first 5 rows of 108; raise -top N to see them all
```

The NOTE sits in the notes block at the bottom, under the grid (the WARNING
rows instead close it, right under the TOTAL): the grid stays purely
numeric, the notices stay aside. The cut does not touch the TOTAL: the data
rows are truncated, the totals are not — the entry count too stays that of
all 108 rows, not of the 5 shown.

Combining `-top` with the sort order makes the classic "top by": `-o A.C
-top 10` for the first 10 names in alphabetical order,

```
> mfd-cli -o A.C -top 10 C:\Windows

       SIZE     %                                  FILES    FOLDERS  NAME
-------------------------------------------------------------------------------
    41,4 GB  100%  ########################      181.507    102.992  C:\Windows
      802 B    0%  ........................            1           0  |-- addins\
   358,6 KB    0%  ........................            6           8  |-- appcompat\
    19,2 MB    0%  ........................           26          11  |-- apppatch\
       0 B    0%  ........................            0           0  |-- AppReadiness\
     6,8 GB   16%  ####....................        4.441      3.745  |-- assembly\
   766,8 KB    0%  ........................            2           0  |-- bcastdvr\
   144,0 KB    0%  ........................            -           -  |-- bfsvc.exe
    70,3 MB    0%  ........................          302         128  |-- Boot\
    66,0 KB    0%  ........................            -           -  |-- bootstat.dat
-------------------------------------------------------------------------------
    41,4 GB                                      181.507    102.992  TOTAL: 108 entries

NOTE: first 10 rows of 108; raise -top N to see them all
```

and `-o F.D -top 10` for the 10 folders with the most files (WinSxS,
System32, servicing, SystemApps...). How deep to go and how many rows to
see are two different questions: `-d N` decides the depth, `-top` decides
the cut. To go all the way down there is `-d a` (or `-d all`): the same
complete scan of `-d a -json`, shown as a tree — and the rows are governed by
`-top`, as always:

```
> mfd-cli -d a -top 6 crates      (run from rust\)

       SIZE     %                                  FILES    FOLDERS  NAME
-------------------------------------------------------------------------------
   553,4 KB  100%  ########################           27           8  crates
   274,5 KB   49%  ###########.............           18           3  |-- mfd-core\
   168,6 KB   30%  ########................            7           0  |   |-- lang\
    26,6 KB    4%  #.......................            -           -  |   |   |-- ja.json
    24,7 KB    4%  #.......................            -           -  |   |   |-- de.json
    24,3 KB    4%  #.......................            -           -  |   |   |-- es.json
-------------------------------------------------------------------------------
   553,4 KB                                            27           8  TOTAL: 36 entries

NOTE: first 6 rows of 36; raise -top N to see them all
```

The same walk through a lens — only the Rust sources, filter declared next
to the root:

```
> mfd-cli -f *.rs -d a -top 8 crates

       SIZE     %                                  FILES    FOLDERS  NAME
-------------------------------------------------------------------------------
   382,8 KB  100%  ########################           17           8  crates (filter: *.rs)
   187,4 KB   48%  ############...........            5           1  |-- mfd-tui\
   187,4 KB   48%  ############...........            5           0  |   `-- src\
    99,2 KB   25%  ######..................            -           -  |       |-- app.rs
    59,8 KB   15%  #####...................            -           -  |       |-- ui.rs
    10,2 KB    2%  ........................            -           -  |       |-- theme.rs
     9,6 KB    2%  ........................            -           -  |       |-- settings.rs
     8,7 KB    2%  ........................            -           -  |       `-- main.rs
-------------------------------------------------------------------------------
   382,8 KB                                            17           8  TOTAL: 25 entries

NOTE: first 8 rows of 25; raise -top N to see them all
```

## 7. How do I use it inside a script?

Three golden rules:

1. **No arguments = the current folder**: `mfd-cli` without a path scans
   the current folder, like `du` (the help stays on `-h`).
2. **Access errors do not fail the scan**: exit code 0 even with 56
   warnings. The nonzero exit code (2) means wrong arguments — and the
   error says which ones:

```
> mfd-cli -lang
mfd-cli: -lang wants a language code (e.g. -lang it; mfd-cli -h for help)

> mfd-cli -big 5 -d 2 C:\Windows
mfd-cli: -big does not combine with -d (it scans the whole tree on its own)
```

(both exit with code 2, on stderr; a clean scan, warnings included, exits 0)

3. **For machines there is MFD_BENCH**: with the environment variable set
   to 1 the output becomes one machine-readable line in place of the grid:

```
> $env:MFD_BENCH = '1'
> mfd-cli "C:\Program Files" 2
ITER 1/2 ms=187 bytes=27123366303 files=142564 dirs=20327 errors=19
ITER 2/2 ms=174 bytes=27123366303 files=142564 dirs=20327 errors=19
```

Without loops the line is `RESULT ms=... bytes=... files=... dirs=...
errors=...`; with more than one loop, one `ITER k/n` line per run — the
tool that tells one-time costs from steady-state throughput, and the one
that caught the cold/warm gap of entry 3. It is the contract the scripts
in `bench\` sit on — see docs/BENCHMARKS.md before using it.

## 8. Rust vs C#: how much does the language really matter?

The comparison is part of the project. Same walker, same folder, same
protocol (warmup of all, interleaved rounds with rotated order, median):

```
> powershell -ExecutionPolicy Bypass -File bench\run-bench.ps1 -Target . -Runs 2

Impl                                         Best_ms Median_ms Runs_ms Files Dirs Errors Bytes
----                                         ------- --------- ------- ----- ---- ------ -----
c#   (find-data: dimensioni gratis, no stat)      21        21 29,21   4462  588  0 1183231510
rust  (find-data via std::fs, no stat)             5         5 5,6     4462  588  0 1183231510
node  (readdir+stat per file, libuv)             174       174 174,187 4462  588  0 1183231510

Totali coerenti fra implementazioni (bytes/files/dirs identici) [OK]
```

What the numbers say:

- **The totals always match** across the three implementations (the final
  `[OK]` is an automatic check): before measuring speed, the bench
  verifies that everybody is counting the same things.
- **The small folder lies about C#**: 21 ms against 5 is the JIT warmup
  of the FDD mode, an artifact of the tiny target — on a real tree the
  race is a tie. The 181.513-file session (C:\Windows, 5 interleaved
  rounds, run-bench + mem-bench, 2026-10-27):

```
> powershell -ExecutionPolicy Bypass -File bench\run-bench.ps1 -Target C:\Windows -Runs 5
> powershell -ExecutionPolicy Bypass -File bench\mem-bench.ps1 -Target C:\Windows -Runs 5

Impl                                         Best_ms Median_ms Runs_ms                  Files   Dirs   Errors Bytes
----                                         ------- --------- -------                  -----   ----   ------ -----
c#   (find-data: dimensioni gratis, no stat)    1370      1378 1375,1370,1380,1378,1413 181513 102998 56     44453468412
c#R2R (ReadyToRun: precompilato, zero JIT)      1380      1464 1494,1520,1380,1464,1405 181513 102998 56     44453468412
rust  (find-data via std::fs, no stat)          1379      1462 1379,1462,1468,1478,1457 181513 102998 56     44453468412
node  (readdir+stat per file, libuv)            8378      8531 8531,8598,8378,8401,8662 181513 102998 56     44453468412

Totali coerenti fra implementazioni (bytes/files/dirs identici) [OK]

Variante                    PeakWS_MB PeakCommit_MB RunsWS_MB                      Files
--------                    --------- ------------- ---------                      -----
c# JIT         (FDD driver)      59,3          41,8 59,3,59,4,54,7,58,7,60,9      181513
c# R2R         (FDD driver)      59,2          41,1 60,3,59,2,60,2,58,55,5        181513
c# R2R exe     (apphost)         56,6          39,1 56,6,57,4,56,6,57,8,56,6      181513
rust                             12,5           9,9 12,5,12,5,12,6,12,4,12,7      181513
node                           251,3         270,2 253,5,232,6,225,8,251,3,251,7 181513

Conteggio file coerente fra varianti [OK]
```

  Speed: C# JIT 1378 ms of median, C# R2R 1464, rust 1462 — neck and
  neck, this session even has C# a hair ahead. Memory: rust 12,5 MB of
  peak working set against ~59 for C# and ~251 for node — one fifth, one
  twentieth. **The language matters less than the runtime around it: the
  verdict of the experiment is memory, not speed.** The whole sessions,
  command included, live in `bench\results\`.
- These are numbers from **one machine, one session**: they serve to
  compare the implementations with each other, not to rank computers.
- (The implementation labels are the bench harness's own words, and they
  stay Italian: the harness is maintainer-facing code, not the product —
  docs/DESIGN-I18N.md §2.4 keeps it that way on purpose.)

The final standings — 1-5 per axis, the first two rows measured, the
last three engineering judgment (the whole analysis, with the startup
bench and the portability axis, is in the READMEs):

| Axis | Rust | C# | Node |
| --- | :---: | :---: | :---: |
| CPU | 🥇 5 | 🥇 5 | 2 |
| RAM | 🥇 5 | 3 | 1 |
| Maturity & ecosystem | 3 | 🥇 5 | 4 |
| Ease of development | 2 | 🥇 5 | 4 |
| Portability | 1 | 4 | 🥇 5 |
| **Total** | **16/25** | **22/25** | **16/25** |
| **The winner, if…** | you are a performance maniac | you want the weighed way | you use it now and then, from one command line |

Nobody wins in absolute terms: Rust the performance crown, Node the
portability crown, C# the best sum — the right compromise.

## 9. I want a before/after report of a cleanup

The question has been in the queue since the project was born, and the
answer is `-json`: the snapshot in a format machines know how to compare.
The picture follows the grid — without `-d` it photographs what you see
(the level-1 grid: the root and its rows), `-d N` opens N levels like the
table does — but a diff wants the whole tree, so the recipe is "two
pictures, one diff" with `-d a`:

```
> mfd-cli -d a -json C:\Users\Public\mfd-captures\demo > before.json
> Remove-Item C:\Users\Public\mfd-captures\demo\ROADMAP.md
> mfd-cli -d a -json C:\Users\Public\mfd-captures\demo > after.json
> Compare-Object (Get-Content before.json) (Get-Content after.json)

InputObject                                                                     SideIndicator
-----------                                                                     -------------
  "totals": {"bytes": 5903, "files": 2, "dirs": 0, "warnings": 0, "elapsed_ms": 1.0}, =>
    "bytes": 5903,                                                              =>
    "files": 2,                                                                 =>
      {"name": "BENCHMARKS.md", "bytes": 2304}                                  =>
  "totals": {"bytes": 7527, "files": 3, "dirs": 0, "warnings": 0, "elapsed_ms": 0.9}, <=
    "bytes": 7527,                                                             <=
    "files": 3,                                                                <=
      {"name": "BENCHMARKS.md", "bytes": 2304},                                <=
      {"name": "ROADMAP.md", "bytes": 1624}                                    <=
```

(the demo "cleanup" is the deletion of one file; `=>` marks the lines only
in the after picture, `<=` only in the before one)

Worth knowing:

- **The only line that always changes is `elapsed_ms`**: scan time is not
  repeatable. Everything else diffs only if the disk changed: the order of
  the children is fixed (descending size), the keys are fixed, non-ASCII
  names come out as \uXXXX escapes (valid JSON in every codepage, like all
  mfd-cli output).
- **The default picture is what you see** — the grid at its default depth
  (level 1: the root, its children as totals rows, its files as leaves):
  small and readable, the JSON of the screen. `-d a` opens the whole tree
  (every folder, every file): that is what a diff needs. With `-d N` the
  picture opens N levels, for lighter diffs — the rows of the grid at N
  levels, exactly.
- `-json` combines with `-f` (the picture of the matching content only)
  and not with `MFD_BENCH`, loops, `-o`, `-top`: deterministic data, no
  cuts.
- **A tree is a living thing**: if something else writes on the disk
  between the two pictures, the diff says so (a capture for this entry once
  caught a 17-file drift mid-capture). Diff a quiet tree — or read the
  drift as a bonus report of what else was happening.

The shape of the picture (the same demo folder, before the cleanup):

```
> mfd-cli -d a -json C:\Users\Public\mfd-captures\demo

{
  "mfd": "0.7.1",
  "root": "C:\\Users\\Public\\mfd-captures\\demo",
  "filter": null,
  "depth": 4294967295,
  "totals": {"bytes": 7527, "files": 3, "dirs": 0, "warnings": 0, "elapsed_ms": 0.9},
  "warnings": [],
  "tree": {
    "bytes": 7527,
    "files": 3,
    "dirs": 0,
    "children": [],
    "leaves": [
      {"name": "ARCHITECTURE.md", "bytes": 3599},
      {"name": "BENCHMARKS.md", "bytes": 2304},
      {"name": "ROADMAP.md", "bytes": 1624}
    ]
  }
}
```

(`"depth": 4294967295` is the full tree: the real limit of the aggregation
depth. Files appear as leaves of their folder's node, folders as children
with their recursive totals.)

And from the TUI you can take a picture without a terminal: the **Shift+P** key
(entry 12) writes the JSON file of the folder you are navigating — with the
framing of the view (open branches, the lens), not the whole world of
`-d a -json`.

## 10. Is there a version to navigate, not just to read?

Since 0.7.0, yes: `mfd-tui <path>` (or just `mfd-tui`, which starts from
the current folder). There is no output to paste — it is an interface, not
a report — but the screenshot campaign left the photographs: the keys and
the semantics below, the looks as they really are.

- **The scan screen**: spinner, files, folders and a progress bar while the
  walker works; ESC cancels with no partial results (all or nothing).
  F5 rescans.

  ![The scan loader](docs/img/tui-scan.png)
- **Two panels**: on the left the folder tree (size, bar, name — the bar
  already shows the %, and the indentation of the branches takes that
  space); on the right the detail of the selection, with the most important
  datum on top — totals, files and folders — and the content below,
  folders and files together. ENTER goes in (from the tree), <- goes up.

  ![mfd-tui: the two panels after the scan](docs/img/tui-start.png)
- **The tree opens where you are looking**: `Shift+X` (or F4) opens or closes the
  branch under the cursor — the children appear indented under the parent,
  in the right place in the lens order — and `Shift+A` opens the whole subtree:
  the interactive equivalent of `-d a` (entry 6), one branch at a time.
  The tree is a view: ENTER navigates inside, and the open branches are
  still there when you go back up. **TAB** brings the cursor, **in reading
  mode**, to the detail — tree open or closed — the Total Commander panel
  switch: arrows and Home/End scroll the panel's list, ENTER does not go in
  (who navigates stays in the tree, so there are not two ways in and no
  doubt about which one commands) — and TAB again goes back to the tree
  (the lit border says where the cursor is).

  ![mfd-tui: branches opened one by one with Shift+X](docs/img/tui-expand.png)
- **One lens only**: `Shift+S` (sort, the vocabulary of `-o`) and `Shift+F` (filter,
  the rules of `-f`) hold for list and detail together. On the left you
  hunt, on the right you read what the selection is made of, through the
  same lens. `Shift+F` opens a big window with the instructions and the examples
  of the two syntaxes (glob and `REG(...)`): a filter that does not compile
  does not close the window — the error stays in sight until you fix it.

  ![The filter prompt (Shift+F / F7)](docs/img/tui-filter.png)
- **The ranking at the bottom**: the biggest files of the branch under the
  cursor, always visible — it follows the selection as you move. Select
  the root row `.` (the top one, the current folder) and it is the whole
  folder; select a child and it is that branch. It is `-big` (entry 11)
  with a moving root: ranking, % and bar over the branch total. **Digits
  rewrite the top on the fly** (2, 25, 100...), `F9` removes the panel
  when you need the whole screen. The ranking goes through the lens too:
  with a filter on, only matching files count. `TAB` brings a reading
  cursor there: the arrows walk it, and the status line says the full
  path of the pointed row.
- **Shift+G** toggles the bar, **Shift+W** opens the warnings window (which declares
  by itself what they are: the entries the scan could not read, whose
  content is not in the totals), **Shift+T** (or F3) chooses the theme, **F1**
  the help, **Shift+Q** quits. Four themes: **Night** (Total Commander blue),
  **Debian** (installer blue, gray windows, red selection), **Console**
  (the default: white on black, yellow accents, light-green selector) and
  **CLI** (black: your terminal's colors). 16 colors. The **mouse** wears
  keys: the **wheel** scrolls like the arrows (one detent = one row,
  anywhere; tilted left/right it is <- and ->: up to the parent and in,
  from the tree), the **click** selects the pointed row (or brings the
  reading cursor to the detail), the **double click** enters the folder
  like ENTER, the **right button** opens or closes the branch like Shift+X. The
  side buttons (Back/Forward, the browser ones) do not cross the terminal:
  they cannot be used. To copy the terminal's text you hold Shift while
  dragging (the mouse capture belongs to the TUI).

  ![The theme menu of the TUI (Shift+T / F3)](docs/img/tui-theme-menu.png)

  ![The Night theme](docs/img/tui-theme-night.png)

  ![mfd-tui: the F1 help](docs/img/tui-help.png)
- **F2 saves the settings**: theme, lens (sort and filter) and bar end up
  in the user profile (%APPDATA%\mfd-tui\settings.txt on Windows,
  ~/.config/mfd-tui/settings.txt elsewhere) and the next start picks them
  up again; F2 again updates them, deleting the file brings back the
  factory defaults. Without F2 everything stays per-session. **F5**
  rescans.

  ![mfd-tui: F2 saves the settings](docs/img/tui-save.png)

The engine is the same as the CLI: sizes, filters and warnings have the
same semantics — and the totals line on top of the detail is the same TOTAL
that closes the CLI grid. The TUI measures, it does not mutate: the report
of a cleanup stays `-d a -json` + diff business (entry 9).

The approved design (wireframes, decisions, keys) is in docs/DESIGN-TUI.md.
To build it: `cargo build --release` in `rust\` (the workspace takes care
of everything), the executable is `rust\target\release\mfd-tui.exe`. Rust
missing? The README's Install section starts with the one-liner:
`winget install Rustlang.Rustup`.

## 11. I want the biggest files, not the map

The map says where the space is; to act you need the address. `-big N` is
the direct answer: the N biggest files of the tree with full path, in fixed
descending size order — the row a batch, a script or an AI can act on right
away, without post-processing `-json`:

```
> mfd-cli -big 10 C:\Windows
```

![The ten biggest files: mfd-cli -big 10](docs/img/cli-big.png)

On a real terminal the list wears the same duf box as the grid — the
photograph above; redirected, it is the flat ASCII table of the contract,
like the grid (entry 1).

The shape: the first column is the SIZE of the grid, next to it the full
path. No TOTAL — the list is the answer, and the files left out by the cut
are not lost totals. The order is fixed (descending size, on ties the
smaller path wins): two scans of the same folder give the same rows in the
same order, the same determinism as `-json`. Like `-d a -json`, it scans the
complete tree: the cost is that, the answer is N rows.

With a filter you ask the same question through a lens: `-big 3 -f *.log`
are the biggest .log files, and the header declares it (`PATH (filter:
*.log)`), like the root of the grid declares the filter:

```
> mfd-cli -big 3 -f *.log C:\Users\Public\mfd-captures\demo2

       SIZE  PATH (filter: *.log)
-------------------------------------------------------------------------------
      302 B  C:\Users\Public\mfd-captures\demo2\report.log
      202 B  C:\Users\Public\mfd-captures\demo2\ANALYSIS.LOG
```

(two rows, not three: in that folder only two files match `*.log` — the
third one asked for was not there to be found)

With `-json` the answer changes shape: the JSON picture of the report —
the ranking and the full paths in a format machines read, the totals of
the branch in the header. Not combinable with `-d`, `-o`, `-top`, `MFD_BENCH`
and loops: they are options about shape, and `-big` already has its own
(entry 7 shows what the refusal looks like).

In the TUI the same answer sits at the bottom of the screen: the
**ranking panel** (F9) — the same rows as `-big` (size and full path)
plus the grid's % and bar, the top changed by digits, and it too goes
through the lens of the active filter (entry 10).

## 12. Can the TUI take the JSON picture, for reporting?

Yes: it is the **print**, key `Shift+P` (or `F11`). While you navigate, Shift+P writes
to file the JSON picture of what you are looking at — **both panes, left
and right** — the same format as `-json` (entry 9), because the writer
is the same serializer, promoted from the CLI into the shared engine
(mfd-core): the picture is a contract, not a CLI output.

![mfd-tui: the print from the main screen](docs/img/tui-print-home.png)

- **What it photographs**: the current folder, root included — it follows
  the drill-down: go into a subfolder and
  Shift+P photographs that, not the whole scan. And it photographs the
  screen **as you see it, both panes**: the `tree` section is the left
  one — the branches you opened with `Shift+X` / `Shift+A` open in the picture too,
  the closed ones stay rows with their recursive totals (`"children":
  null` is not an empty folder: it is a branch the picture left as it
  was on screen) — while the `detail` section is the right one: the
  selected row (`selected`) with the children the preview shows, which
  the tree may still keep closed. Two panes, one picture: the print
  says what you have on both sides. The lens holds too, with the same
  rules as the list: non-matching leaves do not appear, branches
  without a match neither, the totals are the filtered ones — and the
  filter is declared in the header (`"filter": "*.dll"` instead of
  null). The rows keep the picture's canonical order (size, T.D), not
  the one chosen with `Shift+S` at that moment: two pictures of the same view
  diff only if the filesystem changed. The `root` field always says
  which folder was photographed; the `totals` are the recursive ones of
  the branch (through the lens) and the `warnings` only those of the
  branch.
- **Where it lands**: the folder where you launched `mfd-tui`, not the
  scanned one (mfd measures, it does not mutate) — `mfd-stampa-<name>.json`.
  If the name
  already exists, the next print becomes `mfd-stampa-<name>-2.json`, then
  -3: no silent overwrites, two pictures are worth a diff. The flash on
  the status line says the full path.
- **How to use it**: like the two pictures of entry 9 — print, clean,
  print again, `Compare-Object` on the two files. Without retyping the CLI
  with the redirect: you are already where you need to be.

Both pictures, real, on the seven-file demo folder — all branches closed
on the left, the selection on `alpha`: the `detail` section carries what
the `tree` section keeps closed (`mfd-stampa-demo-tui.json`):

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
      {
        "name": "alpha",
        "bytes": 3000,
        "files": 2,
        "dirs": 0,
        "children": null,
        "leaves": null
      },
      {
        "name": "gamma",
        "bytes": 310,
        "files": 2,
        "dirs": 1,
        "children": null,
        "leaves": null
      },
      {
        "name": "beta",
        "bytes": 120,
        "files": 2,
        "dirs": 1,
        "children": null,
        "leaves": null
      }
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

The report picture — the ranking and the full paths — is the CLI's job:
`mfd-cli -report -json` (entry 11 shows the shape).

The `mfd` field of the header is the version of the frontend that took
the picture (`0.7.x` the CLI with `--json`, `0.2.x` the TUI with Shift+P):
knowing who takes the picture is part of the data, and the format is the
same.

## 13. What languages do the CLI and the TUI speak?

English out of the factory, and everything else is files. The UI speaks the
language chosen from the `Shift+L` / `F12` menu — or from `mfd-tui --lang it`, or
the `MFD_LANG` variable, or the `lingua=` line that F2 writes in the
profile, in this order. The CLI uses the same chain and the same registry
with `mfd-cli -lang it` (it also accepts `-lang=it`, `-lang:it` and the
doubled `--`): `-h`, errors, grid headers, WARNING/NOTE and the loader
speak the active language; RESULT/ITER, `-json` and the numbers stay the
contract of always. A real session in Italian:

```
> mfd-cli -lang it -top 3 C:\Windows

 DIMENSIONE     %                                   FILE   CARTELLE  NOME
-------------------------------------------------------------------------------
    41,4 GB  100%  ########################      181.496    102.997  C:\Windows
    18,5 GB   44%  ###########.............      124.088     92.330  |-- WinSxS\
     7,1 GB   17%  ####....................       20.437      1.651  |-- System32\
-------------------------------------------------------------------------------
    41,4 GB                                      181.496    102.997  TOTALE: 108 voci
    WARNING  - apertura cartella C:\Windows\appcompat\UA: Accesso negato.
             - apertura cartella C:\Windows\WUModels: Accesso negato.
             - apertura cartella C:\Windows\SystemTemp: Accesso negato.
             - apertura cartella C:\Windows\Temp: Accesso negato.
             - apertura cartella C:\Windows\System32\appmgmt\S-1-5-18: Accesso negato.
             - apertura cartella C:\Windows\System32\appmgmt\MACHINE: Accesso negato.
             - apertura cartella C:\Windows\SysWOW64\config: Accesso negato.
             - apertura cartella C:\Windows\SysWOW64\Configuration: Accesso negato.
             - apertura cartella C:\Windows\SysWOW64\Com\dmp: Accesso negato.
             - apertura cartella C:\Windows\System32\Com\dmp: Accesso negato.
             ... e altre 46 segnalazioni

NOTE: prime 3 righe di 108; alzare -top N per vederle tutte
```

![La griglia parla la lingua della sessione: -lang it](docs/img/cli-lang.png)

(the totals line too: `mfd-cli -lang it -d 0 <folder>` answers `mfd-cli:
C:\Windows - 181.496 file, 102.997 cartelle, 41,4 GB in 1441 ms, 56
warning`; and `-h` opens with
`mfd-cli 0.7.1 - conteggio ricorsivo di file, cartelle e byte`)

One case that fools the eye: in the WARNING block the prefix ("opening
folder", "apertura cartella") is ours and follows the language, but the
text after the colon ("Accesso negato.") is the operating system's answer,
passed through untouched: on an Italian Windows it stays Italian, on an
English one it is "Access is denied." — the authentic message, not our
translation (entry 2).

English lives inside the binary (the guaranteed fallback); the other
languages are JSON files in `%APPDATA%\mfd\lang\` that the menu discovers
on its own at the next start: anyone can drop one in, zero code touched.

![The language menu (Shift+L / F12)](docs/img/tui-lang.png)
In the repo we start with seven — en, it, es, pt-BR, de, ja, zh-CN — and
five are AI-translated: the menu marks them `[AI]` with the notice at the
bottom, and the file carries the disclaimer, because an unverified
translation must be presented for what it is. When a person checks one,
the mark comes off and the language leaves the menu clean. The English
vocabulary is the reference: an incomplete language shows English only
where it lacks the translation, and key parity is a test (PRs with
half-translated languages do not pass). An unknown code does not stop the
run:

```
> mfd-cli -lang xx -d 0 C:\Windows
mfd-cli: language 'xx' not found (available: en, de, es, it, ja, pt-BR, zh-CN)
mfd-cli: C:\Windows - 181.496 files, 102.997 folders, 41,4 GB in 1473 ms, 56 warnings
```

(a warning on stderr, the run starts in English, exit code 0 — like the
TUI, never an error)

`-json` and the `Shift+P` prints stay the machine contract of always: English
keys, byte for byte.

## 14. Glob or REG? A field guide to filters

`-f` speaks two syntaxes, and they are not interchangeable: the glob thinks
in whole names, the regex thinks in substrings. A scratch folder with
seven files, chosen to tell them apart: report.log (302 B), ANALYSIS.LOG
(202 B), logger.txt (402 B), catalog.txt (152 B), backup.logs (252 B),
archive.zip (502 B), archive.7z (352 B).

**The glob matches the whole name** (and does not care about case, like
everything else on Windows): `*.log` means "any name ending in .log":

```
> mfd-cli -f *.log C:\Users\Public\mfd-captures\demo2

       SIZE     %                                  FILES    FOLDERS  NAME
-------------------------------------------------------------------------------
      504 B  100%  ########################            2          0  C:\Users\Public\mfd-captures\demo2 (filter: *.log)
      302 B   59%  ##############..........            -          -  |-- report.log
      202 B   40%  ##########..............            -          -  `-- ANALYSIS.LOG
-------------------------------------------------------------------------------
      504 B                                            2          0  TOTAL: 3 entries
```

report.log and ANALYSIS.LOG match — upper case included — while
backup.logs does not (its extension is .logs, not .log). The % is computed
on what the lens lets through: 302 is 59% of 504.

**The regex, in the `REG(...)` form, is not anchored and matches anywhere
in the name** — the classic trap:

```
> mfd-cli -f 'REG(log)' C:\Users\Public\mfd-captures\demo2

       SIZE     %                                  FILES    FOLDERS  NAME
-------------------------------------------------------------------------------
     1,3 KB  100%  ########################            5          0  C:\Users\Public\mfd-captures\demo2 (filter: REG(log))
      402 B   30%  #######.................            -          -  |-- logger.txt
      302 B   23%  ######..................            -          -  |-- report.log
      252 B   19%  #####...................            -          -  |-- backup.logs
      202 B   15%  ####....................            -          -  |-- ANALYSIS.LOG
      152 B   11%  ###.....................            -          -  `-- catalog.txt
-------------------------------------------------------------------------------
     1,3 KB                                            5          0  TOTAL: 6 entries
```

Five files: logger.txt contains "log" in the middle of the name, catalog
contains it too ("cata-log"), backup.logs matches with its s. `REG(log)`
answers "whose name contains log", not "which ones are the logs". To say
"which ones are the logs" you anchor the end: in `REG(\.log$)` the escaped
dot is a real dot and the `$` nails it to the end of the name — and it
gives back the same two files as the glob:

```
> mfd-cli -f 'REG(\.log$)' C:\Users\Public\mfd-captures\demo2

       SIZE     %                                  FILES    FOLDERS  NAME
-------------------------------------------------------------------------------
      504 B  100%  ########################            2          0  C:\Users\Public\mfd-captures\demo2 (filter: REG(\.log$))
      302 B   59%  ##############..........            -          -  |-- report.log
      202 B   40%  ##########..............            -          -  `-- ANALYSIS.LOG
-------------------------------------------------------------------------------
      504 B                                            2          0  TOTAL: 3 entries
```

The regex pays back its complexity with the alternation — one pattern,
many extensions:

```
> mfd-cli -f 'REG(\.(zip|7z)$)' C:\Users\Public\mfd-captures\demo2

       SIZE     %                                  FILES    FOLDERS  NAME
-------------------------------------------------------------------------------
      854 B  100%  ########################            2          0  C:\Users\Public\mfd-captures\demo2 (filter: REG(\.(zip|7z)$))
      502 B   58%  ##############..........            -          -  |-- archive.zip
      352 B   41%  ##########..............            -          -  `-- archive.7z
-------------------------------------------------------------------------------
      854 B                                            2          0  TOTAL: 3 entries
```

The rule of thumb: a glob for the simple cases (`*.iso`, `*.vhdx`,
`Cargo.*`), a regex when the pattern is a sentence ("the archives",
"whatever ends in a backup extension"). And remember the lens is a lens:
folders are traversed all the same, no-match folders are hidden with a
NOTE (entry 5), and the totals follow the filter.

## 15. Why do the numbers use Italian separators?

Because they are a contract, and contracts change loudly, not quietly.
`41,4 GB`, `181.507 files`: dot for thousands, comma for decimals — the
format the project was born with, kept identical in every language. The
reason is in the design (docs/DESIGN-I18N.md, §5): the redirected grid
must stay byte-for-byte the format it has always had, and changing a
separator changes the bytes — of the logs, of the scripts, of every diff
already saved. Localizing the words is a dictionary lookup; localizing the
numbers is a signature on a contract.

Machines are not affected: `-json`, RESULT/ITER and the raw bytes of the
totals speak plain numbers (`"bytes": 152128347006`, no separators). If one
day the numbers follow the language too, it will happen the way contract
changes happen: declared in the CHANGELOG, not discovered in a diff.

## 16. A full disk hunt, end to end

The single answers are one option each; a real hunt chains them. The
scene: the system folder after a round of updates feels heavy, and the
question is the oldest one — where did the space go? First, the size of
the problem:

```
> mfd-cli -d 0 C:\Windows
mfd-cli: C:\Windows - 181.496 files, 102.997 folders, 41,4 GB in 3849 ms, 56 warnings
```

Then the map — one level is enough to see the shape (entry 1 has the same
grid in text; here it is as it dresses on a real terminal):

```
> mfd-cli -top 10 C:\Windows
```

![The grid on a real terminal: mfd-cli -top 10](docs/img/cli-grid.png)

44% of the folder is WinSxS\ — the component store — with System32 and
assembly behind it. The map says where; the addresses come from `-big`:

```
> mfd-cli -big 10 C:\Windows

       SIZE  PATH
-------------------------------------------------------------------------------
   348,5 MB  C:\Windows\Installer\3b7172f7.msi
   327,7 MB  C:\Windows\System32\Microsoft-Edge-WebView\msedge.dll
   327,7 MB  C:\Windows\WinSxS\amd64_microsoft-edge-webview_31bf3856ad364e35_10.0.26100.9711_none_2e9083253adbaceb\msedge.dll
   325,9 MB  C:\Windows\WinSxS\amd64_microsoft-edge-webview_31bf3856ad364e35_10.0.26100.9444_none_2eb3af6f3ac0f200\msedge.dll
   251,4 MB  C:\Windows\WinSxS\amd64_microsoft-edge-webview_31bf3856ad364e35_10.0.26100.1_none_8fe1c356f34a19ca\msedge.dll
   222,8 MB  C:\Windows\System32\DriverStore\FileRepository\nvmd.inf_amd64_cc1a5d7d67a7b51d\nvoptix.dll
   219,6 MB  C:\Windows\System32\MRT.exe
   163,7 MB  C:\Windows\Installer\114fc849.msi
   160,4 MB  C:\Windows\Installer\501d750a.msi
   143,8 MB  C:\Windows\Installer\14d7f8ae.msi
    WARNING  - opening folder C:\Windows\WUModels: Accesso negato.
             - opening folder C:\Windows\Temp: Accesso negato.
             - opening folder C:\Windows\ServiceProfiles\SQLTELEMETRY$SQLEXPRESS01: Accesso negato.
             - opening folder C:\Windows\ServiceProfiles\SQLTELEMETRY$SQLEXPRESS: Accesso negato.
             - opening folder C:\Windows\ServiceProfiles\SQLTELEMETRY: Accesso negato.
             - opening folder C:\Windows\SystemTemp: Accesso negato.
             - opening folder C:\Windows\security\cap: Accesso negato.
             - opening folder C:\Windows\ServiceProfiles\NetworkService: Accesso negato.
             - opening folder C:\Windows\ServiceProfiles\MSSQLSERVER: Accesso negato.
             - opening folder C:\Windows\security\audit: Accesso negato.
             ... and 46 more reports
```

The plot thickens: one installer package is worth 348,5 MB, and msedge.dll
shows up four times — three versions that WinSxS, the component store,
keeps on disk. The tree view completes the picture for the folder you
decide to keep:

```
> mfd-cli -d a -top 10 C:\Windows\Installer

       SIZE     %                                  FILES    FOLDERS  NAME
-------------------------------------------------------------------------------
     4,0 GB  100%  ########################          922        100  C:\Windows\Installer
   348,5 MB    8%  ##......................            -          -  |-- 3b7172f7.msi
   163,7 MB    3%  #.......................            -          -  |-- 114fc849.msi
   160,4 MB    3%  #.......................            -          -  |-- 501d750a.msi
   143,8 MB    3%  #.......................            -          -  |-- 14d7f8ae.msi
   136,2 MB    3%  #.......................            -          -  |-- 57cc8f0.msi
   136,0 MB    3%  #.......................            -          -  |-- 8f41bbe9.msi
   119,8 MB    2%  #.......................            -          -  |-- 114824c1.msi
   115,0 MB    2%  #.......................            -          -  |-- 8f41bb29.msi
   112,3 MB    2%  #.......................            -          -  |-- 41478d7a.msi
-------------------------------------------------------------------------------
     4,0 GB                                          922        100  TOTAL: 1.023 entries

NOTE: first 10 rows of 1.023; raise -top N to see them all
```

1.023 entries, 10 shown: the classic shape of a system folder — a few
huge packages scattered over a thousand patch files. And the lens answers
the accountant's question — how much of this folder is code?:

```
> mfd-cli -f 'REG(\.(exe|dll)$)' -top 6 C:\Windows

       SIZE     %                                  FILES    FOLDERS  NAME
-------------------------------------------------------------------------------
    28,4 GB  100%  ########################       44.090    102.997  C:\Windows (filter: REG(\.(exe|dll)$))
    13,5 GB   47%  ###########.............       30.568     92.330  |-- WinSxS\
     6,8 GB   23%  ######..................        2.243      3.745  |-- assembly\
     5,0 GB   17%  ####....................        5.507      1.651  |-- System32\
     1,2 GB    4%  #.......................        3.040        387  |-- SysWOW64\
     1,1 GB    3%  #.......................          556      1.943  |-- SystemApps\
-------------------------------------------------------------------------------
    28,4 GB                                       44.090    102.997  TOTAL: 30 entries
    WARNING  - opening folder C:\Windows\appcompat\Backup: Accesso negato.
             - opening folder C:\Windows\ModemLogs: Accesso negato.
             - opening folder C:\Windows\Prefetch: Accesso negato.
             - opening folder C:\Windows\ServiceState: Accesso negato.
             - opening folder C:\Windows\Provisioning\Autopilot: Accesso negato.
             - opening folder C:\Windows\ServiceProfiles\SQLTELEMETRY$SQLEXPRESS: Accesso negato.
             - opening folder C:\Windows\ServiceProfiles\SQLTELEMETRY$SQLEXPRESS01: Accesso negato.
             - opening folder C:\Windows\WUModels: Accesso negato.
             - opening folder C:\Windows\Temp: Accesso negato.
             - opening folder C:\Windows\SystemTemp: Accesso negato.
             ... and 46 more reports

NOTE: first 6 rows of 30; raise -top N to see them all
```

28,4 GB of executables and libraries out of 41,4: through the lens, what
sits in C:\Windows turns out to be mostly Windows. What is left is the
proof: photograph with `-d a -json`, clean, photograph again, diff —
entry 9 is the complete recipe, and the TUI takes the same pictures with
`Shift+P` without leaving the place you are in (entry 12).

## The queue (questions still without an answer: the feature queue)

- "Can I reopen a saved picture in the TUI?" — opening the saved snapshots
  (the plan has it, with the GUI).
