# Changelog

Format inspired by "Keep a Changelog"; the version is mfd-cli's.

## [Unreleased]

### Changed
- **The picture, in one box**: `mfd-cli` now answers with the whole
  picture — the folder grid above and the 10 biggest files below, one
  box, two sections. The root row is gone (its 100% said nothing the
  TOTAL doesn't already say): the scanned path lives in the title band
  on top (the ncdu convention), and the files section declares itself
  with a yellow header — the SIZE column re-labeled, the section title
  centered, uppercase. `-grid` keeps just the grid (the lean contract
  of pipes and logs), `-report N` just the ranking (`-big` stays as its
  historical alias, N now optional and 10 by default); `-grid` and
  `-report` together refuse with a clear error. The warnings close the
  box, after the data: notes, contextual to both parts, secondary to
  both. The classification reads the whole tree in the same single walk
  the grid reads, and `-top` no longer counts the root among its rows.
  New keys in all seven languages (`cli.err_view`, `cli.top_file`,
  `cli.top_files`) — the parity gate is green.

### Fixed
- **The three Linux-failing tests now run everywhere**: they compared paths
  written with the Windows backslash (`m\eq.bin`) — on Linux the backslash
  is a plain filename character, so the fixture built the wrong tree and
  the asserts checked the wrong tail. Paths are now built with the platform
  separator (a `rel()` helper for the asserts, plain forward slashes for
  the fixtures, which both platforms accept), and the CI runs
  `--no-fail-fast` so every test binary reports even when an earlier one
  fails.
- **The duf box no longer shifts with short file counts**: the FILES and
  FOLDERS columns of the terminal table had a hardcoded minimum width (4
  and 8, the Italian headers) — with few files the English header FILES
  (5 chars) overflowed its cell by one and everything after it shifted
  right; Spanish ARCHIVOS shifted by four, German DATEIEN by three. The
  minimum width now follows the active language's own headers, so no
  translation can ever break out of its column (regression test included,
  English catalog with short counts).

### Added
- **Issue templates**: three doors for the public — bug (command,
  redirected output, version), question (FAQ-driven: the real ones become
  manual entries), feature request (the command you wish you could write)
  — with the manual linked as the first stop. `CONTRIBUTING.md` and the
  READMEs point to them.
- **Linux in the release**: the workflow now builds on `ubuntu-latest`
  too, packages a `mantofreedisk-<ver>-linux-x64.tar.gz` (same content
  as the Windows ZIP: the two binaries, the seven languages in lang/,
  an install.sh, the license and a README) and uploads it to the same
  GitHub Release. Installation mirrors the Windows one: extract and run
  (portable — lang/ next to the binaries works), or `install.sh` to
  publish as user commands (~/.local/bin) and install the languages
  (~/.config/mfd/lang). SHA256 alongside.
- **Portable mode**: `mfd-cli` and `mfd-tui` now search for a `lang/`
  folder next to the executable before falling back to the profile
  (`%APPDATA%\mfd\lang`). The ZIP carries its own languages: extract and
  run, even on machines where writing to the profile is not an option
  (locked-down terminals, restricted accounts). The profile remains the
  installed mode and the override.

### Changed
- **The CI blocks on all three systems now**: Linux and macOS tests used
  to run as information only (`continue-on-error`) — with the paths fixed
  they run green, so every platform gates every push and every PR.

## [0.8.0] - 2026-10-27

### Added
- **The experiment, with the real numbers**: the READMEs' front table now shows speed
  AND memory from a real 181.513-file session (run-bench + mem-bench on
  C:\Windows, the outputs pasted whole in `bench\results\`): the walk is
  a tie between Rust and C# — the old 21-vs-5 ms was the JIT warmup on a
  tiny folder — and the verdict is memory: 12,5 MB of peak working set
  against ~59 for the C# walker and ~251 for Node. The experiment now
  reads on three axes — performance (CPU tie, RAM one fifth: the garbage
  collector against ownership), robustness/maturity/development
  difficulty, portability — two games: .NET cross-publishes any target
  from one machine, Rust ships the smaller artifact (a single 2 MB
  binary, no runtime) — and the third bench, startup, was run and pasted
  whole too:
  ~10 ms for the native exe against ~60 for the CLR and V8. The C# twin
  is declared for what it is today — a proof of concept, the walker
  without the CLI and the TUI — with its promotion as the roadmap's next
  chapter. The verdict closes on a final scorecard, 1-5 per axis: nobody
  wins in absolute terms — Rust the performance crown, Node the
  portability crown, C# the best sum, the right compromise. The FAQ
  (entry 8), ARCHITECTURE and ROADMAP tell the same story, and the C#
  walker is declared a proof of concept everywhere. The road from here
  is explicit in the READMEs: Rust ships first, the C# twin grows
  through the CLI and blooms in the GUI, Node stays the frozen
  patriarch — and the roads not taken stay open to the community.
- **`mfd-cli` — `-big` wears the duf dress too**: the ranking was the last
  answer that still looked flat on a real terminal; now it sits in the
  same rounded box as the grid — yellow borders and headers, blue full
  paths, the WARNING section closing the box with the one-cross footer —
  while the redirected output stays the flat ASCII contract of always,
  byte for byte (logs, scripts and the FAQ's pasted outputs unchanged).
  `render_big_table` in theme.rs, tested like its sibling `render_table`.
  mfd-cli 0.8.0.
- **The docs tell no private stories anymore**: every example in the FAQ
  now runs on public ground — `C:\Windows`, `C:\Program Files`, a demo
  folder under `C:\Users\Public` — the commands were re-run for real and
  the outputs pasted whole from the new runs; where a look matters more
  than a list, the photographs carry it (the duf dress in entry 1, the
  TUI manual in entry 10, the hunt of entry 16). The two READMEs and
  BENCHMARKS follow the same rule.
- **The READMEs dress like a real GitHub home**: the mascot portrait opens
  the page, the two frontends stand side by side — each with its screenshot
  and its manual (the FAQ, one entry per tool) — and the badges say what
  the family is made of. Same content underneath, new order: the
  question first, the two answers after, the galleries inside their own
  sections.
- **The READMEs open their eyes — the real screenshots**: what the story
  told so far with text outputs now shows itself: `docs/img/` collects the
  real photographs of the family — the ten biggest files (`-big 10`), the
  grid in its duf dress on a real terminal, and the TUI: the two panels
  after the scan, the branch-by-branch expand of Shift+X, the tree with
  the reading cursor of TAB, the report window (Shift+R / F9), the F1
  help, the F2 save, the print (Shift+P) from the main screen and from
  the report window. Then the whole tree (-d a), the Italian grid
  (-lang it), the filter lens, the themes, the filter prompt, the
  language menu, the scan loader. One set of pictures, English captions
  for README.md and Italian ones for README.it.md.
- **READMEs — the ten biggest files open the page, and Rust gets its
  install note**: "where are the 10 biggest files on your PC?" is the
  question this tool hears most, so both READMEs answer it first — one
  command (`mfd-cli -big 10 C:\`), its real output (a true `C:\Windows`
  scan: the system's own heavyweights on top, no private paths), and only
  after that the pitch, the grid and the rest of the story. The Install
  section opens with the
  Rust prerequisite: `winget install Rustlang.Rustup` (or
  `rustup-init.exe` from rustup.rs), a new terminal, `cargo --version`;
  the FAQ's build note links the same.
- **`mfd-cli` — `-json` photographs what you see, and the report too**:
  the field test asked for three things, and they are in. (1) The flag
  reads `-json` like every other flag of the family — the doubled
  `--json` keeps working, but the help and the errors say the
  single-dash way. (2) The default picture is the grid's: without `-d`
  it photographs what you see (level 1 — the root, its children as
  totals rows, its files as leaves: small and readable, not the huge
  dump of the whole disk), `-d N` opens N levels exactly like the table
  does; the whole world of the before/after diff stays one `-d a` away,
  byte-for-byte the contract of always. (3) `-big N -json` is the JSON
  picture of the report (`json::report`: ranking, full paths, the
  totals of the branch in the header) — the same one the TUI
  photographs from its report window; before, the two flags refused
  each other. And `-d 0 -json` does not panic anymore: there is no tree
  to photograph, the picture says `"tree": null` with the totals in the
  header. mfd-core grows the `depth` parameter of `json::subtree` (the
  aggregation depth, declared in the header of the picture); the help
  of the seven languages, the two READMEs and FAQ 9/11 say the new
  behavior. `mfd-cli` 0.7.1.
- **`mfd-tui` — a bare letter does not command anymore: the command
  letters wear SHIFT**: the field test paid for it — a stray `q` next
  to `x` closed the app, scan paid and gone. Since now the letter
  shortcuts are pressed with SHIFT (`Shift+Q` quits, `Shift+P` prints,
  `Shift+X` opens the branch, and so on), while the function keys stay
  direct — they are made for that. The miss is not silent: a bare
  command letter flashes the shifted way to say it (`Shift+Q`), in the
  main screen and inside the windows — the filter prompt keeps typing
  letters, there they are text. F1, the `-h` and the footers of the
  seven languages say the rule. Still `mfd-tui` 0.2.1, unpublished: it
  ships together with the print.
- **`mfd-tui` — the print photographs both panes, and the report
  window too**: `Shift+P` / F11 from the main screen now saves what you are
  looking at, left AND right. The `tree` section stays the left pane
  (the branches opened with Shift+X / Shift+A, the closed ones as totals
  rows, the lens declared) and a new `detail` section adds the right
  one: the
  selected row and the children the preview shows — which the tree may
  still keep closed; the picture says what you have on both sides. And
  with the report window open (`Shift+R` / F9), Shift+P photographs that: the top
  asked and the big files with their ranking and absolute paths — the
  framing follows the window, and the file family says it
  (`mfd-report-<name>.json` beside `mfd-stampa-<name>.json`, both
  numbered on collision: no silent overwrites). The two framings are
  new photos of the shared contract — mfd-core grows `json::view` /
  `json::report` beside `json::subtree`: same header (root, filter,
  totals and warnings of the branch), same lens everywhere, rows in
  the picture's canonical order so that two pictures diff only for the
  filesystem. The report footer, the F1 help and the `-h` of the seven
  languages say the new behavior; FAQ 12 shows both pictures, real.
  `mfd-tui` 0.2.1.
- **The documentation speaks English — the FAQ becomes a manual**:
  with publication approaching, the docs adopt one language for
  everybody: every document (FAQ, CONTRIBUTING, ARCHITECTURE,
  BENCHMARKS, ROADMAP, DESIGN-I18N, DESIGN-TUI and the CHANGELOG
  itself) is rewritten in English; `README.it.md` stays as the single
  Italian counterpart, and no other doc translation enters the repo
  (the decision is recorded in docs/DESIGN-I18N.md §2.4). The FAQ is
  rebuilt on mfd-cli 0.7.1 with every example re-run — English grid,
  today's real outputs — and grows by three entries: the glob-vs-REG
  field guide with the substring trap (14), why the numbers keep the
  Italian format (15) and the full disk hunt, end to end (16). The
  quick-reference tables of the two READMEs speak the new outputs too.
- **`mfd-cli` — the languages reach the CLI (phase 2 of
  docs/DESIGN-I18N.md)**: `-lang` — in all the shapes that get typed
  naturally, `-lang it`, `-lang=it`, `-lang:it` and the doubled `--` —
  beats `MFD_LANG` which beats the built-in English: the same chain,
  the same registry and the same folder as the TUI
  (`%APPDATA%\mfd\lang\`). What speaks the active language: `-h`
  (tagline and help), the 18 parsing and combination errors, the grid
  headers (SIZE/FILES/FOLDERS/NAME), the TOTAL with the entry count,
  the WARNING/NOTE rows, the filter next to the root and the loader
  (braille and history: the usual shape, the words of the language).
  What never speaks: RESULT/ITER, `-json` and the numbers (Italian
  separators, §5 of the design keeps them declared). A missing value
  is an error (`-lang` alone exits 2), a nonexistent code warns on
  stderr and the run starts in English — like the TUI, never an error.
  The 46 `cli.*` keys live in the same seven files of `lang/` (the
  parity gate nails them all); the nine `cli.*` keys the TUI used for
  its own banner are renamed `tui.*` — the prefix says who is
  speaking. `install.ps1` publishes the registry too: after the two
  exes it copies the seven languages of `lang/` into
  `%APPDATA%\mfd\lang\` (that step used to be manual — a new machine
  started with only the built-in English).
- **`mfd-tui` — the languages: English in the binary, the others as
  files (phase 1 of docs/DESIGN-I18N.md)**: the TUI is born English
  out of the factory and every other language is a JSON file you drop
  into `%APPDATA%\mfd\lang\` (or `~/.config/mfd/lang/`): the `Shift+L` /
  F12 menu — the last F twin of the keyboard — lists them (the
  built-in English plus the found ones), ENTER switches the session's
  language with the confirmation flash, F2 makes it permanent
  (`lingua=` in the profile, like the theme). From the command line:
  `mfd-tui --lang it` (or the `MFD_LANG` variable) beats everything;
  a nonexistent code warns on stderr and the start goes on with the
  rest of the chain, never an error. In the repo's `lang/` we start
  with seven: `en` (built in, the translators' reference), `it` (the
  maintainer's) and the five AI-translated ones — es, pt-BR, de, ja,
  zh-CN — which say so: the file carries the `_ai` meta with the
  disclaimer and the menu marks them `[AI]` with the notice at the
  bottom, because an unverified translation must be presented for
  what it is (verifying it is a one-line PR: delete `_ai`). All the
  TUI strings — flashes, F1, popups, headers, `-h`, errors — go
  through the vocabulary (`mfd-core lang.rs`: hand-written JSON
  reader, zero dependencies), key parity with English is a test — the
  gate of future PRs — and the existing tests assert Italian through
  the crate's `it.json`: the mechanism is explicit in the test code.
  `-json` and the `mfd-stampa-*.json` files are not touched (machine
  contract, English by definition). The round-trip with the language
  reader also brought to light an old `esc` bug: characters outside
  the BMP (the emoji file names) came out as six-digit `\u`, which is
  not valid JSON — now they come out as a surrogate pair, as JSON
  commands.

- **`mfd-cli` — the grid dresses like duf on the terminal**: when the
  output goes to a real terminal, the breakdown table takes the look of
  [duf](https://github.com/muesli/duf) (the reference of line 9 of
  `infot.txt`): rounded unicode box with the column crossings, bar
  and % in the working tint at duf's thresholds (green below half of
  the share of the total, yellow from half, red from 90%; the empty
  part of the bar in the same tint, no solid fills that in 16 colors
  would light up the screen), blue names, gray counts, yellow borders
  and headers (the accent shared with the TUI's black console) and
  the bold TOTAL closing the box, lined up under the headers — 16
  ANSI colors, the same family as the TUI. The names are not chopped:
  the box is as wide as it takes, like the ASCII grid (on a narrower
  terminal it is the one that wraps, the datum stays whole). `NO_COLOR`
  turns the colors off keeping the box, `MFD_ASCII=1` brings the flat
  grid back even on the terminal, `MFD_DUF=1` forces the duf look
  even redirected (for `less -R`). Redirected without forcing, the
  output stays the historical ASCII format byte for byte: the contract
  of logs, captures and data consumers does not change. The ANSI
  colors on Windows ask for `ENABLE_VIRTUAL_TERMINAL_PROCESSING`:
  where the console does not know them, the CLI stays alone on the
  ASCII format.
- **`mfd-tui` — the four themes become recognizable**: **Night** (the
  usual Norton/Total Commander blue), **Debian** (the installer's blue
  background, gray windows with the black text of the debconf dialogs,
  red selection), **Console** (the factory default: white on black,
  yellow accents like the CLI borders, light-green selector) and
  **CLI** (black: no imposed color, the terminal's). The Shift+T / F3 menu
  lists name and description and marks the default; the theme saved
  with F2 survives as before (`tema=` in the profile, and a retired
  code — like the old `norton` — falls back to the default). And the
  windows' shadow is now a veil: the text behind fades instead of
  disappearing under a slab of color — the popup effect of the real
  consoles (before, on Console it was a gray slab, and on Night the
  effect was lost).
- **`mfd-tui` — `Shift+P` / `F11`: the print, the JSON picture of the
  current folder as you see it**. The reporting the TUI was missing:
  while you navigate, Shift+P writes to file the JSON picture of the folder
  you are in — the same contract as `-json` (FAQ 9), because the
  serializer is promoted from mfd-cli to mfd-core (`json::full` /
  `json::subtree`) and the two frontends speak the same format, as
  principle 2 of the plan asks. The CLI keeps printing it on stdout
  byte for byte (the regression test nails it); the TUI writes it to
  file with the view's framing: the picture's root follows the
  drill-down like the Shift+R / F9 report, the branches opened with Shift+X / Shift+A
  open in the picture too, the closed ones stay rows of recursive
  totals ("children": null), the filter lens applies (non-matching
  leaves and branches invisible, filtered totals) and is declared in
  the header — the picture saves what you read on screen, not the
  whole world (that remains the CLI's `-d a -json` job). The warnings stay those
  of the branch (the rule of count_under). The file is born in the
  launch folder — not in the scanned one: mfd measures, it does not
  mutate — with the name `mfd-stampa-<name>.json`, numbered (-2,
  -3...) if the name already exists: two pictures are worth a diff.
  The flash says the path; the help (F1) and `-h` declare the key (p
  and its F11 twin, like every TUI feature has its F).

### Changed
- **`mfd-tui` — the scan screen dresses like the CLI's loader**:
  ten-frame braille spinner with the tint rotating through the twelve
  ANSI ones, rainbow bar as wide as the box (the same `#`/`.`
  alphabet of always: the bar is a product trademark) lighting up
  from red and reaching magenta, path shortened with dots and one new
  row: elapsed time and average speed in files/s. The point is not the
  luxury: the honest % estimate (closed over closed + open)
  saturates at 99% for almost the whole scan — the walker goes deep
  and the open folders stay few — and the screen looked frozen; now
  the animation lives on the clock (spinner, time, speed: something
  always moves) and the counters row carries the bytes too.
  `mfd-core` follows along: the progress picture leaves on a timer —
  every 100 ms — and no longer every 128 closed folders, a milestone
  the workers' race used to skip as a rule (whoever closes finds the
  counter already past the multiple) and that on slow disks left the
  numbers still for seconds. The % stays the honest estimate of
  always: it is not made to say what it does not know.

- **`mfd-tui` — the function-key bar declares F11 and F12 too**: the
  print (`Shift+P` / `F11`) and the language (`Shift+L` / `F12`) were F twins
  without a place in the bottom bar, which stopped at F10: the
  keyboard's geography is F1-F12, and now the bar lists them like the
  others (minimal step, Total Commander style). The vocabulary labels
  got shorter where needed (`Reescan.` → `Reesc.`, `Informe` →
  `Info.`; in Japanese the rescan goes from five to three characters)
  because twelve entries fit in 120 columns only if each one is short.
- **`mfd-cli` — the TOTAL is aligned**: the row that closes the grid
  is no longer the sentence `TOTAL: 41,4 GB - 181.506 files -
  102.992 folders` but a row of the table: every column's total under
  its own header (the size under SIZE, the files under FILES, the
  folders under FOLDERS) and in the NAME column the label with the
  entry count — `TOTAL: 108 entries`. The reading is the classic
  vertical one, with the cumulative datum below; % and bar stay empty
  (a sum is not a share of itself: the root, on top, is already
  100%). In the duf box the row closes the box in bold; in the ASCII
  grid a thread of separator opens it, like the row that closes a real
  table. The entry count no longer sits under the FILES column (where
  it looked like a file count) and is no longer that of the rows
  shown: it is the total of the names column, and `-top` does not cut
  it, like all totals. The footer of the redirected ASCII format
  changes: whoever parsed it must update (the rest of the grid does not
  change by one byte).
- **`mfd-cli` — the warnings close the grid**: the scan's reports
  (access denied and the like) are no longer a separate block after an
  empty line but rows of the table, under the TOTAL: the first cell is
  the `WARNING` label (only if there are any), all the other columns
  merge into one text cell — one report per row, split by a dash, all
  lined up. Past ten, the last row reports how many are left out
  (`... and 46 more reports`); the count up front (`WARNING: 56
  reports`) is gone. In the duf box the section is closed by a single
  crossing (one merged cell, yellow label); `-big` adopts the same
  shape in place of the block at the bottom. The redirected ASCII
  footer changes, like for the TOTAL: whoever parsed it must update.
- **`mfd-cli` — the loader dresses in braille and rainbow**: the live
  progress line of the scan takes off the ASCII suit (`|/-\` and
  counters) and puts on the ten-frame braille spinner, whose tint
  rotates through the twelve ANSI tints — one step per row, slowly:
  it scrolls, it does not strobe — the progress bar filling up
  following the spectrum (red at the head, magenta at the tail) and
  the honest % estimate of the TUI's scan screen (closed folders over
  the seen total: it can even go down when the walker discovers new
  folders, and rightly so). `NO_COLOR` keeps the new row but
  monochrome; `MFD_ASCII=1` — or a console that refuses the ANSI
  sequences on stderr, where the loader lives — brings back the
  historical loader byte for byte. Everything on stderr as before:
  bench, redirects and captures keep seeing nothing.

## [0.7.1] - 2026-10-23

`mfd-tui` 0.2.0: the list becomes a tree, the detail gets a reading
cursor and `r` / F9 opens the big-files report of the folder you are in
(steps 2-3 of the expand-all plan, decisions approved by the user: the
`a` key for the subtree, the reading cursor in the detail via TAB, SIZE
+ bar + name columns without the %).

### Added
- **`mfd-tui` — the tree in the list (`x` / F4)**: the branch under
  the cursor opens inline, the child folders appear indented under the
  parent in the right place in the lens order; closing puts the whole
  branch away, whatever depth it had reached. **`a`** opens the whole
  subtree of the branch under the cursor: the interactive equivalent
  of `-d a`, one branch at a time (the flash says how many folders it
  opened). ENTER stays the drill-down — the tree is a view, the
  navigation is not — and the open branches survive the drill-down and
  the lens (they reset only on a rescan: new tree). The filter holds
  at every level: branches without a match are neither seen nor
  crossed, like in the `-f` grid. The list columns lose the %: the
  indentation eats horizontal space and the 24-char bar already shows
  the proportion (the bar is the trademark, the % is the caption). The
  footer declares F4 (Expand) and the help (F1) the new keys.

- **`mfd-tui` — TAB and the reading cursor of the detail**: TAB brings
  the cursor to the detail, tree open or closed — the Total Commander
  panel switch — and in reading mode: arrows, PgUp/PgDn and Home/End
  scroll the panel's list, ENTER does not go in and <- does not go up,
  because who navigates stays in the tree (two ways to go in would
  never be clear). TAB again goes back to the tree. The lit border
  says where the cursor is, the pointed row sits in reverse video with
  the `>` arrow like in the list, and the cursor restarts from the top
  when the selection or the lens changes.
- **`mfd-tui` — the big report (`r` / F9)**: the biggest files of the
  folder you are in — the subtree of the current folder, root
  included, because the report follows the drill-down: pinned to the
  whole scan it would always be the same answer — the same answer as
  the CLI's `-big` (mfd-core `top_files_under`, no duplicated logic),
  with the same row (size and full path) plus the ranking number and
  the % and bar over the folder total. The top is changed in the
  window: digits rewrite it on the fly (2, 25, 100...), `+` and `-`
  move it by ten, Backspace deletes the last digit. Cross-cutting like
  the warnings: the biggest files are scattered on the branches at
  every depth, and this is where they sit together. The lens holds
  here too: with a filter on, only matching files count (the window
  declares it); a path that does not fit the width is cut at the head,
  never in the name. Arrows and PgUp/PgDn scroll, ESC or `r` close.
  The footer fills the F9 hole (Report) and the help declares the
  key.
- **`mfd-tui` — the mouse wheel**: one detent = one arrow down/up,
  wherever the arrows live — the list or the detail (where the cursor
  is), the report and the warnings windows (the windows scroll), the
  menus (the entry) — because the wheel wears keys, not the keys a
  wheel. And the **horizontal tilt** (the tilted wheel, or the two
  fingers of the touchpad) is `<-` and `->`: left goes up to the
  parent and right goes in, through the same gates as the arrow (you
  do not navigate from the detail: you go back with TAB). The side
  buttons (Back/Forward, the browser ones) cannot be had: they do not
  cross the terminal, crossterm does not see them. Drag is not used:
  to copy the terminal's text you hold Shift while the capture is on
  (the help and the FAQ say so). The design collects the exception:
  the mouse leaves the non-goals, the principle stays "everything
  from the keyboard" — the mouse is a shortcut of the gestures, not a
  way out.
- **`mfd-tui` — the mouse clicks**: the left click selects the
  pointed row of the list (the cursor goes there, the preview
  follows, like with the arrows), the click on the detail panel
  brings the cursor in reading mode like TAB (and on the pointed
  row), the double click enters the folder like ENTER and the right
  button opens or closes the branch of the clicked row like `x` / F4
  — first the cursor, then the branch, in one gesture. The menu
  entries apply on click (in the theme menu the blurb too: it is part
  of the entry). The rule is the wheel's: the mouse wears keys, never
  the other way — every gesture stays reachable from the keyboard,
  and the windows (filter, report, warnings, help) are not clickable:
  ESC and ENTER stay keys. The double click does not exist in
  crossterm: half a second on the same row, with two columns of
  tolerance. Not to miss the row where the list scrolls, the tables'
  offset becomes owned (`scroll_top`, shared between render and
  click) and the drawing declares the clickable zones at every frame
  (`Hits`): the event loop is draw-then-poll, so the zones always
  match what the user is looking at. The help (F1) declares the
  gestures.
- Tests: 76 (17 core, 54 tui, 5 cli) — 30 new: 10 for the tree (x
  and F4, recursive a, the lens holding in the open branches, TAB
  bringing the reading even with the tree closed, ENTER and <- from
  the detail not navigating, the last closed branch with the cursor
  in the detail (it stays there), the branches surviving the
  drill-down, the +/- indentation in the rendering, the cursor arrow
  in the detail) and 6 for the report (opening and closing with r,
  F9 and ESC, the lens holding for the big files too, the rendering
  with ranking, size and full path, the subtree in mfd-core
  `top_files_under`, the report of the current folder and the top
  rewritten by digits) and 3 for the wheel (the cursor where it is,
  list or detail; the report scrolling; the menus, cyclic theme and
  sort pinned at the borders; the horizontal tilt going up and back
  in like <- and ->, and not navigating from the detail) and 10 for
  the click (the pointed row and the preview following it; the double
  click entering and the tired one staying a selection; the reading
  in the detail, row or panel; the right button opening and
  re-closing the clicked branch; the clickable menus, with the theme
  blurb counting like its own entry; the wheel passing through
  on_mouse; the windows not being clickable; and the render/zones
  contract — the rows the drawing declares are the rows the drawing
  shows, scrolled list included).

### Changed
- **`mfd-tui` — Debian goes back to cream, the console takes the
  green**: the Debian theme returns to the original standard —
  cream/light-yellow background (ANSI LightYellow), black text and
  dark red accents, white-on-red selection — and the black look it
  had becomes a theme of its own, **console** (black, white, red
  accents) with the light-green selector (LightGreen). The F2 code
  `debian` keeps working (and it is now the cream one); `console` is
  the new one. The theme menu and the t key list all four.

## [0.7.0] - 2026-10-23

Phase 2: `mfd-tui` is born, and with it `mfd-core`. The engine leaves
`main.rs` and becomes a crate of the workspace; the CLI and the TUI are
two skins on the same engine (see docs/ARCHITECTURE.md). All the CLI
contracts were re-verified with before/after regressions on the outputs
(grid, filters, RESULT/ITER lines, exit codes): the deterministic
outputs are identical byte for byte.

### Added
- **`-big <N>`**: the N biggest files of the whole tree with full
  path, in fixed descending size order (on ties, ascending path:
  determinism like `--json`). The immediate answer to "what is eating
  the disk?" for batches, scripts and AIs — before, you had to
  post-process `--json`. Combinable with `-f` (the N biggest that
  match, the lens declared in the header); scans the complete tree
  like `--json`; not combinable with `-d`, `-o`, `-top`, `--json`,
  `MFD_BENCH`, loops. The WARNING block at the bottom stays the grid's
  one (no TOTAL: the list is the answer).
- **`-d a` / `-d all`**: the breakdown table at unlimited depth — the
  same complete scan as `--json`, shown as a tree; how many rows to
  see is governed by `-top`, the TOTAL stays the closing contract.
- **`--json`**: the complete snapshot of the tree as pure-ASCII JSON —
  the answer to the question in the queue since forever ("I want to
  diff before/after a cleanup", FAQ entry 9). Children and leaves in
  descending size order: two scans of the same folder diff only if the
  disk changed. Without an explicit `-d` it scans the whole tree; not
  combinable with `MFD_BENCH`, loops, `-o`, `-top`.
- **`mfd-core` 0.7.0**: the extracted engine — parallel walker,
  breakdown tree, filters, warnings. API: `scan` / `scan_with_progress`
  (progress via callback, cancellation with no partial results), `Tree`
  with the views (`rows_of`, `sort_rows`, `subtree_match`, `path_of`).

- **`mfd-tui` 0.1.0**: interactive navigation of the tree (ratatui +
  crossterm): scan with live progress and ESC cancelling, double panel
  list + detail with a single lens (s and f hold for both; on top of
  the detail the selection's totals: size, files, folders),
  drill-down with ENTER, the filter window (f / F7) with instructions
  and examples of the two syntaxes — glob and `REG(...)` — where a
  filter that does not compile stays in the window with the error in
  sight, to fix it on the spot, the warnings window (w / F8) that
  declares by itself what the reports are and what they mean for the
  totals, four themes (Norton blue, Debian cream, black console,
  dark minimal) with the shadow on the windows, **F2** saving the
  settings (theme, lens, bar) to an ASCII file in the user profile
  and the next start picking them up again (F5 rescans: the key F2
  freed). The approved design is in docs/DESIGN-TUI.md.
- **The CLI loader**: during the scan a live line on stderr (spinner,
  files, folders, bytes) updates the counters and disappears when the
  results arrive — no more apparently frozen wait on the big folders.
  Interactive terminal only, never with `MFD_BENCH` or loops: stdout
  and the RESULT/ITER contract do not change by one byte.
- NOTE in the notes block: the no-match folders hidden by the `-f`
  filter (see "Changed") are reported, not silent.

### Changed
- **No arguments = the current folder**: `mfd-cli` and `mfd-tui`
  without arguments start from the current folder (like `du` and
  `ncdu`); before, they showed the help. Whoever types the command
  and ENTER gets the answer they expect; the help stays on `-h`.
- **`-f` hides the no-match folders**: in the filtered grid the
  folders that do not contain even one matching file are not printed
  (they would be 0 B and 0-file rows: noise); the notes block
  reports them with `NOTE: N folders without a match hidden by the
  filter`. It was the question in the queue for five sessions.
- The `mfd-cli` crate depends on `mfd-core` (regex stays indirect,
  the engine's).

### Unchanged (verified)
- Breakdown grid with bars, % and files as leaves; `-o`, `-top`
- Notes block at the bottom: WARNING with count, singular/plural,
  truncation
- `MFD_BENCH` contract (RESULT/ITER lines), exit codes, pure ASCII
  output (no arguments now means the current folder: see Changed)

## [0.6.2] - 2026-10-23

First version in the public repository. No functional change from the
0.6.2 developed in private: the house changes, not the code. All the
contracts were re-verified after the migration (grid, filters,
warnings, RESULT/ITER line, totals matching across walkers).

### Changed
- **Command rename: `mfd-scan` -> `mfd-cli`** — crate, binary and
  self-references (help, examples, totals line with `-d 0`). The
  `mfd` prefix is now consistent with the project's name.
- The bench crate `walker-rs` becomes `rust\crates\mfd-cli`, member
  of a Cargo workspace ready for `mfd-core` and `mfd-tui` (see
  docs/ARCHITECTURE.md).
- The C# walker becomes `net\src\Manto.FreeDisk.Cli` (the twin of
  the comparison).
- The phase-0 Node prototype lives in `node\mfd-scan\` (frozen).
- Project license: GNU AGPLv3.

### Unchanged (verified)
- Breakdown grid with bars, % and files as leaves
- `-o/--order`, `-top` with the cut NOTE, `-f/--filter` (glob and
  `REG(...)`)
- Notes block at the bottom: WARNING with count, singular/plural and
  truncation at 10 entries
- `MFD_BENCH` contract (RESULT/ITER lines) for the bench scripts
- No arguments = help; pure ASCII output

## 0.5 - 0.6 series (private development)

Building the CLI on the winning walker: `--depth` grid with sorting
and cut, filters, notes block and warnings, machine-readable contract
for the benches, compact totals line.

## Phases 0 - 1 (pre-version)

- Phase 1: stack-selection bench — parallel walker in Node, C# and
  Rust, same logic, run/startup/mem scripts with interleaved rounds
  and cross-checked totals.
- Phase 0: the Node `mfd-scan` prototype (report, aggregated ls,
  navigation).
