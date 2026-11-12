#!/bin/sh
# Remove the bundle, the login item, and stop a running copy.
set -eu
DEST=${DEST:-$HOME/Applications}
APP="$DEST/Opus.app"
AGENT="$HOME/Library/LaunchAgents/dev.xproxy.opus.plist"

pkill -f "Opus.app/Contents/MacOS/opus" 2>/dev/null || true
rm -rf "$APP"
rm -f "$AGENT"
echo "removed $APP"
echo "the data folder is untouched: ~/Library/Application Support/dev.xproxy"
