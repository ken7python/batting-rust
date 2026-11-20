#!/usr/bin/env bash
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo"
sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"
if [[ -z "$sdk" ]]; then
    printf '%s\n' 'Set ANDROID_HOME to the Android SDK directory.' >&2
    exit 1
fi
ndk="${ANDROID_NDK_HOME:-$sdk/ndk/30.0.16248370}"
tools="$sdk/build-tools/35.0.0"
export ANDROID_HOME="$sdk"
if [[ ! -d "$ndk" || ! -x "$tools/apksigner" || ! -x "$tools/zipalign" ]]; then
    printf '%s\n' 'Install Android NDK 30.0.16248370 and SDK Build Tools 35.0.0 with sdkmanager.' >&2
    exit 1
fi
case "$(uname -s)" in
    Darwin)
        bin="$ndk/toolchains/llvm/prebuilt/darwin-arm64/bin"
        [[ -d "$bin" ]] || bin="$ndk/toolchains/llvm/prebuilt/darwin-x86_64/bin"
        ;;
    Linux) bin="$ndk/toolchains/llvm/prebuilt/linux-x86_64/bin" ;;
    *) printf '%s\n' 'Use macOS or x64 Linux for the Android build script.' >&2; exit 1 ;;
esac
if [[ ! -x "$bin/llvm-nm" ]]; then
    printf '%s\n' 'The Android NDK toolchain for this host was not found.' >&2
    exit 1
fi
workspace="$repo/target/android"
generated="$workspace/generated"
version="$(cargo metadata --locked --format-version 1 --no-deps |
    python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "batting"))')"

build_abi() {
    local target="$1" abi="$2" linker_var="$3" library symbols
    local linker="$bin/${target}26-clang"
    env "$linker_var=$linker" cargo rustc --locked --release --lib --target "$target" \
        --target-dir "$workspace/rust" --crate-type cdylib -- -Clink-arg=-Wl,-z,max-page-size=16384
    library="$workspace/rust/$target/release/libbatting.so"
    symbols="$("$bin/llvm-nm" --dynamic --defined-only "$library" --format=posix)"
    for symbol in JNI_OnLoad Java_quad_1native_QuadNative_activityOnCreate Java_quad_1native_QuadNative_surfaceOnTouch; do
        if ! grep -q "^$symbol " <<< "$symbols"; then
            printf 'Missing Android entry point %s in %s\n' "$symbol" "$library" >&2
            exit 1
        fi
    done
    python3 - "$library" <<'PY'
import pathlib, struct, sys
data = pathlib.Path(sys.argv[1]).read_bytes()
assert data[:6] == b"\x7fELF\x02\x01", "expected a 64-bit little-endian ELF"
table = struct.unpack_from("<Q", data, 32)[0]
size, count = struct.unpack_from("<HH", data, 54)
segments = [struct.unpack_from("<II6Q", data, table + i * size) for i in range(count)]
assert any(segment[0] == 1 for segment in segments)
assert all(segment[-1] >= 16384 and (segment[2] - segment[3]) % 16384 == 0
           for segment in segments if segment[0] == 1), "ELF load segments must support 16 KB pages"
PY
    mkdir -p "$generated/jniLibs/$abi"
    install -m 644 "$library" "$generated/jniLibs/$abi/libbatting.so"
}

build_abi aarch64-linux-android arm64-v8a CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER
build_abi x86_64-linux-android x86_64 CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER

cargo metadata --locked --format-version 1 --filter-platform aarch64-linux-android |
    python3 -c '
import json, pathlib, sys
data = json.load(sys.stdin)
packages = {package["id"]: package for package in data["packages"]}
nodes = {node["id"]: node for node in data["resolve"]["nodes"]}
root = nodes[data["resolve"]["root"]]
macroquad = nodes[next(dep["pkg"] for dep in root["deps"] if dep["name"] == "macroquad")]
miniquad = packages[next(dep["pkg"] for dep in macroquad["deps"] if dep["name"] == "miniquad")]
source = pathlib.Path(miniquad["manifest_path"]).parent
generated = pathlib.Path(sys.argv[1])
activity = (source / "java/MainActivity.java").read_text(encoding="utf-8")
activity = activity.replace("TARGET_PACKAGE_NAME", "com.ken7python.batting").replace("LIBRARY_NAME", "batting")
activity = activity.replace("getInsets(WindowInsets.Type.systemBars())",
                            "getInsets(WindowInsets.Type.systemBars() | WindowInsets.Type.displayCutout())")
java = generated / "java"
activity_file = java / "com/ken7python/batting/MainActivity.java"
activity_file.parent.mkdir(parents=True, exist_ok=True)
activity_file.write_text(activity, encoding="utf-8")
native_file = java / "quad_native/QuadNative.java"
native_file.parent.mkdir(parents=True, exist_ok=True)
native_file.write_bytes((source / "java/QuadNative.java").read_bytes())
license_dir = generated / "assets/licenses"
license_dir.mkdir(parents=True, exist_ok=True)
for name in ("LICENSE-MIT", "LICENSE-APACHE"):
    (license_dir / ("miniquad-" + name)).write_bytes((source / name).read_bytes())
' "$generated"

mkdir -p "$generated/res/drawable-nodpi" "$repo/dist/android"
install -m 644 web/apple-touch-icon.png "$generated/res/drawable-nodpi/batting_icon.png"
install -m 644 assets/OFL-NotoSansJP.txt "$generated/assets/licenses/"
"${BATTING_GRADLE:-gradle}" -p "$repo/android" --no-daemon ":app:assembleDebug" "-PbattingVersion=$version"
apk="$workspace/gradle/app/outputs/apk/debug/app-debug.apk"
"$tools/apksigner" verify "$apk"
"$tools/zipalign" -c -P 16 4 "$apk"
install -m 644 "$apk" "$repo/dist/android/batting-debug.apk"
printf '%s\n' "$repo/dist/android/batting-debug.apk"
