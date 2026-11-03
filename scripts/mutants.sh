#!/bin/sh
# Mutation testing, one area at a time (T-19): changes the code in small ways (a `+` to `-`, a
# body to `Default::default()`) and reruns the tests; a mutant the tests don't catch is a gap.
# Settings are in .cargo/mutants.toml. Needs `cargo install cargo-mutants`.
#
# Usage: scripts/mutants.sh <engine|menu|gui-checks|config-store> [cargo-mutants args]
#   scripts/mutants.sh config-store --list     only list the mutants
# The results go to target/mutants-<area>/mutants.out; missed.txt there names the survivors.
# The engine sweep takes hours; CI runs it weekly (.github/workflows/weekly.yml).
set -eu
area=${1:?usage: scripts/mutants.sh engine|menu|gui-checks|config-store [args]}
shift
case $area in
engine) files="src/engine/**/*.rs" ;;
menu) files="src/menu/**/*.rs" ;;
gui-checks) files="src/gui/checks.rs src/gui/checks/**/*.rs" ;;
config-store) files="src/config/store.rs src/config/store/**/*.rs" ;;
*) echo "unknown area: $area" >&2; exit 2 ;;
esac
cd "$(dirname "$0")/.."
set --  "$@"
for f in $files; do set -- --file "$f" "$@"; done
exec cargo mutants --output "target/mutants-$area" "$@"
