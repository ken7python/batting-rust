#!/usr/bin/env bash
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$repo"

build_web() {
    local target=wasm32-unknown-unknown
    local metadata target_dir miniquad quad_snd output
    cargo build --release --locked --target "$target"
    metadata="$(cargo metadata --locked --format-version 1 --filter-platform "$target")"
    target_dir="$(read_metadata target_directory <<< "$metadata")"
    miniquad="$(read_metadata miniquad <<< "$metadata")"
    quad_snd="$(read_metadata quad-snd <<< "$metadata")"

    output="$repo/dist/web"
    mkdir -p "$output/licenses"
    install -m 644 web/index.html "$output/index.html"
    install -m 644 "$target_dir/$target/release/batting.wasm" "$output/batting.wasm"
    # Only the plugins used by the game are needed in the loader.
    cat "$miniquad/js/gl.js" "$quad_snd/js/audio.js" > "$output/mq_js_bundle.js"
    install -m 644 "$miniquad/LICENSE-MIT" "$output/licenses/miniquad-MIT.txt"
    install -m 644 "$miniquad/LICENSE-APACHE" "$output/licenses/Apache-2.0.txt"
    install -m 644 assets/OFL-NotoSansJP.txt "$output/licenses/OFL-NotoSansJP.txt"
    printf '%s\n' "$output/index.html"
}

build_pages() {
    build_web
    mkdir -p play/licenses
    install -m 644 dist/web/batting.wasm dist/web/mq_js_bundle.js play/
    install -m 644 dist/web/licenses/*.txt play/licenses/
    touch .nojekyll
    python3 - <<'PY'
from hashlib import sha256
from pathlib import Path

html = Path("dist/web/index.html").read_text(encoding="utf-8")
html = html.replace("<head>", '<head>\n  <base href="./play/">', 1)
for filename in ("batting.wasm", "mq_js_bundle.js"):
    revision = sha256((Path("play") / filename).read_bytes()).hexdigest()[:12]
    html = html.replace(f'"./{filename}"', f'"./{filename}?v={revision}"')
Path("index.html").write_text(html, encoding="utf-8")
PY
}

build_macos() {
    local target=aarch64-apple-darwin
    local target_dir bundle
    if [[ "$(uname -s)" != Darwin && -z "${SDKROOT:-}" && -z "${CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER:-}" ]]; then
        printf '%s\n' 'macOS cross-build needs SDKROOT or a configured Cargo linker.' >&2
        return 1
    fi
    CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER="${CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER:-$repo/scripts/macos-linker.sh}" \
        cargo build --release --locked --target "$target"
    target_dir="$(cargo metadata --locked --format-version 1 --no-deps | read_metadata target_directory)"
    bundle="$repo/dist/macos/Batting.app"
    mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources"
    install -m 644 macos/Info.plist "$bundle/Contents/Info.plist"
    install -m 755 "$target_dir/$target/release/batting" "$bundle/Contents/MacOS/batting"
    install -m 644 assets/OFL-NotoSansJP.txt "$bundle/Contents/Resources/OFL-NotoSansJP.txt"
    if [[ "$(uname -s)" == Darwin ]]; then
        codesign --force --sign - "$bundle"
    fi
    printf '%s\n' "$bundle"
}

read_metadata() {
    python3 -c '
import json
import pathlib
import sys
data = json.load(sys.stdin)
key = sys.argv[1]
if key == "target_directory":
    print(data[key])
else:
    packages = {package["id"]: package for package in data["packages"]}
    project = next(node for node in data["resolve"]["nodes"] if node["id"] == data["resolve"]["root"])
    direct = packages[next(dep["pkg"] for dep in project["deps"] if dep["name"] == "macroquad")]
    macroquad = next(node for node in data["resolve"]["nodes"] if node["id"] == direct["id"])
    package = packages[next(dep["pkg"] for dep in macroquad["deps"] if dep["name"] == key.replace("-", "_"))]
    print(pathlib.Path(package["manifest_path"]).parent)
' "$1"
}

usage() {
    printf 'Usage: %s [all|web|macos|pages]\n' "$0"
    printf '%s\n' 'Default: build both Web and macOS.'
}

if (( $# > 1 )); then
    usage >&2
    exit 2
fi

case "${1:-all}" in
    web) build_web ;;
    pages) build_pages ;;
    macos) build_macos ;;
    all) build_web; build_macos ;;
    -h|--help) usage ;;
    *) usage >&2; exit 2 ;;
esac
