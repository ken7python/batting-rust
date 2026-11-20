#!/usr/bin/env bash
set -euo pipefail

export PATH="$PATH:$HOME/.cargo/bin"
cd "$(dirname "${BASH_SOURCE[0]}")/.."
case "${PLATFORM_NAME:?Run from the Batting Xcode target}" in
    iphonesimulator) target=aarch64-apple-ios-sim ;;
    iphoneos) target=aarch64-apple-ios ;;
    *) printf 'Unsupported SDK: %s\n' "$PLATFORM_NAME" >&2; exit 1 ;;
esac
# SDKROOT provided by Xcode can be a build setting name rather than a path.
export SDKROOT="$(xcrun --sdk "$PLATFORM_NAME" --show-sdk-path)"
cargo rustc --locked --lib --release --target "$target" --target-dir "$BUILD_DIR/batting-rust" --crate-type staticlib
install -m 644 "$BUILD_DIR/batting-rust/$target/release/libbatting.a" "$DERIVED_FILE_DIR/libbatting.a"
