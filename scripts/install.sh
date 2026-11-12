#!/bin/sh
# Install Opus from GitHub. No compiler, no clone.
#
#   curl -fsSL https://raw.githubusercontent.com/morganID/opus/main/scripts/install.sh | sh
#   curl -fsSL ... | sh -s -- --login     # also start it at login
#
# Downloads the latest release's bundle, drops it in ~/Applications, signs it
# ad-hoc and gets out of the way. `--from-source` builds instead, which needs
# Rust and two sibling checkouts; that path is for developing Opus, not for
# using it.
set -eu

REPO=${OPUS_REPO:-morganID/opus}
DEST=${DEST:-$HOME/Applications}
APP="$DEST/Opus.app"
LOGIN=0
SOURCE=0
for arg in "$@"; do
    case "$arg" in
        --login) LOGIN=1 ;;
        --from-source) SOURCE=1 ;;
        -h|--help)
            sed -n '2,10p' "$0" | sed 's/^# \{0,1\}//'
            exit 0 ;;
        *) echo "unknown argument: $arg" >&2; exit 2 ;;
    esac
done

if [ "$SOURCE" = 1 ]; then
    ROOT=$(cd "$(dirname "$0")/.." && pwd)
    PROFILE=${PROFILE:-release}
    case "$PROFILE" in
        release) FLAGS="--release" ;;
        debug)   FLAGS="" ;;
        *)       FLAGS="--profile $PROFILE" ;;
    esac
    TARGET=${CARGO_TARGET_DIR:-$ROOT/target}
    XPROXY_DIR=${XPROXY_DIR:-$ROOT/../xproxy}
    echo "→ building ($PROFILE)"
    ( cd "$ROOT" && cargo build $FLAGS )
    ( cd "$XPROXY_DIR" && cargo build $FLAGS -p xproxy --target-dir "$TARGET" )
    STAGE="$TARGET/$PROFILE/opus"
    STAGE_CLI="$TARGET/$PROFILE/xproxy"
else
    echo "→ looking up the latest release of $REPO"
    URL=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
        | sed -n 's/.*"browser_download_url": "\([^"]*Opus\.app\.tar\.gz\)".*/\1/p' | head -1)
    if [ -z "$URL" ]; then
        echo "no release found for $REPO" >&2
        echo "build it yourself with: $0 --from-source" >&2
        exit 1
    fi

    TMP=$(mktemp -d)
    trap 'rm -rf "$TMP"' EXIT
    echo "→ downloading $(basename "$URL")"
    curl -fsSL "$URL" -o "$TMP/Opus.app.tar.gz"
    tar -xzf "$TMP/Opus.app.tar.gz" -C "$TMP"
    STAGE="$TMP/Opus.app"
fi

echo "→ installing into $APP"
# Quit a running copy first: replacing a bundle under a live process leaves it
# running the old binary until the next launch.
pkill -f "Opus.app/Contents/MacOS/opus" 2>/dev/null || true
sleep 1
mkdir -p "$DEST"
rm -rf "$APP"
if [ "$SOURCE" = 1 ]; then
    mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
    cp "$STAGE" "$APP/Contents/MacOS/opus"
    cp "$STAGE_CLI" "$APP/Contents/MacOS/xproxy"
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
    <key>CFBundleShortVersionString</key><string>0.1.0</string>
    <key>CFBundleVersion</key>           <string>0.1.0</string>
    <key>LSMinimumSystemVersion</key>    <string>12.0</string>
    <key>LSUIElement</key>               <true/>
    <key>NSHighResolutionCapable</key>   <true/>
</dict>
</plist>
PLIST
else
    cp -R "$STAGE" "$APP"
fi

# Ad-hoc: enough for the system to treat the bundle as one application. A real
# Developer ID would also survive an update without a fresh keychain prompt.
codesign --force --sign - --timestamp=none "$APP" >/dev/null 2>&1 || \
    echo "  (codesign failed; the app still runs, unsigned)"

if [ "$LOGIN" = 1 ]; then
    AGENT="$HOME/Library/LaunchAgents/dev.xproxy.opus.plist"
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

# The CLI lives in the bundle; a symlink is what makes it reachable from a
# shell. First writable directory wins, so Homebrew users get it on PATH
# without editing anything.
CLI="$APP/Contents/MacOS/xproxy"
if [ -x "$CLI" ]; then
    for dir in /opt/homebrew/bin /usr/local/bin "$HOME/.local/bin"; do
        if [ -d "$dir" ] && [ -w "$dir" ]; then
            ln -sf "$CLI" "$dir/xproxy"
            echo "cli        $dir/xproxy"
            break
        fi
    done
fi

echo
echo "installed  $APP"
echo "run it     open \"$APP\""
[ "$LOGIN" = 1 ] && echo "at login   yes"
echo "update     re-run this script"
