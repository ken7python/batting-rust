#!/usr/bin/env bash
set -euo pipefail

app=dist/ios/sim/Batting.app
bundle_id=com.ken7python.batting.ios
for family in iPhone iPad; do
    udid="$(xcrun simctl list devices booted available --json | jq -r --arg family "$family" '[.devices[][] | select(.name | startswith($family))][0].udid // empty')"
    if [[ -z "$udid" ]]; then
        udid="$(xcrun simctl list devices available --json | jq -r --arg family "$family" '[.devices[][] | select(.name | startswith($family))][0].udid // empty')"
        test -n "$udid"
        xcrun simctl boot "$udid"
    fi
    xcrun simctl bootstatus "$udid" -b
    xcrun simctl install "$udid" "$app"
    launch="$(xcrun simctl launch "$udid" "$bundle_id")"
    pid="${launch##*: }"
    sleep 3
    kill -0 "$pid"
    xcrun simctl io "$udid" screenshot "dist/ios/$family.png"
    xcrun simctl shutdown "$udid"
done
