#!/usr/bin/env bash
# Peek macOS App Bundle Builder
# Creates a standalone macOS Peek.app bundle with native icon and metadata.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$ROOT_DIR"

echo "==> 1. Building Svelte UI bundle..."
(cd ui && npm run build)

echo "==> 2. Compiling Rust release binary..."
cargo build --release -p peek-app

echo "==> 3. Generating macOS AppIcon.icns..."
if [[ ! -f assets/AppIcon.icns ]]; then
  TMP_ICONSET="/tmp/Peek_$$.iconset"
  mkdir -p "$TMP_ICONSET"
  sips -z 16 16     assets/logo.png --out "$TMP_ICONSET/icon_16x16.png" >/dev/null
  sips -z 32 32     assets/logo.png --out "$TMP_ICONSET/icon_16x16@2x.png" >/dev/null
  sips -z 32 32     assets/logo.png --out "$TMP_ICONSET/icon_32x32.png" >/dev/null
  sips -z 64 64     assets/logo.png --out "$TMP_ICONSET/icon_32x32@2x.png" >/dev/null
  sips -z 128 128   assets/logo.png --out "$TMP_ICONSET/icon_128x128.png" >/dev/null
  sips -z 256 256   assets/logo.png --out "$TMP_ICONSET/icon_128x128@2x.png" >/dev/null
  sips -z 256 256   assets/logo.png --out "$TMP_ICONSET/icon_256x256.png" >/dev/null
  sips -z 512 512   assets/logo.png --out "$TMP_ICONSET/icon_256x256@2x.png" >/dev/null
  sips -z 512 512   assets/logo.png --out "$TMP_ICONSET/icon_512x512.png" >/dev/null
  sips -z 1024 1024 assets/logo.png --out "$TMP_ICONSET/icon_512x512@2x.png" >/dev/null
  iconutil -c icns "$TMP_ICONSET" -o assets/AppIcon.icns
  rm -rf "$TMP_ICONSET"
fi

echo "==> 4. Assembling Peek.app bundle..."
APP_BUNDLE="Peek.app"
rm -rf "$APP_BUNDLE"
mkdir -p "$APP_BUNDLE/Contents/MacOS"
mkdir -p "$APP_BUNDLE/Contents/Resources"

cp target/release/peek "$APP_BUNDLE/Contents/MacOS/peek"
chmod +x "$APP_BUNDLE/Contents/MacOS/peek"
cp assets/AppIcon.icns "$APP_BUNDLE/Contents/Resources/AppIcon.icns"

cat > "$APP_BUNDLE/Contents/Info.plist" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>peek</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundleIdentifier</key>
    <string>dev.Peek.Peek</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>Peek</string>
    <key>CFBundleDisplayName</key>
    <string>Peek</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.15</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
</dict>
</plist>
EOF

touch "$APP_BUNDLE"

echo "==> Done! Successfully created: $ROOT_DIR/Peek.app"
echo "Launch it with:"
echo "    open Peek.app"
