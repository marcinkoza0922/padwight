#!/bin/sh
# Raw screenshots, then the annotated pictures in docs/images/manual/.
cd "$(dirname "$0")/../.." || exit 1
scripts/manual-shots/shoot.sh || exit 1
for spec in scripts/manual-shots/specs/*.json; do
    scripts/annotate.py "$spec" >/dev/null || exit 1
done
