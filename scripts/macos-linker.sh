#!/usr/bin/env bash
set -euo pipefail

# quad-snd 0.2.8 spells AudioToolbox with an uppercase B in its link directive.
args=()
for arg in "$@"; do
    if [[ "$arg" == AudioToolBox ]]; then
        arg=AudioToolbox
    fi
    args+=("$arg")
done

if [[ "$(uname -s)" == Darwin ]]; then
    exec xcrun clang "${args[@]}"
fi

exec "${BATTING_MACOS_CLANG:-clang}" --target=arm64-apple-macos11.0 -fuse-ld=lld \
    -isysroot "${SDKROOT:?Set SDKROOT to the macOS SDK}" "${args[@]}"
