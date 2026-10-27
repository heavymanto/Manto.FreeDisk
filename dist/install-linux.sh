#!/bin/bash
# install.sh (ZIP version) - publishes mfd-cli and mfd-tui as user commands.
# Same semantics as the repo's original (rust/install.ps1), for Linux:
# binaries go to ~/.local/bin, languages to ~/.config/mfd/lang (XDG).
# The installed copy is a snapshot: re-run after every new tar.gz.
set -e

BIN_DIR="$HOME/.local/bin"
LANG_DIR="$HOME/.config/mfd/lang"

mkdir -p "$BIN_DIR"

for name in mfd-cli mfd-tui; do
    SRC="$(dirname "$0")/$name"
    if [ ! -f "$SRC" ]; then
        echo "missing $SRC - the archive is incomplete" >&2
        exit 1
    fi
    DST="$BIN_DIR/$name"
    cp -f "$SRC" "$DST"
    chmod +x "$DST"
    echo "installed: $DST"
done

# language registry: the JSON files from lang/ go to the user profile,
# where CLI and TUI discover them at runtime (English is already embedded).
LANG_SRC="$(dirname "$0")/lang"
if [ ! -d "$LANG_SRC" ]; then
    echo "language folder missing: $LANG_SRC" >&2
    exit 1
fi
mkdir -p "$LANG_DIR"
cp -f "$LANG_SRC"/*.json "$LANG_DIR/"
echo "languages installed: $(ls "$LANG_DIR"/*.json | wc -l) JSON in $LANG_DIR"

echo "self-test:"
"$BIN_DIR/mfd-cli" -V