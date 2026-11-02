#!/bin/sh
# Runs the fuzz targets (fuzz/): `scripts/fuzz.sh [target [seconds]]`. With no target it runs each
# in turn, `seconds` (default 60) apiece. Targets: pack_parse pack_assets config_load ipc_request
# tokens calculator engine_input.
#
# The seed corpus (library packs, built-in assets, debug-scene text, a config folder or two) is
# made on the first run; the corpus and any crash files stay in fuzz/corpus and fuzz/artifacts
# (not in git). A crash is saved as fuzz/artifacts/<target>/crash-*: replay it with
# `cargo +nightly fuzz run <target> fuzz/artifacts/<target>/crash-...`. Needs cargo-fuzz and a
# nightly toolchain. FUZZ_JOBS caps the compile jobs and the fuzzing workers (default 2).
# Exits non-zero if any target crashed, hung or ran out of memory.
set -eu

cd "$(dirname "$0")/.."
targets="pack_parse pack_assets config_load ipc_request tokens calculator engine_input"
only=${1:-}
seconds=${2:-60}
jobs=${FUZZ_JOBS:-2}
export CARGO_BUILD_JOBS=$jobs
export PADWIGHT_FUZZ_WORK="$PWD/fuzz/work"
mkdir -p "$PADWIGHT_FUZZ_WORK"

if [ -n "$only" ]; then
    case " $targets " in *" $only "*) targets=$only ;; *) echo "unknown target $only (one of: $targets)" >&2; exit 2 ;; esac
fi

[ -d fuzz/corpus/pack_parse ] || python3 -I scripts/fuzz-seeds.py fuzz
if [ ! -d fuzz/corpus/config_load ]; then
    mkdir -p fuzz/corpus/config_load
    PADWIGHT_FUZZ_SEEDS="$PWD/fuzz/corpus/config_load" cargo +nightly fuzz run -s none config_load -- -runs=0 >/dev/null 2>&1 || true
fi

status=0
for target in $targets; do
    echo "== $target (${seconds}s)"
    # -s none: no sanitizer, for a build that takes minutes instead of tens of them; the targets
    # look for panics, hangs and the engine's own assertions. -rss_limit_mb catches runaway memory.
    cargo +nightly fuzz run -s none "$target" -- -max_total_time="$seconds" -timeout=10 -rss_limit_mb=4096 -workers="$jobs" -jobs="$jobs" || status=1
done
exit $status
