#!/usr/bin/env python3
"""Compares two results folders of the profiling suite (scripts/profile.sh, PADWIGHT_PROFILE_OUT)
and flags each measurement that got worse by more than its noise.

Usage: python3 -I scripts/profile-compare.py [--json] [--all] old/ new/

Both folders hold the JSON files tests/profile writes (input_latency.json, resources.json, ...).
A measurement is flagged when new - old (in the bad direction) is larger than both its limit
and its noise:

  limit  latency (p50/p90/p99/mean, startup medians, draw times)  20% of the old value
         CPU                                                      0.5 points of one core
         memory (RSS)                                             10%, at least 2 MB
         threads, open files, missed samples, dropped events      any rise
         daemon binary size                                       5%
  noise  the spread (max - min) of the same measurement across the repeat runs the suite makes:
         per_session in input_latency.json, all_ms in the start-up files, rounds in the overlay
         styles file. Without repeats the noise is 0.

Idle CPU over 1% of a core is flagged whatever the old value was. Folders from different machines,
kernels or build kinds are not comparable; the script says so and stops. Latency on shared CI runners
is too noisy to compare: run this on the same machine each week.

Exit status: 0 nothing worse, 1 something flagged, 2 the folders can't be compared.
"""
import json
import sys
from pathlib import Path

LATENCY_KEYS = {"p50_us", "p90_us", "p99_us", "mean_us", "build_p50_us"}
CPU_BUDGET = 1.0  # percent of one core, idle
ENV_KEYS = ("cpu", "cores", "kernel", "container", "release_build")
SKIP = {"environment", "per_session", "samples_us", "all_ms", "runs", "rounds", "what", "history"}


class Rule:
    def __init__(self, kind, rel=0.0, floor=0.0, unit=""):
        self.kind, self.rel, self.floor, self.unit = kind, rel, floor, unit


def rule_for(key):
    """How to judge a number with this key, or None to ignore it."""
    if key in LATENCY_KEYS:
        return Rule("latency", 0.20, 0, "us")
    if key == "median_ms":
        return Rule("startup", 0.20, 5, "ms")
    if key == "cpu_percent" or key.endswith("_cpu_percent"):
        return Rule("cpu", 0, 0.5, "%")
    if key in ("rss_end_kb", "rss_peak_kb"):
        return Rule("memory", 0.10, 2048, "kB")
    if key in ("threads_peak", "fds_peak", "missed", "dropped_output_events"):
        return Rule("count", 0, 0, "")
    if key == "daemon_binary_bytes":
        return Rule("size", 0.05, 0, "B")
    return None


def walk(node, path=()):
    """Yields (path, value) for each judged number under node."""
    if not isinstance(node, dict):
        return
    for key, value in node.items():
        if key in SKIP or isinstance(value, bool):
            continue
        if isinstance(value, (int, float)):
            if rule_for(key):
                yield path + (key,), value
        else:
            yield from walk(value, path + (key,))


def noise_samples(doc, path):
    """The same measurement in each repeat run of doc, if the suite made repeats."""
    key = path[-1]
    node = doc
    for step in path[:-1]:
        node = node.get(step, {}) if isinstance(node, dict) else {}
    if not isinstance(node, dict):
        return []
    if key == "median_ms" and isinstance(node.get("all_ms"), list):
        return node["all_ms"]
    if isinstance(node.get("rounds"), list):
        return [r[key] for r in node["rounds"] if isinstance(r, dict) and isinstance(r.get(key), (int, float))]
    if len(path) >= 3 and path[-2] == "latency" and isinstance(doc.get("per_session"), list):
        out = [s.get(path[-3], {}).get(key) for s in doc["per_session"]]
        return [v for v in out if isinstance(v, (int, float))]
    return []


def spread(values):
    return max(values) - min(values) if len(values) > 1 else 0.0


def load(folder):
    docs = {}
    for file in sorted(Path(folder).glob("*.json")):
        try:
            docs[file.stem] = json.loads(file.read_text())
        except (OSError, ValueError) as err:
            print(f"warning: {file}: {err}", file=sys.stderr)
    return docs


def environment_mismatch(old, new):
    for name in sorted(old.keys() & new.keys()):
        a, b = old[name].get("environment", {}), new[name].get("environment", {})
        notes = [f"{key}: {a[key]!r} -> {b[key]!r}" for key in ENV_KEYS if key in a and key in b and a[key] != b[key]]
        if notes:
            return notes
    return []


def compare(old, new):
    rows = []
    for name in sorted(old.keys() & new.keys()):
        if "skipped" in old[name] or "skipped" in new[name]:
            continue
        before, after = dict(walk(old[name])), dict(walk(new[name]))
        for path in sorted(before.keys() & after.keys()):
            rule = rule_for(path[-1])
            a, b = before[path], after[path]
            noise = max(spread(noise_samples(old[name], path)), spread(noise_samples(new[name], path)))
            limit = max(rule.rel * abs(a), rule.floor)
            delta = b - a
            over = rule.kind == "cpu" and "idle" in path and b > CPU_BUDGET
            if delta > limit and delta > noise:
                status = "WORSE"
            elif over:
                status = "over 1% budget"
            elif -delta > limit and -delta > noise:
                status = "better"
            else:
                status = "same"
            rows.append((name + "." + ".".join(path), rule, a, b, noise, status))
    return rows


def fmt(value, rule):
    if rule.unit == "kB":
        return f"{value / 1024:.1f} MB"
    if rule.unit == "B":
        return f"{value / 1e6:.2f} MB"
    if rule.unit == "%":
        return f"{value:.2f}%"
    return f"{value:.1f} {rule.unit}".strip()


def main(argv):
    args = [a for a in argv if not a.startswith("--")]
    flags = {a for a in argv if a.startswith("--")}
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    old, new = load(args[0]), load(args[1])
    if not (old.keys() & new.keys()):
        print("no result file in both folders", file=sys.stderr)
        return 2
    mismatch = environment_mismatch(old, new)
    if mismatch:
        print("The runs are not comparable (" + "; ".join(mismatch) + "). Compare runs from the same machine.", file=sys.stderr)
        return 2
    rows = compare(old, new)
    flagged = [r for r in rows if r[5] in ("WORSE", "over 1% budget")]
    if "--json" in flags:
        print(json.dumps([{"metric": r[0], "old": r[2], "new": r[3], "noise": r[4], "status": r[5]} for r in rows], indent=2))
    else:
        for metric, rule, a, b, noise, status in rows:
            if "--all" in flags or status != "same":
                print(f"{status:<15} {metric}: {fmt(a, rule)} -> {fmt(b, rule)} (noise {fmt(noise, rule)})")
        better = sum(1 for r in rows if r[5] == "better")
        print(f"{len(rows)} measurements, {len(flagged)} worse, {better} better")
    return 1 if flagged else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
