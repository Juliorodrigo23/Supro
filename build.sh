#!/bin/bash
set -euo pipefail

echo "Building bundle..."
cargo bundle --release

APP="target/release/bundle/osx/Arm Tracker.app"
PLIST="$APP/Contents/Info.plist"
RES="$APP/Contents/Resources"

# Your .icns built from PNG beforehand and tracked by Cargo.toml:
# [package.metadata.bundle]
# icon = ["assets/AppIcon.icns"]
ASSETS_ICNS="assets/AppIcon.icns"

echo "Ensuring AppIcon.icns exists inside the bundle..."
if [[ ! -f "$RES/AppIcon.icns" ]]; then
  echo "  - AppIcon.icns not found in bundle Resources. Attempting to copy from assets/..."
  mkdir -p "$RES"
  if [[ -f "$ASSETS_ICNS" ]]; then
    cp "$ASSETS_ICNS" "$RES/AppIcon.icns"
    echo "  - Copied: $ASSETS_ICNS -> $RES/AppIcon.icns"
  else
    echo "ERROR: $ASSETS_ICNS not found. Build AppIcon.icns from your 1024x1024 PNG first."
    echo "Hint:"
    echo "  mkdir -p assets/AppIcon.iconset"
    echo "  sips -z 16 16     appicon_1024.png --out assets/AppIcon.iconset/icon_16x16.png"
    echo "  sips -z 32 32     appicon_1024.png --out assets/AppIcon.iconset/icon_16x16@2x.png"
    echo "  sips -z 32 32     appicon_1024.png --out assets/AppIcon.iconset/icon_32x32.png"
    echo "  sips -z 64 64     appicon_1024.png --out assets/AppIcon.iconset/icon_32x32@2x.png"
    echo "  sips -z 128 128   appicon_1024.png --out assets/AppIcon.iconset/icon_128x128.png"
    echo "  sips -z 256 256   appicon_1024.png --out assets/AppIcon.iconset/icon_128x128@2x.png"
    echo "  sips -z 256 256   appicon_1024.png --out assets/AppIcon.iconset/icon_256x256.png"
    echo "  sips -z 512 512   appicon_1024.png --out assets/AppIcon.iconset/icon_256x256@2x.png"
    echo "  sips -z 512 512   appicon_1024.png --out assets/AppIcon.iconset/icon_512x512.png"
    echo "  cp appicon_1024.png assets/AppIcon.iconset/icon_512x512@2x.png"
    echo "  iconutil -c icns assets/AppIcon.iconset -o assets/AppIcon.icns"
    exit 1
  fi
fi

echo "Setting icon and permissions keys in Info.plist..."

# Ensure CFBundleIconName = AppIcon (no extension)
# If the key exists, set; otherwise, add.
if /usr/libexec/PlistBuddy -c 'Print :CFBundleIconName' "$PLIST" >/dev/null 2>&1; then
  /usr/libexec/PlistBuddy -c 'Set :CFBundleIconName AppIcon' "$PLIST"
else
  /usr/libexec/PlistBuddy -c 'Add :CFBundleIconName string AppIcon' "$PLIST"
fi

# If legacy CFBundleIconFile exists and has an extension, fix it (macOS prefers name w/o extension)
if /usr/libexec/PlistBuddy -c 'Print :CFBundleIconFile' "$PLIST" >/dev/null 2>&1; then
  /usr/libexec/PlistBuddy -c 'Set :CFBundleIconFile AppIcon' "$PLIST" || true
fi

# (Optional) Set App Store category string to Developer Tools
if /usr/libexec/PlistBuddy -c 'Print :LSApplicationCategoryType' "$PLIST" >/dev/null 2>&1; then
  /usr/libexec/PlistBuddy -c 'Set :LSApplicationCategoryType public.app-category.developer-tools' "$PLIST"
else
  /usr/libexec/PlistBuddy -c 'Add :LSApplicationCategoryType string public.app-category.developer-tools' "$PLIST"
fi

# Add/Update privacy usage descriptions
if /usr/libexec/PlistBuddy -c 'Print :NSCameraUsageDescription' "$PLIST" >/dev/null 2>&1; then
  /usr/libexec/PlistBuddy -c 'Set :NSCameraUsageDescription This app requires camera access to track arm rotation movements for analysis.' "$PLIST"
else
  /usr/libexec/PlistBuddy -c 'Add :NSCameraUsageDescription string This app requires camera access to track arm rotation movements for analysis.' "$PLIST"
fi

if /usr/libexec/PlistBuddy -c 'Print :NSMicrophoneUsageDescription' "$PLIST" >/dev/null 2>&1; then
  /usr/libexec/PlistBuddy -c 'Set :NSMicrophoneUsageDescription Arm Tracker may record audio while capturing video.' "$PLIST"
else
  /usr/libexec/PlistBuddy -c 'Add :NSMicrophoneUsageDescription string Arm Tracker may record audio while capturing video.' "$PLIST"
fi

# (Nice to have) Mark high-DPI capable
if /usr/libexec/PlistBuddy -c 'Print :NSHighResolutionCapable' "$PLIST" >/dev/null 2>&1; then
  /usr/libexec/PlistBuddy -c 'Set :NSHighResolutionCapable true' "$PLIST"
else
  /usr/libexec/PlistBuddy -c 'Add :NSHighResolutionCapable bool true' "$PLIST"
fi

# (Strongly recommended) bump CFBundleVersion to avoid icon caching issues
# If CFBundleVersion is numeric, increment; else set a fresh numeric build
if /usr/libexec/PlistBuddy -c 'Print :CFBundleVersion' "$PLIST" >/dev/null 2>&1; then
  CUR_VER="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleVersion' "$PLIST" || echo 0)"
  if [[ "$CUR_VER" =~ ^[0-9]+$ ]]; then
    NEW_VER=$((CUR_VER + 1))
  else
    NEW_VER="$(date +%s)"
  fi
  /usr/libexec/PlistBuddy -c "Set :CFBundleVersion $NEW_VER" "$PLIST"
else
  /usr/libexec/PlistBuddy -c "Add :CFBundleVersion string $(date +%s)" "$PLIST"
fi

echo ""
echo "✓ Bundle created successfully"
echo "Verifying keys..."
/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$PLIST" || echo "CFBundleIdentifier missing (check Cargo.toml identifier)"
/usr/libexec/PlistBuddy -c 'Print :CFBundleIconName' "$PLIST" || echo "CFBundleIconName missing"
ls -l "$RES/AppIcon.icns" || true
echo ""

# After creating the bundle, copy Python files
PYTHON_DIR="$APP/Contents/Resources/python"
mkdir -p "$PYTHON_DIR"
cp -r python/*.py "$PYTHON_DIR/"
echo "Copied Python scripts to bundle"

# Replace old app in /Applications to avoid duplicate cache entries
DEST="/Applications/Arm Tracker.app"
if [[ -d "$DEST" ]]; then
  echo "Removing old /Applications bundle..."
  rm -rf "$DEST"
fi
echo "Copying new bundle to /Applications..."
cp -R "$APP" "$DEST"

echo "Refreshing Dock/Finder to clear icon caches..."
killall Dock || true
killall Finder || true

echo "Opening app..."
open "$DEST"

echo "Done."
