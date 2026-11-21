#!/usr/bin/env bash
set -euo pipefail

sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT:?Set ANDROID_HOME or ANDROID_SDK_ROOT}}"
export PATH="$sdk/cmdline-tools/latest/bin:$sdk/emulator:$sdk/platform-tools:$PATH"

name=batting-ci
bundle_id=com.ken7python.batting.debug
# Both Android tools must resolve the same AVD metadata on the hosted runner.
export ANDROID_USER_HOME="$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/batting-android.XXXXXX")"
export ANDROID_AVD_HOME="$ANDROID_USER_HOME/avd"
mkdir -p "$ANDROID_AVD_HOME"
printf 'no\n' | avdmanager create avd --force --name "$name" \
    --path "$ANDROID_AVD_HOME/$name.avd" --package 'system-images;android-35;google_apis;x86_64'
test -s "$ANDROID_AVD_HOME/$name.ini"
emulator -list-avds | grep -Fx "$name"
mkdir -p target/android
emulator -avd "$name" -no-window -no-audio -no-snapshot -no-boot-anim -port 5554 \
    -gpu swiftshader -feature -Vulkan > target/android/emulator.log 2>&1 &
emulator_pid=$!
cleanup() {
    local status=$?
    if (( status != 0 )); then
        tail -100 target/android/emulator.log >&2
    fi
    kill "$emulator_pid" 2>/dev/null || true
}
trap cleanup EXIT

for attempt in $(seq 1 150); do
    if ! kill -0 "$emulator_pid" 2>/dev/null; then
        echo 'Android emulator exited before boot completed' >&2
        exit 1
    fi
    if [[ "$(timeout 5 adb -e shell getprop sys.boot_completed 2>/dev/null | tr -d '\r' || true)" == 1 ]]; then
        break
    fi
    sleep 2
done
[[ "$(timeout 5 adb -e shell getprop sys.boot_completed | tr -d '\r')" == 1 ]]
export ANDROID_SERIAL=emulator-5554
adb shell input keyevent 82
adb install -r dist/android/batting-debug.apk
adb shell am start -W -n "$bundle_id/com.ken7python.batting.MainActivity"
sleep 3
adb shell pidof "$bundle_id"
adb exec-out screencap -p > dist/android/android.png
