#!/usr/bin/env python3
"""Writes the results of scripts/desktop-weekly.sh into the desktop table of qa-plan.md.

Usage: python3 -I scripts/update-desktop-table.py docs/development/qa-plan.md results.tsv

results.tsv has one line per desktop: VM kind, exit status of desktop-test.sh (the number of
failed checks), kernel, date, separated by tabs. In the "Desktop session runs" table the row of
each desktop gets:
  pass  the date and kernel of its "all pass" note replaced (or added), any earlier FAILED
        prefix removed
  fail  "FAILED date, kernel, n check(s) failed (log in target/desktop-weekly)." put in front of
        the old result, which stays as the last known good one
"Last full run <date>, kernel <k>" above the table follows when every desktop passed. The
Version column is left alone: the versions are in each run's log.
"""
import re
import sys
from pathlib import Path

ROWS = {
    "gnome": "GNOME",
    "cinnamon": "Cinnamon",
    "cosmic": "COSMIC",
    "budgie": "Budgie (Wayland)",
    "ubuntu-budgie": "Ubuntu Budgie",
    "niri": "niri",
    "kde": "KDE Plasma",
    "sway": "Sway",
    "hyprland": "Hyprland",
    "labwc": "labwc",
}
FAILED = re.compile(r"^FAILED .*? check\(s\) failed\.\s*")
PASS = re.compile(r"all pass(?: \(\d{4}-\d{2}-\d{2}(?:, kernel [^)\s;]+)?)?")


def update_cell(cell, ok, failures, kernel, date):
    cell = FAILED.sub("", cell.strip())
    if not ok:
        return f"FAILED {date}, kernel {kernel}, {failures} check(s) failed. {cell}"
    stamp = f"all pass ({date}, kernel {kernel}"
    match = PASS.search(cell)
    if match:
        # A bare "all pass" needs its parenthesis closed; a dated one already has it further on.
        return PASS.sub(stamp + ("" if "(" in match.group(0) else ")"), cell, count=1)
    return f"{stamp}); {cell}" if cell else f"{stamp})"


def main(plan, results):
    lines = Path(plan).read_text().split("\n")
    try:
        start = next(i for i, line in enumerate(lines) if line.startswith("## 8. Desktop session runs"))
    except StopIteration:
        print("no section 8 in", plan, file=sys.stderr)
        return 2
    end = next((i for i in range(start + 1, len(lines)) if lines[i].startswith("## ")), len(lines))
    outcomes = {}
    for line in Path(results).read_text().splitlines():
        if line.strip():
            kind, status, kernel, date = line.split("\t")
            outcomes[kind] = (int(status), kernel, date)
    done = set()
    for i in range(start, end):
        cells = lines[i].split("|")
        if len(cells) < 5 or not lines[i].startswith("|"):
            continue
        label = cells[1].strip()
        for kind, prefix in ROWS.items():
            if kind in outcomes and label.startswith(prefix):
                status, kernel, date = outcomes[kind]
                cells[3] = " " + update_cell(cells[3], status == 0, status, kernel, date) + " "
                lines[i] = "|".join(cells)
                done.add(kind)
    for kind in sorted(set(outcomes) - done):
        print(f"warning: no table row for {kind}", file=sys.stderr)
    if outcomes and all(status == 0 for status, _, _ in outcomes.values()) and len(done) == len(outcomes):
        date = max(date for _, _, date in outcomes.values())
        kernel = sorted({k for _, k, _ in outcomes.values()})[-1]
        for i in range(start, end):
            if lines[i].startswith("Last full run"):
                lines[i] = re.sub(r"^Last full run \d{4}-\d{2}-\d{2}, kernel [^;:]+", f"Last full run {date}, kernel {kernel}", lines[i])
    Path(plan).write_text("\n".join(lines))
    print(f"updated {len(done)} row(s) in {plan}")
    return 0


if __name__ == "__main__":
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        sys.exit(2)
    sys.exit(main(*sys.argv[1:]))
