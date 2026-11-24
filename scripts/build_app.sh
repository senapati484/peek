#!/usr/bin/env bash
# Peek macOS App Bundle Builder
# Creates a standalone macOS Peek.app bundle with native icon and metadata.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$ROOT_DIR"

if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
  echo "==> 1. Building Svelte UI bundle..."
  (cd ui && npm run build)

  echo "==> 2. Compiling Rust release binary..."
  cargo build --release -p peek-app
else
  echo "==> Skipping UI and Cargo build (SKIP_BUILD=1)"
fi

echo "==> 3. Generating macOS AppIcon.icns..."
ICON_SRC="assets/app_icon_1024.png"
if [[ ! -f "$ICON_SRC" ]]; then
  ICON_SRC="assets/logo.png"
fi

if [[ ! -f assets/AppIcon.icns || "$ICON_SRC" -nt assets/AppIcon.icns ]]; then
  TMP_ICONSET="/tmp/Peek_$$.iconset"
  rm -rf "$TMP_ICONSET"
  mkdir -p "$TMP_ICONSET"
  sips -z 16 16     "$ICON_SRC" --out "$TMP_ICONSET/icon_16x16.png" >/dev/null
  sips -z 32 32     "$ICON_SRC" --out "$TMP_ICONSET/icon_16x16@2x.png" >/dev/null
  sips -z 32 32     "$ICON_SRC" --out "$TMP_ICONSET/icon_32x32.png" >/dev/null
  sips -z 64 64     "$ICON_SRC" --out "$TMP_ICONSET/icon_32x32@2x.png" >/dev/null
  sips -z 128 128   "$ICON_SRC" --out "$TMP_ICONSET/icon_128x128.png" >/dev/null
  sips -z 256 256   "$ICON_SRC" --out "$TMP_ICONSET/icon_128x128@2x.png" >/dev/null
  sips -z 256 256   "$ICON_SRC" --out "$TMP_ICONSET/icon_256x256.png" >/dev/null
  sips -z 512 512   "$ICON_SRC" --out "$TMP_ICONSET/icon_256x256@2x.png" >/dev/null
  sips -z 512 512   "$ICON_SRC" --out "$TMP_ICONSET/icon_512x512.png" >/dev/null
  sips -z 1024 1024 "$ICON_SRC" --out "$TMP_ICONSET/icon_512x512@2x.png" >/dev/null
  iconutil -c icns "$TMP_ICONSET" -o assets/AppIcon.icns
  rm -rf "$TMP_ICONSET"
fi

echo "==> 4. Assembling Peek.app bundle..."
APP_BUNDLE="Peek.app"
rm -rf "$APP_BUNDLE"
mkdir -p "$APP_BUNDLE/Contents/MacOS"
mkdir -p "$APP_BUNDLE/Contents/Resources"

cp target/release/Peek "$APP_BUNDLE/Contents/MacOS/Peek"
chmod +x "$APP_BUNDLE/Contents/MacOS/Peek"
cp assets/AppIcon.icns "$APP_BUNDLE/Contents/Resources/AppIcon.icns"

cat > "$APP_BUNDLE/Contents/Info.plist" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>Peek</string>
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

echo "==> 5. Creating macOS DMG installer package..."
DIST_DIR="$ROOT_DIR/dist"
mkdir -p "$DIST_DIR"
DMG_VOLNAME="Peek"
DMG_FINAL="$DIST_DIR/Peek-macos-arm64.dmg"
TMP_DMG="/tmp/peek_rw_$$.dmg"
rm -f "$DMG_FINAL" "$TMP_DMG"

APP_SIZE_MB=$(du -sm "$APP_BUNDLE" | awk '{print $1}')
DMG_SIZE_MB=$((APP_SIZE_MB + 60))

hdiutil create -size "${DMG_SIZE_MB}m" -fs HFS+ -volname "$DMG_VOLNAME" -ov "$TMP_DMG" >/dev/null

MOUNT_OUT=$(hdiutil attach -nobrowse -noverify -noautoopen "$TMP_DMG")
DEV_NODE=$(echo "$MOUNT_OUT" | head -n 1 | awk '{print $1}')
MOUNT_DIR="/Volumes/$DMG_VOLNAME"

cp -R "$APP_BUNDLE" "$MOUNT_DIR/"
ln -sf /Applications "$MOUNT_DIR/Applications"

# Set Finder window bounds and side-by-side drag-and-drop icon positions
osascript << EOF >/dev/null 2>&1 || true
tell application "Finder"
    tell disk "$DMG_VOLNAME"
        open
        set current view of container window to icon view
        set toolbar visible of container window to false
        set statusbar visible of container window to false
        set the bounds of container window to {200, 150, 740, 480}
        set theViewOptions to the icon view options of container window
        set icon size of theViewOptions to 96
        set arrangement of theViewOptions to not arranged
        set position of item "Peek.app" of container window to {150, 160}
        set position of item "Applications" of container window to {390, 160}
        update without registering applications
        delay 1
        close
    end tell
end tell
EOF

sync
sleep 1
hdiutil detach "$DEV_NODE" >/dev/null 2>&1 || diskutil eject "$DEV_NODE" >/dev/null 2>&1 || true
hdiutil convert "$TMP_DMG" -format UDZO -imagekey zlib-level=9 -o "$DMG_FINAL" -ov >/dev/null
rm -f "$TMP_DMG"

# Also create universal and simple named copies in dist
cp -f "$DMG_FINAL" "$DIST_DIR/Peek-macos-universal.dmg" 2>/dev/null || true
cp -f "$DMG_FINAL" "$DIST_DIR/peek.dmg" 2>/dev/null || true
cp -f "$DMG_FINAL" "$DIST_DIR/peek-arm64.dmg" 2>/dev/null || true

echo "==> Done! Successfully created:"
echo "    App bundle: $ROOT_DIR/Peek.app"
echo "    DMG installer: $DIST_DIR/Peek-macos-arm64.dmg"
echo ""
echo "Launch it with:"
echo "    open Peek.app"
