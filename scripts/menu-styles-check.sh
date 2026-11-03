#!/usr/bin/env bash
# OVL-3 as pass or fail: opens and closes every menu style many times on a headless KWin, holds
# one open for a minute, and judges flicker, stale frames, unfinished fades and CPU against limits
# (scripts/harness/menu-styles-check.py). Needs what scripts/overlay-harness.sh needs.
#   scripts/menu-styles-check.sh [out dir]        (default: screenshots/menu-styles)
#   PADWIGHT_HARNESS_RELEASE=1                    the optimized build, for the CPU limits' sake
#   PADWIGHT_CYCLES=4 PADWIGHT_HOLD_SECONDS=60   the length of the run
# Extra arguments for the checker (limits) go in MENU_STYLES_LIMITS, e.g. "--still-cpu 10".
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
out=${1:-$root/screenshots/menu-styles}
"$root/scripts/overlay-harness.sh" "$root/scripts/harness/menu-styles.sh" "$out"
# shellcheck disable=SC2086
python3 -I "$root/scripts/harness/menu-styles-check.py" "$out" ${MENU_STYLES_LIMITS:-}
