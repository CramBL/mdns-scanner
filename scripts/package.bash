#!/usr/bin/env bash

set -euxo pipefail

BIN_NAME="mdns-scanner"

VERSION=${REF#"refs/tags/v"}

if [[ $VERSION == "refs/heads/trunk" ]]; then
    VERSION="trunk"
fi

DIST=$(pwd)/dist

echo "Packaging ${BIN_NAME} $VERSION for $TARGET..."

test -f Cargo.lock || cargo generate-lockfile

echo "Building ${BIN_NAME}..."

if [[ $TARGET == aarch64-unknown-linux-musl ]]; then
    export CC=aarch64-linux-gnu-gcc
elif [[ $TARGET == arm-unknown-linux-musleabihf ]]; then
    export CC=arm-linux-gnueabihf-gcc
fi

RUSTFLAGS="--codegen target-feature=+crt-static $TARGET_RUSTFLAGS" \
    cargo build --bin ${BIN_NAME} --target $TARGET --release

EXECUTABLE=target/$TARGET/release/${BIN_NAME}

if [[ $OS == windows-latest ]]; then
    EXECUTABLE=$EXECUTABLE.exe
fi

echo "Copying release files..."
mkdir dist
cp -r \
    "$EXECUTABLE" \
    Cargo.lock \
    Cargo.toml \
    README.md \
    CHANGELOG.md \
    "$DIST"

cd "$DIST"
echo "Creating release archive..."
case $OS in
    ubuntu-latest | macos-latest)
        ARCHIVE=${BIN_NAME}-$VERSION-$TARGET.tar.gz
        tar czf "$ARCHIVE" *
        echo "archive=$DIST/$ARCHIVE" >> $GITHUB_OUTPUT
        ;;
    windows-latest)
        ARCHIVE=${BIN_NAME}-$VERSION-$TARGET.zip
        7z a $ARCHIVE *
        echo "archive=$(pwd -W)/$ARCHIVE" >> $GITHUB_OUTPUT
        ;;
esac