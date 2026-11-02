#!/usr/bin/env python3
"""Steam's own launch executables for the library packs' Steam app ids -> steam-executables.tsv.

Complements Discord's list (discord-executables.py): Steam app info has the executable the
developer set in each app's `config.launch` entries, whether or not anyone's Discord ever saw it.

    python3 -I scripts/steam-executables.py                 # fetch what isn't cached, write the TSV
    python3 -I scripts/steam-executables.py --refresh       # fetch everything again
    python3 -I scripts/steam-executables.py --ids 870780,1145360   # these ids only (no packs)
    python3 -I scripts/steam-executables.py --self-test

Source: https://api.steamcmd.net/v1/info/<appid>, a third-party mirror of anonymous Steam app info
(one app per request, so a short pause between requests). Answers are cached as
pcgamingwiki/steam/<appid>.json. The output has executables.tsv's columns, so pack-audit.py reads
both; `discord_name` holds the Steam app name. Names are lowercased, with a folder kept when the
entry has one. Steam app info is data from the publisher: treat it as unverified.
"""

import argparse
import csv
import json
import sys
import time
import tomllib
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
URL = "https://api.steamcmd.net/v1/info/"
COLUMNS = ["executable", "os", "launcher", "arguments", "discord_name", "steam_appids",
           "other_skus", "pcgw_title", "pcgw_file"]
OS_NAMES = {"windows": "win32", "linux": "linux", "macos": "darwin"}


def find_wiki(arg):
    cands = [Path(arg)] if arg else [Path("pcgamingwiki"), ROOT / "pcgamingwiki"]
    if not arg:
        cands += [p / "pcgamingwiki" for p in ROOT.parents]
    for c in cands:
        if (c / "steam-appids.tsv").is_file():
            return c
    sys.exit("pcgamingwiki snapshot not found (see pcgamingwiki/README.md); pass --wiki DIR")


def pack_ids(packs):
    ids = set()
    for p in sorted(Path(packs).glob("*.padpack")):
        with open(p, "rb") as fh:
            for r in tomllib.load(fh).get("rules", []):
                if r.get("kind") == "steam_app_id" and str(r.get("value", "")).isdigit():
                    ids.add(str(r["value"]))
    return sorted(ids, key=int)


def fetch(appid, cache, refresh):
    path = cache / f"{appid}.json"
    if path.is_file() and not refresh:
        return json.loads(path.read_text(encoding="utf-8")), False
    req = urllib.request.Request(URL + appid, headers={"User-Agent": "padwight-pack-audit"})
    for attempt in range(3):
        try:
            with urllib.request.urlopen(req, timeout=30) as resp:
                body = json.load(resp)
            break
        except (urllib.error.URLError, TimeoutError, json.JSONDecodeError) as err:
            if attempt == 2:
                print(f"  {appid}: {err}", file=sys.stderr)
                return None, True
            time.sleep(2 * (attempt + 1))
    app = (body.get("data") or {}).get(appid)
    if body.get("status") != "success" or not app:
        return None, True
    slim = {"name": app.get("common", {}).get("name", ""),
            "launch": app.get("config", {}).get("launch", {})}
    path.write_text(json.dumps(slim, indent=1), encoding="utf-8")
    return slim, True


def rows_for(appid, info, titles):
    title, file = titles.get(appid, ("", ""))
    seen = set()
    for entry in (info.get("launch") or {}).values():
        exe = str(entry.get("executable", "")).replace("\\", "/").strip().lower()
        if not exe or exe.endswith((".app", ".sh")) and entry.get("config", {}).get("oslist") != "linux":
            continue
        os_name = OS_NAMES.get(entry.get("config", {}).get("oslist", "windows").split(",")[0], "win32")
        args = str(entry.get("arguments", ""))
        if (exe, os_name, args) in seen:
            continue
        seen.add((exe, os_name, args))
        yield [exe, os_name, "", args, info.get("name", ""), appid, "", title, file]


def write_tsv(path, rows):
    with open(path, "w", encoding="utf-8", newline="") as fh:
        fh.write("\t".join(COLUMNS) + "\n")
        for r in rows:
            fh.write("\t".join(str(v).replace("\t", " ").replace("\n", " ") for v in r) + "\n")


def self_test():
    info = {"name": "Alpha", "launch": {
        "0": {"executable": "Bin\\Alpha.EXE", "config": {"oslist": "windows"}},
        "1": {"executable": "Alpha.app", "config": {"oslist": "macos"}},
        "2": {"executable": "alpha.sh", "config": {"oslist": "linux"}},
        "3": {"executable": "Bin/alpha.exe", "config": {"oslist": "windows"}},
        "4": {"executable": "Alpha_VR.exe", "arguments": "-vr"}}}
    got = [r[:5] for r in rows_for("10", info, {"10": ("Alpha", "pages/a/Alpha.wiki")})]
    want = [["bin/alpha.exe", "win32", "", "", "Alpha"], ["alpha.sh", "linux", "", "", "Alpha"],
            ["alpha_vr.exe", "win32", "", "-vr", "Alpha"]]
    assert got == want, got
    print("self-test ok")


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--wiki")
    ap.add_argument("--packs", default=str(ROOT / "packs"))
    ap.add_argument("--ids", help="comma-separated app ids instead of the packs'")
    ap.add_argument("--refresh", action="store_true")
    ap.add_argument("--self-test", action="store_true")
    a = ap.parse_args()
    if a.self_test:
        return self_test()
    wiki = find_wiki(a.wiki)
    with open(wiki / "steam-appids.tsv", newline="", encoding="utf-8") as fh:
        titles = {r["steam_appid"]: (r["title"], r["file"])
                  for r in csv.DictReader(fh, delimiter="\t", quoting=csv.QUOTE_NONE)}
    ids = a.ids.split(",") if a.ids else pack_ids(a.packs)
    cache = wiki / "steam"
    cache.mkdir(exist_ok=True)
    rows, missing = [], []
    for i, appid in enumerate(ids, 1):
        info, fetched = fetch(appid, cache, a.refresh)
        if info is None:
            missing.append(appid)
        else:
            rows += list(rows_for(appid, info, titles))
        if fetched and i < len(ids):
            time.sleep(0.5)
    out = wiki / "steam-executables.tsv"
    write_tsv(out, rows)
    print(f"{len(rows)} executables from {len(ids) - len(missing)} of {len(ids)} apps -> {out}")
    if missing:
        print("no data for:", ",".join(missing))


if __name__ == "__main__":
    main()
