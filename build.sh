#!/bin/bash
set -euo pipefail

echo "Building release binary..."
cargo build --release

echo "Building bundle..."
cargo bundle --release

# CHANGE THIS LINE - use the actual bundle name from cargo bundle
APP="target/release/bundle/osx/SuPro.app"  # Changed from "Arm Tracker.app"
PLIST="$APP/Contents/Info.plist"
RES="$APP/Contents/Resources"
MACOS="$APP/Contents/MacOS"

# Your .icns built from PNG beforehand and tracked by Cargo.toml
ASSETS_ICNS="assets/AppIcon.icns"

echo "Ensuring AppIcon.icns exists inside the bundle..."
if [[ ! -f "$RES/AppIcon.icns" ]]; then
  echo "  - AppIcon.icns not found in bundle Resources. Attempting to copy from assets/..."
  mkdir -p "$RES"
  if [[ -f "$ASSETS_ICNS" ]]; then
    cp "$ASSETS_ICNS" "$RES/AppIcon.icns"
    echo "  - Copied: $ASSETS_ICNS -> $RES/AppIcon.icns"
  else
    echo "WARNING: $ASSETS_ICNS not found. App may not have an icon."
  fi
fi

echo "Setting required Info.plist keys..."

# CRITICAL: Set CFBundleExecutable to just the binary name (no path)
if /usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$PLIST" >/dev/null 2>&1; then
  /usr/libexec/PlistBuddy -c 'Set :CFBundleExecutable supro' "$PLIST"
else
  /usr/libexec/PlistBuddy -c 'Add :CFBundleExecutable string supro' "$PLIST"
fi

# Ensure CFBundleIdentifier exists
if ! /usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$PLIST" >/dev/null 2>&1; then
  /usr/libexec/PlistBuddy -c 'Add :CFBundleIdentifier string com.supro.app' "$PLIST"
fi

# Set icon name (no extension)
if /usr/libexec/PlistBuddy -c 'Print :CFBundleIconFile' "$PLIST" >/dev/null 2>&1; then
  /usr/libexec/PlistBuddy -c 'Set :CFBundleIconFile AppIcon' "$PLIST"
else
  /usr/libexec/PlistBuddy -c 'Add :CFBundleIconFile string AppIcon' "$PLIST"
fi

# Add camera permission
if /usr/libexec/PlistBuddy -c 'Print :NSCameraUsageDescription' "$PLIST" >/dev/null 2>&1; then
  /usr/libexec/PlistBuddy -c 'Set :NSCameraUsageDescription This app requires camera access to track arm rotation movements.' "$PLIST"
else
  /usr/libexec/PlistBuddy -c 'Add :NSCameraUsageDescription string This app requires camera access to track arm rotation movements.' "$PLIST"
fi

echo ""
echo "Verifying bundle structure..."
echo "Contents of MacOS directory:"
ls -la "$MACOS"
echo ""
echo "CFBundleExecutable value:"
/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$PLIST"
echo ""
echo "CFBundleIdentifier value:"
/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$PLIST"
echo ""

# Verify the executable exists
if [[ ! -f "$MACOS/supro" ]]; then
  echo "ERROR: Executable not found at $MACOS/supro"
  echo "Available files in MacOS directory:"
  ls -la "$MACOS"
  exit 1
fi

# Make sure executable has execute permissions
chmod +x "$MACOS/supro"

# Copy Python files
PYTHON_DIR="$RES/python"
mkdir -p "$PYTHON_DIR"
cp python/*.py "$PYTHON_DIR/"
echo "Copied Python scripts to bundle"

# Replace old app in /Applications
DEST="/Applications/SuPro.app"  # Changed from "Arm Tracker.app"
if [[ -d "$DEST" ]]; then
  echo "Removing old /Applications bundle..."
  rm -rf "$DEST"
fi

echo "Copying new bundle to /Applications..."
cp -R "$APP" "$DEST"

echo "Refreshing Launch Services cache..."
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -kill -r -domain local -domain system -domain user

echo "Opening app..."
open "$DEST"

echo "✓ Build complete!"