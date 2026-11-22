#!/usr/bin/env bash
# Peek Linux Distribution Builder
# Builds a standalone tarball and .deb package for Linux.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$ROOT_DIR"

VERSION="0.1.0"
ARCH="x86_64"
DIST_DIR="$ROOT_DIR/dist"
mkdir -p "$DIST_DIR"

if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
  echo "==> 1. Building Svelte UI bundle..."
  (cd ui && npm run build)

  echo "==> 2. Compiling Rust release binary..."
  cargo build --release -p peek-app
fi

BIN_SRC="$ROOT_DIR/target/release/Peek"
if [[ ! -f "$BIN_SRC" ]]; then
  echo "Error: Binary not found at $BIN_SRC"
  exit 1
fi

echo "==> 3. Creating Linux tarball distribution..."
TARBALL_DIR="$ROOT_DIR/target/Peek-linux-$ARCH"
rm -rf "$TARBALL_DIR"
mkdir -p "$TARBALL_DIR"

cp "$BIN_SRC" "$TARBALL_DIR/Peek"
chmod +x "$TARBALL_DIR/Peek"
cp "$ROOT_DIR/assets/peek.desktop" "$TARBALL_DIR/peek.desktop"
cp "$ROOT_DIR/assets/icon_512.png" "$TARBALL_DIR/peek.png"
if [[ -f "$ROOT_DIR/README.md" ]]; then
  cp "$ROOT_DIR/README.md" "$TARBALL_DIR/README.md"
fi
if [[ -f "$ROOT_DIR/LICENSE" ]]; then
  cp "$ROOT_DIR/LICENSE" "$TARBALL_DIR/LICENSE"
fi

cat > "$TARBALL_DIR/install.sh" << 'EOF'
#!/usr/bin/env bash
set -e
PREFIX="${PREFIX:-/usr/local}"
ICON_DIR="${PREFIX}/share/icons/hicolor/512x512/apps"
APP_DIR="${PREFIX}/share/applications"
BIN_DIR="${PREFIX}/bin"

if [ "$EUID" -ne 0 ] && [ "$PREFIX" = "/usr/local" ]; then
  PREFIX="${HOME}/.local"
  ICON_DIR="${PREFIX}/share/icons/hicolor/512x512/apps"
  APP_DIR="${PREFIX}/share/applications"
  BIN_DIR="${PREFIX}/bin"
fi

mkdir -p "$BIN_DIR" "$APP_DIR" "$ICON_DIR"
cp Peek "$BIN_DIR/Peek"
chmod +x "$BIN_DIR/Peek"
cp peek.desktop "$APP_DIR/peek.desktop"
cp peek.png "$ICON_DIR/peek.png"

echo "Peek installed successfully to $PREFIX"
echo "Run 'Peek' or launch it from your application menu."
EOF
chmod +x "$TARBALL_DIR/install.sh"

cat > "$TARBALL_DIR/uninstall.sh" << 'EOF'
#!/usr/bin/env bash
set -e
PREFIX="${PREFIX:-/usr/local}"
if [ ! -f "${PREFIX}/bin/Peek" ] && [ -f "${HOME}/.local/bin/Peek" ]; then
  PREFIX="${HOME}/.local"
fi

rm -f "${PREFIX}/bin/Peek"
rm -f "${PREFIX}/share/applications/peek.desktop"
rm -f "${PREFIX}/share/icons/hicolor/512x512/apps/peek.png"
echo "Peek uninstalled from $PREFIX."
EOF
chmod +x "$TARBALL_DIR/uninstall.sh"

tar -czf "$DIST_DIR/Peek-linux-$ARCH.tar.gz" -C "$ROOT_DIR/target" "Peek-linux-$ARCH"
echo "==> Created: $DIST_DIR/Peek-linux-$ARCH.tar.gz"

# Build Debian package if dpkg-deb exists
if command -v dpkg-deb >/dev/null 2>&1; then
  echo "==> 4. Assembling Debian .deb package..."
  DEB_DIR="$ROOT_DIR/target/peek_${VERSION}_amd64"
  rm -rf "$DEB_DIR"
  mkdir -p "$DEB_DIR/DEBIAN"
  mkdir -p "$DEB_DIR/usr/bin"
  mkdir -p "$DEB_DIR/usr/share/applications"
  mkdir -p "$DEB_DIR/usr/share/icons/hicolor/512x512/apps"
  mkdir -p "$DEB_DIR/usr/share/icons/hicolor/256x256/apps"
  mkdir -p "$DEB_DIR/usr/share/icons/hicolor/128x128/apps"
  mkdir -p "$DEB_DIR/usr/share/icons/hicolor/64x64/apps"
  mkdir -p "$DEB_DIR/usr/share/icons/hicolor/32x32/apps"

  cp "$BIN_SRC" "$DEB_DIR/usr/bin/Peek"
  chmod 755 "$DEB_DIR/usr/bin/Peek"
  cp "$ROOT_DIR/assets/peek.desktop" "$DEB_DIR/usr/share/applications/peek.desktop"
  chmod 644 "$DEB_DIR/usr/share/applications/peek.desktop"

  cp "$ROOT_DIR/assets/icon_512.png" "$DEB_DIR/usr/share/icons/hicolor/512x512/apps/peek.png"
  cp "$ROOT_DIR/assets/icon_256.png" "$DEB_DIR/usr/share/icons/hicolor/256x256/apps/peek.png"
  cp "$ROOT_DIR/assets/icon_128.png" "$DEB_DIR/usr/share/icons/hicolor/128x128/apps/peek.png"
  cp "$ROOT_DIR/assets/icon_64.png" "$DEB_DIR/usr/share/icons/hicolor/64x64/apps/peek.png"
  cp "$ROOT_DIR/assets/icon_32.png" "$DEB_DIR/usr/share/icons/hicolor/32x32/apps/peek.png"

  cat > "$DEB_DIR/DEBIAN/control" << EOF
Package: peek
Version: ${VERSION}
Section: web
Priority: optional
Architecture: amd64
Maintainer: Sayan Senapati <senapati@users.noreply.github.com>
Depends: libgtk-3-0, libwebkit2gtk-4.1-0 | libwebkit2gtk-4.0-37
Homepage: https://github.com/senapati484/peek
Description: Minimal, blazing-fast native browser built in Rust
 Peek is a modern, lightweight, keyboard-driven native web browser
 powered by Rust and WebKit. It features instant previews, integrated
 ad-blocking, tab grouping, pinned websites, and a minimal UI.
EOF

  dpkg-deb --build --root-owner-group "$DEB_DIR" "$DIST_DIR/peek_${VERSION}_amd64.deb"
  echo "==> Created: $DIST_DIR/peek_${VERSION}_amd64.deb"
fi

echo "==> Linux packaging complete!"
