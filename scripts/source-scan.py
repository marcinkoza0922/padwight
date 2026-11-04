#!/usr/bin/env python3
"""Scan other stores for games that could get a pack: GOG, Epic and Battle.net (not Steam).

    python3 -I scripts/source-scan.py                      # table of unpacked, no-controller games sold off Steam
    python3 -I scripts/source-scan.py --online             # also ask GOG's product API for each GOG id
    python3 -I scripts/source-scan.py --all-controllers    # keep games whose pad support is 'true' too
    python3 -I scripts/source-scan.py --include-steam      # keep games that are also on Steam
    python3 -I scripts/source-scan.py --out pack-candidates/other-stores.tsv
    python3 -I scripts/source-scan.py --self-test

Candidates come from the PCGamingWiki snapshot (pcgamingwiki/games.tsv, plus stores from games.jsonl).
A game is a candidate when:
  * it has no pack (matched by pack-audit.py's name and Steam id rules, as store-check.py does);
  * the wiki says its controller support is false, hackable, limited or unknown (unless --all-controllers);
  * it is sold on GOG, Epic or Battle.net (and it is off Steam, unless --include-steam).
The output lists each store's ids, so a pack author can take the Steam-less executables from
find-executables.py (GOG depots) and the keys by hand. `--online` checks GOG ids against
api.gog.com, so a stale wiki id shows up as "not found" and the GOG title is shown for comparison.
Wiki data is community-written: treat it as unverified.
"""

import argparse
import csv
import importlib.util
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
NO_PAD = {"false", "hackable", "limited", "unknown", ""}
OFF_STEAM_FAMILIES = ("GOG", "Epic", "Battle.net")


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def helpers():
    """pack-audit (matcher, snapshot) and store-check (store families, GOG lookup)."""
    return load("pack_audit", ROOT / "scripts" / "pack-audit.py"), load("store_check", ROOT / "scripts" / "store-check.py")


def packed_files(pa, wiki):
    """Wiki article files already matched by some pack."""
    snap = pa.load_snapshot(wiki)
    done = set()
    for p in sorted((ROOT / "packs").glob("*.padpack")):
        _match, games, _f = pa.audit(pa.read_pack(p), snap)
        done |= set(games)
    return done


def candidates(wiki, args, pa, sc):
    """(wiki row, stores) for each candidate game."""
    done = packed_files(pa, wiki)
    rows = []
    with open(wiki / "games.tsv", newline="", encoding="utf-8") as fh:
        for r in csv.DictReader(fh, delimiter="\t", quoting=csv.QUOTE_NONE):
            if r["file"] in done:
                continue
            pad = (r["controller_support"] or "").strip().lower()
            if not args.all_controllers and pad not in NO_PAD:
                continue
            if not args.include_steam and r["steam_appids"].strip():
                continue
            rows.append(r)
    want = {r["file"] for r in rows}
    stores = {}
    with open(wiki / "games.jsonl", encoding="utf-8") as fh:
        for line in fh:
            if not any(f'"file": "{f}"' in line[:400] for f in want):
                continue
            rec = json.loads(line)
            if rec.get("file") in want:
                stores[rec["file"]] = sc.stores_of(rec)
    out = []
    for r in rows:
        st = stores.get(r["file"], {})
        if not any(f in st for f in OFF_STEAM_FAMILIES) and not r["gog_ids"].strip():
            continue  # only on Steam, or nowhere
        if not args.include_steam and "Steam" in st:
            continue
        out.append((r, st))
    return out


def run(args):
    pa, sc = helpers()
    wiki = pa.find_wiki(args.wiki)
    cache = {}
    lines = ["title\tyear\tcontroller\tfull_controller\tgog_ids\tgog_title\tepic\tbattle.net\tsteam\tfile"]
    shown = 0
    for r, st in sorted(candidates(wiki, args, pa, sc), key=lambda x: x[0]["title"].lower()):
        # numeric GOG ids only: the wiki also lists GOG URL slugs (ad_2044), which the API can't look up
        gog_ids = [g for g in (st.get("GOG", []) if st else []) + r["gog_ids"].split(",") if g.isdigit()]
        gog_ids = list(dict.fromkeys(gog_ids))
        gog_title = ""
        if args.online:
            titles = []
            for g in gog_ids:
                if g.isdigit():
                    t = sc.gog_lookup(g, cache)
                    titles.append(f"{g}={t or 'not found'}")
            gog_title = "; ".join(titles)
        lines.append("\t".join([
            r["title"], r["released"][-4:] if r["released"][-4:].isdigit() else "",
            r["controller_support"] or "-", r["full_controller"] or "-",
            ",".join(gog_ids) or "-", gog_title or "-",
            ",".join(st.get("Epic", [])) if st else "-",
            ",".join(st.get("Battle.net", [])) if st else "-",
            ",".join(st.get("Steam", [])) if st else "-",
            r["file"]]))
        shown += 1
    text = "\n".join(lines) + "\n"
    if args.out:
        Path(args.out).write_text(text, encoding="utf-8")
        print(f"wrote {shown} rows to {args.out}", file=sys.stderr)
    else:
        sys.stdout.write(text)
    print(f"{shown} candidates", file=sys.stderr)


def self_test():
    sc = load("store_check", ROOT / "scripts" / "store-check.py")
    assert sc.family("GOG.com") == "GOG" and sc.family("Epic games store") == "Epic"
    rec = {"stores": [{"store": "Battle.net", "id": "b"}], "gog_ids": ["7"], "steam_appids": []}
    got = sc.stores_of(rec)
    assert got == {"Battle.net": ["b"], "GOG": ["7"]}, got
    assert "pad" not in NO_PAD and "false" in NO_PAD and "true" not in NO_PAD
    print("ok")


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--wiki")
    ap.add_argument("--online", action="store_true", help="check GOG ids against api.gog.com")
    ap.add_argument("--all-controllers", action="store_true", help="keep games whose pad support is true")
    ap.add_argument("--include-steam", action="store_true", help="keep games that are also on Steam")
    ap.add_argument("--out", help="write the TSV here instead of stdout")
    ap.add_argument("--self-test", action="store_true")
    args = ap.parse_args()
    self_test() if args.self_test else run(args)


if __name__ == "__main__":
    main()
