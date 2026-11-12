#!/bin/sh
# Build Opus, wrap it in an application bundle, and put it where applications go.
#
# A menu bar extra does not have to be a bundle — a bare binary runs fine — but
# a bundle is what makes it a *thing*: an icon in ~/Applications, a login item,
# a name in the notification settings, and a Dock-less launch that survives a
# reboot. The three lines that matter are LSUIElement (no Dock tile), the binary
# inside Contents/MacOS, and an ad-hoc signature so macOS stops treating it as
# an unknown process every time it starts.
#
#   ./scripts/install.sh              # build, bundle, install
#   ./scripts/install.sh --login      # also start it at login
#   DEST=/Applications ./scripts/install.sh
#   PROFILE=debug ./scripts/install.sh   # skip the optimiser while iterating
#
# `CARGO_TARGET_DIR` is honoured, so a shared build directory (the engine's,
# for instance) is reused instead of compiled twice.
set -eu

ROOT=$(cd "$(dirname "$0")/.." && pwd)
DEST=${DEST:-$HOME/Applications}
APP="$DEST/Opus.app"
LOGIN=0
for arg in "$@"; do
    case "$arg" in
        --login) LOGIN=1 ;;
        *) echo "unknown argument: $arg" >&2; exit 2 ;;
    esac
done

PROFILE=${PROFILE:-release}
case "$PROFILE" in
    release) FLAGS="--release" ;;
    debug)   FLAGS="" ;;
    *)       FLAGS="--profile $PROFILE" ;;
esac
TARGET=${CARGO_TARGET_DIR:-$ROOT/target}

echo "→ building ($PROFILE; the first build compiles the engine, so give it minutes)"
cd "$ROOT"
cargo build $FLAGS

echo "→ assembling $APP"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$TARGET/$PROFILE/opus" "$APP/Contents/MacOS/opus"

VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>              <string>Opus</string>
    <key>CFBundleDisplayName</key>       <string>Opus</string>
    <key>CFBundleIdentifier</key>        <string>dev.xproxy.opus</string>
    <key>CFBundleExecutable</key>        <string>opus</string>
    <key>CFBundlePackageType</key>       <string>APPL</string>
    <key>CFBundleShortVersionString</key><string>${VERSION:-0.1.0}</string>
    <key>CFBundleVersion</key>           <string>${VERSION:-0.1.0}</string>
    <key>LSMinimumSystemVersion</key>    <string>12.0</string>
    <!-- A menu bar extra: no Dock tile, no application menu. -->
    <key>LSUIElement</key>               <true/>
    <key>NSHighResolutionCapable</key>   <true/>
</dict>
</plist>
PLIST

echo "→ signing (ad-hoc)"
# No Developer ID here, so the signature is ad-hoc: enough for the keychain to
# remember the app between launches, but every rebuild changes the hash and
# macOS asks once more. A real identity would stop that.
codesign --force --sign - --timestamp=none "$APP" >/dev/null 2>&1 || \
    echo "  (codesign failed; the app still runs, unsigned)"

if [ "$LOGIN" = 1 ]; then
    AGENT="$HOME/Library/LaunchAgents/dev.xproxy.opus.plist"
    echo "→ starting at login ($AGENT)"
    mkdir -p "$HOME/Library/LaunchAgents"
    cat > "$AGENT" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>            <string>dev.xproxy.opus</string>
    <key>ProgramArguments</key> <array><string>$APP/Contents/MacOS/opus</string></array>
    <key>RunAtLoad</key>        <true/>
    <key>KeepAlive</key>        <false/>
</dict>
</plist>
PLIST
fi

echo
echo "installed  $APP"
echo "run it     open \"$APP\""
[ "$LOGIN" = 1 ] && echo "at login   yes"
echo "update     git pull && ./scripts/install.sh"
