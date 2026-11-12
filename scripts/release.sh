#!/bin/sh
# Cut a release: build, bundle, and publish the bundle as a GitHub asset.
#
#   ./scripts/release.sh            # tag v<version from Cargo.toml>
#   ./scripts/release.sh v0.2.0     # or name it yourself
#
# The asset is `Opus.app.tar.gz`, which is what install.sh downloads. It is a
# bundle rather than a bare binary because a menu bar app needs LSUIElement to
# stay out of the Dock, and because a bundle is what macOS signs as one thing.
set -eu

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"

VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
TAG=${1:-v$VERSION}
ARCH=$(uname -m)
TARGET=${CARGO_TARGET_DIR:-$ROOT/target}

echo "→ building release for $ARCH"
cargo build --release

echo "→ assembling the bundle"
STAGE=$(mktemp -d)
trap 'rm -rf "$STAGE"' EXIT
APP="$STAGE/Opus.app"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$TARGET/release/opus" "$APP/Contents/MacOS/opus"

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
    <key>CFBundleShortVersionString</key><string>$VERSION</string>
    <key>CFBundleVersion</key>           <string>$VERSION</string>
    <key>LSMinimumSystemVersion</key>    <string>12.0</string>
    <key>LSUIElement</key>               <true/>
    <key>NSHighResolutionCapable</key>   <true/>
</dict>
</plist>
PLIST

codesign --force --sign - --timestamp=none "$APP" >/dev/null 2>&1 || \
    echo "  (codesign failed; the bundle is unsigned)"

mkdir -p "$ROOT/dist"
ASSET="$ROOT/dist/Opus.app.tar.gz"
echo "→ packing $(basename "$ASSET")"
( cd "$STAGE" && tar -czf "$ASSET" Opus.app )
ls -lh "$ASSET" | awk '{print "  " $5, $9}'

echo "→ publishing $TAG"
NOTES="Built for $ARCH from $(git rev-parse --short HEAD 2>/dev/null || echo 'a working tree').

Install:

    curl -fsSL https://raw.githubusercontent.com/morganID/opus/main/scripts/install.sh | sh

This bundle is ad-hoc signed, not notarised: macOS will ask for keychain
access on first launch, and again after every update."
gh release create "$TAG" "$ASSET" \
    --title "Opus $VERSION" \
    --notes "$NOTES"

echo
echo "done       https://github.com/morganID/opus/releases/tag/$TAG"
echo "install    curl -fsSL https://raw.githubusercontent.com/morganID/opus/main/scripts/install.sh | sh"
