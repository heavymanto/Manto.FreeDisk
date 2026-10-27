# DESIGN-I18N — internationalization: English in the binary, languages as files

> Decided 2026-10-25. The project wants to speak to the world: the UI is
> born English out of the factory, and every other language — Italian
> included — is a file the community can add, correct, complete, without
> touching a line of code. This document is the contract of the system;
> phase 1 (§7) landed downstream of the `Shift+P` print commit — the seven
> starter languages and the AI disclaimer (§2.5) are the request that
> lit it up.

## 1. The problem, back then

The UI strings lived in the code, in Italian: the TUI flashes, the F1
help, the `-h` of both frontends, the CLI errors, the grid headers.
Whoever did not read Italian understood everything about the project —
README.md is English — except the product itself.

The surface is small and bounded: ~150-200 keys in all (flashes, helps,
menus, popups, headers, errors, WARNING rows). It is not framework work:
it is one well-governed dictionary. The bulk of the cost is not the
system, it is the extraction of the strings — once.

## 2. The decisions

### 2.1 English embedded, the other languages are files

- `en.json` is **embedded in the binary** (`include_str!`): the
  guaranteed fallback, always available even without a single file on
  disk. It is also the translators' reference: English keys and values
  are the base vocabulary.
- Italian — like the German or the Spanish to come — is a **community
  file**: it lives in the repo next to `en.json`, installs by copying it
  into the profile's language folder, activates from the menu or with
  `--lang`. The cost is declared: the Italian user depends on a file;
  the return is a binary that speaks the lingua franca everywhere,
  without assumptions about the disk.
- The degradation is per key, not per file: an incomplete language
  shows English only where the translation is missing. A new feature is
  born English and the community catches up: it is the right model for
  a contributor-driven project, not a defect.

### 2.2 One folder, one file per language

`lang/` in the engine's crate, one JSON file per language, the stem of
the name is the code (`en.json`, `it.json`, `pt-BR.json`). The
`languages.json` monolith is rejected on every front that counts: the
German translator touches only their own file (no merge conflicts), one
malformed file does not contaminate the other languages, the "add
French" PR is one new file that reads in ten seconds. It is also the
repo's precedent: one language, one file — like `README.md` and
`README.it.md`.

### 2.3 Two layers: embedded + discovered at runtime

```
repo (source of truth):
  rust/crates/mfd-core/lang/en.json   <- the only EMBEDDED one (include_str!)
  rust/crates/mfd-core/lang/it.json   <- first community language (and the future ones)

runtime (discovered at every start, zero code):
  %APPDATA%\mfd\lang\*.json           <- the user drops a file -> the language
  ~/.config/mfd/lang/ elsewhere          shows up in the menu at the next start
```

The loader lives in `mfd-core/src/lang.rs` — the same move as
`json.rs`: the engine owns the infrastructure, CLI and TUI consume it.
The profile folder follows the logic of `settings::default_path`; a file
with the same code as an embedded one overrides it (the profile beats
the factory).

### 2.4 What never gets translated

- **`-json` and the `mfd-stampa-*.json` files**: machine contract,
  byte-per-byte (FAQ §9 and §12). The JSON keys stay English by
  definition.
- **Comments, test names, the bench scripts**: the maintainer's voice,
  they stay Italian. No value for the user, huge churn, zero return.
- **The documents**: English by convention — decided 2026-10-26, on the
  eve of publication: one language everybody reads, `README.it.md` the
  single Italian counterpart, and no other doc translations in the repo
  (a community wanting `docs/de/FAQ.md` opens a PR, as for the code).
  It is not runtime, it is not this system.

### 2.5 The starter translations speak AI — and say so

The first languages beyond English and Italian (es, pt-BR, de, ja,
zh-CN) are born as AI-generated translations: nobody in the project has
verified them. The rule is declared honesty, not implied promise:

- the file carries the `_ai` meta: its value is the disclaimer, written
  in the language itself, readable by anyone who opens the file;
- the `Shift+L` / `F12` menu marks those languages with `[AI]` and closes
  with the warning row ("AI-translated: may be imprecise or wrong") —
  whoever chooses knows what they are choosing, while they are
  choosing;
- `en.json` carries no mark (it is the embedded reference, the base
  vocabulary) and neither does `it.json` (it is the maintainer's
  language, extracted from the existing UI and verified by use);
- the mark comes off with one line: when a person verifies a
  translation, `_ai` gets deleted and the entry leaves the menu clean.
  The "I checked the German" PR is welcome exactly like the one that
  adds a new language.

## 3. The format and the loader

JSON, because it is what every translator in the world recognizes
(syntax highlighting, validation, tooling everywhere). The reader,
though, is **written by hand** in `lang.rs` (~100 lines): flat
string→string objects, standard escapes, no arrays, no numbers, no
nesting. The dependency policy is not touched (`regex` stays the core's
only one, no serde). The symmetry is pleasing: `json.rs` writes JSON,
`lang.rs` reads it — the pair lives in the engine.

```json
{
  "_name": "Italiano",
  "tui.flash.stampa_ok": "stampa scritta: {percorso}",
  "tui.aiuto.titolo": "TASTI",
  "cli.header.size": "DIMENSIONE"
}
```

- The keys are **stable IDs with a prefix** (`tui.*`, `cli.*`), not
  text: rephrasing an English flash does not orphan twenty translations.
  The `_` keys (like `_name`) are meta: the language's name in the menu.
- The values of `it.json` are **the Italian of the time, word for
  word**: extraction, not rewriting.
- The long helps (F1, `-h`) become one key per line: the translator
  works line by line and the wrapping stays governed.
- Dynamic values pass through `{percorso}`, `{n}` placeholders.

## 4. Selection and fallback

Precedence: `--lang it` > `MFD_LANG=it` (configuration envs are a
precedent: `MFD_BENCH`, `MFD_ASCII`) > the `lingua=` line in the TUI
settings (F2 writes it, `from_text` is tolerant: old files and old
binaries ignore each other without errors) > English.

The lookup chain per key: active language → embedded English → the key
prints itself on screen. A gap is visible right away and fixed in a
minute.

In the TUI the `Shift+L` / `F12` key opens the language popup, populated
with the found ones: embedded + discovered. The twin follows the TUI
rule — every feature has its F (`Shift+P` / `F11` is the print, `Shift+L` / `F12`
is the language: they are the only free Fs beyond F10). ESC cancels, the
choice flashes, F2 makes it permanent. A nonexistent code → warning and
English.

A discovered but unreadable file is not an error: the language does not
appear in the menu and the report goes where the things that could not
be read go — the `Shift+W` window of the TUI, one line on stderr in the CLI.
Never a crash.

## 5. The numbers: deferred, for a precise reason

Today the format is hardwired Italian: `1.234,5` (`fmt_bytes` with the
dot replace, `group_digits` with the thousands dots). Localizing it is
easy — two meta keys, `_decimal` and `_group` — but **it is a
signature on a contract**: the redirected grid keeps the format it has
always had (the `main.rs` header promises it: from phase 2 the words
follow -lang and MFD_LANG, the numbers do not) and changing a separator
changes the bytes. Phase 1 leaves the numbers as they are; if one day
it happens, it happens by saying so in the CHANGELOG, not in secret.

## 6. The proofs

- **Round-trip of the pair**: `json.rs` writes a dictionary, `lang.rs`
  reads it back. The project owns writing and reading: they get tested
  together.
- **Key parity**: every file in `lang/` must have the same key set as
  `en.json`. It is the automatic gate of PRs: an incomplete language
  fails the test, completeness is not reviewed by hand.
- **The system**: missing key → the key itself; malformed file →
  skipped with its report; the profile overrides the embedded one;
  `--lang` beats `MFD_LANG` which beats `lingua=`.
- **The 102 existing tests stay green**: they assert Italian, and
  Italian stays — under `#[cfg(test)]` the TUI loads `it.json` from the
  crate (`CARGO_MANIFEST_DIR/lang/it.json`), the mechanism is explicit
  in the test code, not a hidden default. The English road is covered
  by the new loader and menu tests.

## 7. The phases

1. **Foundation** (done): `mfd-core::lang` (reader, registry,
   discovery, fallback, tests) + the seven starter languages in
   `lang/` — `en.json` embedded (the reference), the maintainer's
   `it.json` and the five AI-translated ones (§2.5) — + the Shift+L / F12
   key + `lingua=` in F2 + `--lang`/`MFD_LANG`. The key-parity gate is
   a test: PRs with an incomplete language do not pass.
2. **The CLI** (done): grid headers, errors, `-h`, WARNING, loader —
   same registry, same folder.
3. **The community**: a section of CONTRIBUTING ("copy `it.json`,
   translate the values, open the PR — key parity is the gate") and the
   releases carrying the `lang/` files; whether the installation
   copies them into the profile or they stay to download was to be
   decided here, in front of the concrete case (decided: `install.ps1`
   copies them).
4. **(Optional, deliberate) the numbers**: `_decimal`/`_group` and the
   CHANGELOG entry declaring the contract change.

## 8. How to add a language (the pact with the community)

1. Copy `it.json` to `<code>.json` (the stem is the language code).
2. Translate the values, not the keys; `_name` is how the language
   calls itself in the menu. Missing keys show English: better few and
   good.
3. Open the PR: the parity test says what is missing, byte for byte.
4. Accepted, the language lives in the repo and travels with the
   releases; whoever wants to try it first drops it into
   `%APPDATA%\mfd\lang\` — the next start sees it in the menu, zero
   code touched.
