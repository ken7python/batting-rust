#!/usr/bin/env bash
set -euo pipefail

sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT:?Set ANDROID_HOME or ANDROID_SDK_ROOT}}"
export PATH="$sdk/cmdline-tools/latest/bin:$sdk/emulator:$sdk/platform-tools:$PATH"

name=batting-ci
bundle_id=com.ken7python.batting.debug
printf 'no\n' | avdmanager create avd --force --name "$name" --package 'system-images;android-35;google_apis;x86_64'
emulator -avd "$name" -no-window -no-audio -no-snapshot -gpu swiftshader_indirect > target/android/emulator.log 2>&1 &
trap 'adb emu kill >/dev/null 2>&1 || true' EXIT
timeout 240 adb wait-for-device
for attempt in $(seq 1 120); do
    if [[ "$(adb shell getprop sys.boot_completed | tr -d '\r')" == 1 ]]; then
        break
    fi
    sleep 2
done
[[ "$(adb shell getprop sys.boot_completed | tr -d '\r')" == 1 ]]
adb shell input keyevent 82
adb install -r dist/android/batting-debug.apk
adb shell am start -W -n "$bundle_id/com.ken7python.batting.MainActivity"
sleep 3
adb shell pidof "$bundle_id"
adb exec-out screencap -p > dist/android/android.png
