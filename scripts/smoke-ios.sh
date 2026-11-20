#!/usr/bin/env bash
set -euo pipefail

app=dist/ios/sim/Batting.app
bundle_id=com.ken7python.batting.ios
for family in iPhone iPad; do
    started=false
    udid="$(xcrun simctl list devices available --json | jq -r --arg family "$family" '[.devices[][] | select(.state == "Booted" and (.name | startswith($family)))][0].udid // empty')"
    if [[ -z "$udid" ]]; then
        udid="$(xcrun simctl list devices available --json | jq -r --arg family "$family" '[.devices[][] | select(.name | startswith($family))][0].udid // empty')"
        test -n "$udid"
        xcrun simctl boot "$udid"
        started=true
    fi
    xcrun simctl bootstatus "$udid" -b
    xcrun simctl install "$udid" "$app"
    launch="$(xcrun simctl launch "$udid" "$bundle_id")"
    pid="${launch##*: }"
    sleep 3
    kill -0 "$pid"
    xcrun simctl io "$udid" screenshot "dist/ios/$family.png"
    # Scene adoption must also forward pause/resume to Miniquad. The same
    # process must survive a trip to the background and return to the game.
    xcrun simctl launch "$udid" com.apple.Preferences
    sleep 1
    resumed="$(xcrun simctl launch "$udid" "$bundle_id")"
    test "${resumed##*: }" = "$pid"
    sleep 3
    kill -0 "$pid"
    xcrun simctl io "$udid" screenshot "dist/ios/$family-resumed.png"
    if $started; then
        xcrun simctl shutdown "$udid"
    fi
done
