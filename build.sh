#!/bin/bash
set -e

echo "Building bundle..."
cargo bundle --release

APP="target/release/bundle/osx/Arm Tracker.app"
PLIST="$APP/Contents/Info.plist"

echo "Adding camera permissions to Info.plist..."

# Add or update NSCameraUsageDescription
/usr/libexec/PlistBuddy -c 'Add :NSCameraUsageDescription string "This app requires camera access to track arm rotation movements for analysis."' "$PLIST" 2>/dev/null || \
/usr/libexec/PlistBuddy -c 'Set :NSCameraUsageDescription "This app requires camera access to track arm rotation movements for analysis."' "$PLIST"

# Add or update NSMicrophoneUsageDescription
/usr/libexec/PlistBuddy -c 'Add :NSMicrophoneUsageDescription string "Arm Tracker may record audio while capturing video."' "$PLIST" 2>/dev/null || \
/usr/libexec/PlistBuddy -c 'Set :NSMicrophoneUsageDescription "Arm Tracker may record audio while capturing video."' "$PLIST"

echo ""
echo "✓ Bundle created successfully"
echo ""
echo "Verifying permissions..."
echo "Camera: $(/usr/libexec/PlistBuddy -c 'Print :NSCameraUsageDescription' "$PLIST")"
echo "Microphone: $(/usr/libexec/PlistBuddy -c 'Print :NSMicrophoneUsageDescription' "$PLIST")"
echo ""

open "$APP"