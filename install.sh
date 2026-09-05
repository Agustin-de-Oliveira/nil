#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="${HOME}/.local/bin"
APP_DIR="${HOME}/.local/share/applications"
ICON_DIR="${HOME}/.local/share/icons/hicolor/128x128/apps"

mkdir -p "$BIN_DIR" "$APP_DIR" "$ICON_DIR"

if [ ! -f "${SCRIPT_DIR}/target/release/nil-shot" ] || \
   [ ! -f "${SCRIPT_DIR}/target/release/nil-clip" ] || \
   [ ! -f "${SCRIPT_DIR}/target/release/nil-notes" ]; then
    echo "Compilando suite nil en modo release..."
    make -C "$SCRIPT_DIR" build
fi

cp -f "${SCRIPT_DIR}/target/release/nil-shot" "$BIN_DIR/nil-shot"
cp -f "${SCRIPT_DIR}/target/release/nil-clip" "$BIN_DIR/nil-clip"
cp -f "${SCRIPT_DIR}/target/release/nil-notes" "$BIN_DIR/nil-notes"
chmod +x "$BIN_DIR/nil-shot" "$BIN_DIR/nil-clip" "$BIN_DIR/nil-notes"

cp -f "${SCRIPT_DIR}/crates/nil-shot/icons/128x128.png" "$ICON_DIR/nil-shot.png"
cp -f "${SCRIPT_DIR}/crates/nil-clip/icons/128x128.png" "$ICON_DIR/nil-clip.png"
cp -f "${SCRIPT_DIR}/crates/nil-notes/icons/128x128.png" "$ICON_DIR/nil-notes.png"

cp -f "${SCRIPT_DIR}/packaging/nil-shot.desktop" "$APP_DIR/nil-shot.desktop"
cp -f "${SCRIPT_DIR}/packaging/nil-clip.desktop" "$APP_DIR/nil-clip.desktop"
cp -f "${SCRIPT_DIR}/packaging/nil-notes.desktop" "$APP_DIR/nil-notes.desktop"

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$APP_DIR" >/dev/null 2>&1 || true
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "${HOME}/.local/share/icons/hicolor" >/dev/null 2>&1 || true
fi

echo "Suite nil instalada correctamente en $BIN_DIR"

if ! command -v wl-copy >/dev/null 2>&1 && ! command -v xclip >/dev/null 2>&1; then
    echo "Aviso: No se encontro wl-copy ni xclip. Instala uno para el funcionamiento del portapapeles."
fi

if ! command -v tesseract >/dev/null 2>&1; then
    echo "Aviso: No se encontro tesseract. Instala tesseract para el soporte de OCR en nil-shot."
fi
