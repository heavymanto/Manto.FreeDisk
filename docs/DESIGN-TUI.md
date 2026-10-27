# DESIGN-TUI — mfd-tui, the interactive interface (phase 2)

> Status: **implemented** — mfd-tui 0.1.0, shipped with 0.7.0 (draft 3,
> all five decisions of section 8 approved by the user). The drawings
> below remain design wireframes; for real usage see docs/FAQ.md entry
> 10. Four clarifications issued during implementation: the scan % is an
> estimate (section 3), the lens hides no-match folders like the CLI
> since 0.7.0 (section 4), the windows carry the overlap shadow and
> the theme is chosen with Shift+T / F3 (section 6); without arguments
> mfd-tui starts from the current folder. Simplification after the
> field test: one lens only (section 8, decision 1) — no TAB. And
> still after the field test: the filter (Shift+F / F7) is a window with
> instructions and examples of the two syntaxes (glob and REG(...)) —
> no longer a line at the bottom — and the warnings window (Shift+W / F8)
> declares by itself what the reports are and what they mean for the
> totals.
>
> **v2 (mfd-tui 0.2.0, with CLI 0.7.1): the tree and the reading
> cursor.** The left list becomes a tree: Shift+X / F4 opens/closes the
> branch under the cursor (the children appear indented), Shift+A
> opens the whole subtree — the interactive equivalent of -d a. TAB brings the
> cursor to the detail, tree open or closed — the Total Commander panel
> switch: in reading mode — you scroll up and down, ENTER does not go
> in and <- does not go up, because who navigates stays in the tree
> (two ways to go in would never be clear); TAB again goes back. The
> list columns lose the % (SIZE + bar + name): the indentation eats
> horizontal space and the 24-char bar already shows the proportion.
> With this, decision 5 ("drill-down only, no inline tree") is
> superseded — the tree is a view, ENTER stays navigation — and
> decision 1 is completed: the lens stays one, the detail gets its own
> cursor — of reading, not of navigation.
>
> **v2 (0.7.1): the report (`Shift+R` / F9).** The biggest files of the
> folder you are in — the report follows the drill-down, a report
> pinned to the whole scan would always say the same thing — the
> `-big` answer for the current folder: cross-cutting like the
> warnings (the big files are scattered over the branches), the lens
> holds here too, the top is changed by digits, and you scroll and
> close — an answer to read, not a place to navigate.
>
> **v2: the print (`Shift+P` / `F11`).** The JSON picture of what you are
> looking at, AS YOU SEE IT — the same contract as `-json`, because the
> serializer is promoted from the CLI into mfd-core (`json::full` /
> `json::subtree`, then `json::view` / `json::report`): the CLI prints
> it on stdout, the TUI writes it to file — reporting leaves the
> terminal. The framing is the screen, both panes: the `tree` section
> is the left one — the branches opened with Shift+X / Shift+A open
> in the picture too, the closed ones stay rows of recursive totals (`"children":
> null`), the filter lens holds and is declared in the header — while
> the `detail` section is the right one: the selected row and the
> children the preview shows, which the tree may still keep closed. The
> picture saves what you read on screen, not the whole world (that is
> the CLI's `-d a -json` job). With the report window open (`Shift+R` / F9) the picture
> is of the report: the top asked and the big files with their ranking.
> The files are born in the launch folder (`mfd-stampa-<name>.json`
> for the view, `mfd-report-<name>.json` for the report, numbered if
> the name already exists); the flash says where.

## 1. Vision and aesthetics

Navigating the disk-space tree with the aesthetics of the classics:

- **aptitude / the Debian installer**: bordered panels, progress
  windows, ncurses sobriety
- **Total Commander** (Norton Commander / Turbo Vision descent): the
  function-key bar at the bottom, reverse-video selection, flat colors

Principles:

1. **Everything from the keyboard.** Arrows, ENTER, function keys. The
   mouse has been there since 0.7.1, and it wears keys: the wheel is
   an arrow (the horizontal tilt is <- and ->), the click selects, the
   double click is ENTER, the right button is x. Every gesture has its
   key — the mouse is a shortcut, never a way out (drag stays with the
   terminal; the side buttons, Back/Forward, do not cross the
   terminal).
2. **16 ANSI colors.** No truecolor: it must render well everywhere.
3. **Unicode box-drawing borders** (─ │ ┌ ┐ └ ┘): the CLI stays pure
   ASCII by sacred contract; the TUI is another surface and takes the
   "old school" borders, which modern terminals render perfectly.
4. **Product coherence.** Same columns, same vocabulary as the CLI
   grid: whoever knows mfd-cli recognizes mfd-tui at a glance.

## 2. The library

**ratatui** + crossterm (backend): the de-facto standard of Rust
TUIs, renders exactly this style. Rejected alternative: bare
crossterm, too low-level for the goal.

## 3. The scan screen (startup)

```
+-- scan --------------------------------------------+
| ⠋ C:\Windows                                      |
|                                                   |
| #############....................  99%           |
| 82.114 files - 45.230 folders · 41,4 GB           |
| 0:42 · 1.955 files/s                              |
|                                                   |
| ESC cancels                                       |
+---------------------------------------------------+
```

- the walker runs on the threads; the UI receives counters over a
  channel (one picture every 100 ms, on a timer: the clock decides the
  cadence, not the race of the workers)
- live counter of files/folders (and bytes): it makes the wait honest
  (and makes FAQ entry 3 visible: the second run, warm cache, shows
  up faster)
- the % is an honest estimate, not a fact: the total of the folders is
  not known a priori, so it counts closed/(closed + still open); at
  the end of the queue it is 100 out of 100. But the walker goes deep
  and the open ones stay few: the estimate saturates early at 99% and
  stays there until the end — that is why the screen dresses like the
  CLI loader (braille spinner with rotating tint, rainbow bar) and
  carries elapsed time and average speed: the animation lives on the
  clock, something always moves
- ESC cancels (no partial results, first version)

## 4. The main screen (wireframe, TWO panels)

```
+-- C:\Windows --------------- 41,4 GB - ord: size --+
| SIZE     %    BAR               NAME || DETAIL: WinSxS       |
|--------------------------------------|| 18,5 GB - 61.245 files |
|> 18,5GB  44%  ###########...  WinSxS || 7.812 folders        |
|  7,1GB   17%  ####.......... System32|| ord: name            |
|  6,8GB   16%  ####.......... assembl.||----------------------|
|  4,0GB    9%  ##............ Install.||  amd64_microsoft  4,1GB|
|  ...                                 ||  x86_microsoft   3,0GB|
|                                      ||  mfc140.dll      4,1MB|
|                                      ||  3 warnings in branch |
|--------------------------------------||----------------------|
| 1 Help  2 Save  3 Theme  5 Rescan  6 Sort  7 Filter  8 Warn 10 Quit |
+--------------------------------------------------------------+
```

Anatomy:

1. **title row** (left): path, total, the sort order of the list
2. **list** (left): the columns of the CLI grid; reverse-video
   selection `>`
3. **preview** (right): on top the totals of the selection — size,
   files and folders —, then the declared lens (sort and filter, the
   same as the list) and the content of the selected entry below: the
   same honesty as the CLI, which prints "(filter: ...)" next to the
   root — the state does not hide
4. **function-key bar** (bottom): Total Commander style

Interaction rules (v1):

- **one lens only**: Shift+S (sort) and Shift+F (filter) hold for list
  and preview TOGETHER — on the left you hunt (sort: size), on the
  right you read what the selection is made of (filter: *.dll) through
  the same lens.
  No TAB: choosing the target of the commands turned out to be one
  mental step too many, with no real gain (a simplification requested
  by the user after the field test).
- the preview has NO cursor in v1: it is a lens that scrolls by itself
  when long; navigating on the right too (two cursors, TC-style) is v2
  material. *(v2, 0.2.0: the right cursor arrived, but in reading
  mode — the detail scrolls, it does not navigate: who navigates stays
  in the tree.)*
- ENTER goes into the selected folder; the preview follows the new
  selection.
- the list shows ONLY folders (decision 3): a clean drill-down. Files
  live in the preview: the children of the selected folder, folders and
  file-leaves together, with their sizes.
- everything is a view on the in-memory tree: sorting/filtering does
  NOT rescan — the scan is paid once, the views are free. That is the
  TUI's reason to exist next to the CLI.
- with a filter active in the lens, no-match folders are hidden (the
  same rule as the -f grid since 0.7.0) and non-matching leaves do
  not appear in the preview: coherence with the CLI, no 0 B rows.

## 5. Keys (v1; v2: tree, TAB, report, print; 0.7.1: wheel and click; 0.2.1: letters want SHIFT)

Since 0.2.1 a bare letter does nothing: the command letters are pressed
with SHIFT (`Shift+P` prints, `Shift+Q` quits), while the function keys
stay direct — they are made for that. The rule was paid in the field:
a stray `q` next to `x` closed the app, scan gone. The miss is not
silent: a bare command letter flashes the shifted way to say it
(`Shift+Q`), everywhere but in the filter prompt, where the letters are
text.

| Key | Action |
|---|---|
| Up / Down | moves the cursor (tree or detail, v2) |
| wheel | 0.7.1: one detent = Up / Down, wherever the arrows live (list, detail, windows, menus) |
| wheel tilt | 0.7.1: the tilt (or the two fingers of the touchpad) is <- and ->: left goes up to the parent, right goes in — through the same gates as the arrow. The side buttons (Back/Forward, the browser ones) do not cross the terminal: they cannot wear keys |
| click | 0.7.1: selects the pointed row of the list; on the detail it brings the cursor in reading mode (like TAB); on menu entries it applies |
| double click | 0.7.1: enters the folder of the clicked row (like ENTER) |
| right button | 0.7.1: opens or closes the branch of the clicked row (like Shift+X / F4). Drag is not used: hold Shift to select the terminal's text |
| PgUp / PgDn | page jump |
| Home / End | first / last row |
| ENTER or -> | goes into the tree folder under the cursor |
| <- or Backspace | goes up to the parent (from the tree) |
| Shift+X / F4 | v2: opens/closes the tree branch under the cursor |
| Shift+A | v2: opens the whole subtree, like the CLI's `-d a` |
| TAB | v2: reading cursor in the detail, tree open or closed (it scrolls, it does not navigate) |
| Shift+S / F6 | sorts list and detail: cycles / menu (T/A/F/C x D/C, like -o) |
| Shift+F / F7 | filters list and detail: window with instructions and examples (glob or REG(...), -f) |
| Shift+T / F3 | theme: Night, Debian, Console (default) or CLI |
| Shift+G | bar on/off |
| Shift+W / F8 | warnings window: says what the reports are and what they weigh on the totals |
| Shift+R / F9 | v2: report — the biggest files of the current folder (follows the drill-down), the `-big` answer (a window, like the warnings); the top is changed by digits, + and - |
| Shift+P / F11 | v2: the print — the JSON picture of what you see, BOTH panes: the `tree` section (the branches opened with Shift+X / Shift+A open in the picture, the closed ones stay totals rows: `"children": null`) and the `detail` section (the selected row with the children the preview shows); with the report window open (Shift+R / F9) the picture is of the report (top and big files with ranking); the lens applies and is declared in the header — to a file in the launch folder (`mfd-stampa-<name>.json` / `mfd-report-<name>.json`, numbered if it exists): the contract of `-json` (mfd-core `json::view` / `json::report`) |
| Shift+L / F12 | v3: the language — the menu of the found languages (embedded English + the profile's JSON files, mfd-core `lang`): ENTER applies to the session with a flash, F2 makes it permanent (`lingua=`). The AI translations are marked `[AI]` with the disclaimer at the bottom (docs/DESIGN-I18N.md) |
| F2 | saves the settings (theme, lens, bar, language) to the user profile |
| F5 | rescans |
| F1 or ? | help |
| Shift+Q or F10 | quits |

## 6. The theme

Four, chosen with **Shift+T / F3** (menu; ENTER applies, ESC cancels):

- **Night**: the Norton/Total Commander blue, white/cyan text, yellow
  reverse-video selection, black shadow
- **Debian**: the installer's blue background, light gray dialog
  windows with black text (the debconf ones), red accents and
  selection, black shadow
- **Console** (default): white on black, yellow accents (Yellow, the
  same as the CLI borders), light-green selector (LightGreen) — the
  cursor of the real consoles
- **CLI** (black): no imposed color (the terminal's — black, if black

  is the terminal), reverse-video selection — the "almost naked" TUI,
  like the CLI grid.

16 ANSI colors, no truecolor. Every theme also carries the colors of
its own shadow: the veil over the windows, which darkens what is
behind without erasing it — the popup effect, not a solid slab. The
choice is per-session; **F2** makes it survive: theme, lens (sort and
filter) and bar end up in an ASCII key=value file in the user profile
(%APPDATA%\mfd-tui\settings.txt on Windows,
~/.config/mfd-tui/settings.txt elsewhere) and the next start picks
them up again. F2 again updates, deleting the file brings back the
factory defaults. The file is written by the TUI, not by the user's
hand: the lines that do not pass the grammar (-o codes, themes, -f
specs) fall back to the default.

## 7. Architecture: mfd-core is born

The second consumer teaches the API. Extraction from main.rs:

```
mfd-core:  scan(path, options, progress_cb) -> Tree
           Tree::sort(criterion) - Tree::filter(pattern)
           WarningLog (the notes block)
mfd-cli:   consumes core, formats the static grid (CONTRACT UNCHANGED)
mfd-tui:   consumes core, ratatui event loop
```

Rules:

- **the CLI contract does not change by one comma** (rule 1 of
  CONTRIBUTING): same grid, same RESULT/ITER lines, same exit code
- the core does NOT format: numbers and structure; the shape belongs
  to the frontends
- the TUI never blocks on the scan: thread + progress channel

## 8. Decisions (draft 3: all taken, approved by the user)

1. **Double panel.** List on the left + preview on the right: the
   content of the selected entry through the (single) lens —
   originally two independent lenses chosen by TAB, simplified at the
   user's request after the field test. The preview has no cursor in
   v1: it is a lens; two cursors TC-style is v2 material. *(v2,
   0.2.0: the detail cursor arrived — TAB brings it in reading mode,
   tree open or closed; the lens stays one and the navigation stays in
   the tree.)*
2. **`#` and `.` bars like the CLI.** Product coherence: the bar is a
   trademark, identical everywhere. The half blocks stay an idea for a
   future theme, not for v1.
3. **Only folders in the list.** A clean drill-down; the files live
   in the preview: the children of the selected folder, folders and
   file-leaves together, with their sizes.
4. **Deletion out of v1.** mfd measures, it does not mutate: the
   before/after report of a cleanup belongs to -d a -json + diff (a CLI
   feature already in the FAQ queue). A 'd' key behind an explicit
   confirmation remains possible later, if asked for.
5. **Drill-down only, no inline tree.** Current level + ENTER: with
   two panels, the preview acts as the "expansion" of the selection.
   *(v2, 0.2.0: superseded — the inline tree arrived with Shift+X / F4
   and Shift+A: the branches open where you are looking, ENTER stays the
   (only) drill-down — you cannot go in from the detail — and the
   preview stays the detail of the selection.)*

## 9. Non-goals of the first version

Mouse drag (the wheel and the click arrived, wearing keys — vertical
and tilt —; drag did not — to select the terminal's text there is
Shift), the mouse side buttons (Back/Forward, the browser ones: they
do not cross the terminal, they cannot wear keys), a hand-editable
configuration file (the F2 one is written by the TUI), file
deletion, text search, the comparison of two scans (that is -d a -json +
diff, a CLI feature).
