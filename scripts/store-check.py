#!/usr/bin/env python3
"""Where each library pack's game is sold besides Steam: GOG, Epic, Battle.net (and EA, Ubisoft).

    python3 -I scripts/store-check.py                  # table of every pack
    python3 -I scripts/store-check.py --missing-steam  # only packs with no steam_app_id rule
    python3 -I scripts/store-check.py --online         # also check GOG ids against api.gog.com
    python3 -I scripts/store-check.py --github         # packs whose wiki article is missing or lists no store: find the GitHub project
    python3 -I scripts/store-check.py --self-test

Reads the PCGamingWiki snapshot (pcgamingwiki/games.jsonl, by line) and matches packs to articles
with pack-audit.py's matcher (name, then Steam id). Padwight has only `steam_app_id` and
`executable` rules, so this doesn't change packs: it says which packs have no Steam id because
the game isn't on Steam (GOG, Epic or Battle.net only, or a free source port), and which wiki
GOG ids exist for a pack. `--online` asks GOG's public product API whether each id exists and
what it is called; Epic and Battle.net have no public lookup, so they come from the wiki alone.
`--github` covers packs with no wiki article or no store listed in it (source ports, and games
where the wiki is out of date). A game sold in any store is skipped: GitHub would only answer
with fan projects: it searches GitHub through
`gh api` (needs `gh auth login`), and prints the best repository, its latest release and the
release's asset names. Executable names inside the archives aren't visible, so compare the pack's
rules with the project's docs by hand. A pack can name its repository in `GITHUB` below when the
search guesses wrong.
Wiki data is community-written: treat it as unverified.
"""

import argparse
import importlib.util
import json
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
GOG_API = "https://api.gog.com/products/"
GITHUB = {"ironwail.padpack": "andrei-drexler/ironwail", "yamagi-quake-ii.padpack": "yquake2/yquake2"}  # where the search picks the wrong project
FAMILIES = {"gog": "GOG", "epic": "Epic", "battle.net": "Battle.net", "origin": "EA",
            "ea app": "EA", "ubisoft": "Ubisoft"}


def load_audit():
    spec = importlib.util.spec_from_file_location("pack_audit", ROOT / "scripts" / "pack-audit.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def family(store):
    s = store.strip().lower()
    for key, name in FAMILIES.items():
        if s.startswith(key):
            return name
    return None


def stores_of(record):
    """{family: [store ids]} from one games.jsonl record; GOG numeric ids come from gog_ids."""
    out = {}
    for s in record.get("stores", []):
        fam = family(s.get("store", ""))
        if fam:
            out.setdefault(fam, [])
            if s.get("id") and s["id"] not in out[fam]:
                out[fam].append(s["id"])
    for g in record.get("gog_ids") or []:
        out.setdefault("GOG", [])
        if g not in out["GOG"]:
            out["GOG"].append(g)
    if record.get("steam_appids"):
        out["Steam"] = list(record["steam_appids"])
    return out


def wiki_stores(wiki, files):
    found = {}
    with open(wiki / "games.jsonl", encoding="utf-8") as fh:
        for line in fh:
            # cheap prefilter: the record's own file name appears in it
            if not any(f'"file": "{f}"' in line[:400] for f in files):
                continue
            rec = json.loads(line)
            if rec["file"] in files:
                found[rec["file"]] = stores_of(rec)
    return found


def gog_lookup(gid, cache):
    if gid in cache:
        return cache[gid]
    req = urllib.request.Request(GOG_API + gid, headers={"User-Agent": "padwight-pack-audit"})
    try:
        with urllib.request.urlopen(req, timeout=20) as resp:
            body = json.load(resp)
        cache[gid] = body.get("title", "?")
    except urllib.error.HTTPError as err:
        cache[gid] = None if err.code == 404 else f"error {err.code}"
    except (urllib.error.URLError, TimeoutError, json.JSONDecodeError) as err:
        cache[gid] = f"error {err}"
    time.sleep(0.5)
    return cache[gid]


def gh(path):
    import subprocess
    out = subprocess.run(["gh", "api", path], capture_output=True, text=True, timeout=60)
    if out.returncode != 0:
        return None
    return json.loads(out.stdout)


def github_project(pack_file, name):
    """(repo, description, stars, latest tag, date, asset names) for the best match, or None."""
    from urllib.parse import quote
    repo = GITHUB.get(pack_file)
    if not repo:
        found = gh(f"search/repositories?q={quote(name + ' in:name')}&sort=stars&per_page=5")
        if not found or not found.get("items"):
            return None
        key = name.lower().replace(" ", "")
        items = found["items"]
        best = next((i for i in items if i["name"].lower().replace("-", "") == key), items[0])
        repo = best["full_name"]
    info = gh(f"repos/{repo}")
    if not info:
        return None
    rel = gh(f"repos/{repo}/releases/latest") or {}
    return (repo, info.get("description") or "", info.get("stargazers_count", 0),
            rel.get("tag_name", "no release"), (rel.get("published_at") or "")[:10],
            [a["name"] for a in rel.get("assets", [])][:8])


def run(args):
    pa = load_audit()
    wiki = pa.find_wiki(args.wiki)
    snap = pa.load_snapshot(wiki)
    packs = [pa.read_pack(p) for p in sorted((ROOT / "packs").glob("*.padpack"))]
    matched = {}
    for pack in packs:
        _match, games, _f = pa.audit(pack, snap)
        matched[pack["file"]] = games
    files = {f for g in matched.values() for f in g}
    by_file = wiki_stores(wiki, files)
    cache = {}
    rows = []
    for pack in packs:
        if args.missing_steam and pack["ids"]:
            continue
        games = matched[pack["file"]]
        stores = {}
        for f in games:
            for fam, ids in by_file.get(f, {}).items():
                stores.setdefault(fam, [])
                stores[fam] += [i for i in ids if i not in stores[fam]]
        notes = []
        if not games:
            notes.append("no wiki article")
        elif not pack["ids"] and "Steam" in stores:
            notes.append(f"on Steam ({','.join(stores['Steam'])}) but the pack has no steam_app_id")
        elif not pack["ids"] and not stores:
            notes.append("no store listed (free or source port?)")
        if args.online:
            for gid in stores.get("GOG", []):
                if gid.isdigit():
                    title = gog_lookup(gid, cache)
                    if title is None:
                        notes.append(f"GOG {gid} not found")
                    elif title.startswith("error"):
                        notes.append(f"GOG {gid}: {title}")
                    elif pa.norm(title) != pa.norm(pack["name"]):
                        notes.append(f"GOG {gid} is {title!r}")
        if args.github and not stores:  # a game sold anywhere would only match fan projects
            proj = github_project(pack["file"], pack["name"])
            if proj:
                repo, desc, stars, tag, date, assets = proj
                notes.append(f"GitHub {repo} ({stars} stars; {tag} {date}): {desc[:70]}")
                if assets:
                    notes.append("assets: " + ", ".join(assets))
                notes.append("pack rules: " + ", ".join(pack["exes"]))
                seen = (repo.split("/")[1] + " " + " ".join(assets)).lower().replace("-", "")
                unseen = [e for e in pack["exes"] if Path(e).stem.lower().replace("-", "") not in seen]
                if unseen:
                    notes.append("not in repo name or assets: " + ", ".join(unseen))
            else:
                notes.append("no GitHub project found")
        rows.append((pack["file"], stores, notes))
    for name, stores, notes in rows:
        shown = "  ".join(f"{k}:{','.join(v) or '-'}" for k, v in sorted(stores.items()))
        print(f"{name}\t{shown or '-'}" + ("\t" + "; ".join(notes) if notes else ""))
    print(f"\n{len(rows)} packs", file=sys.stderr)


def self_test():
    assert family("GOG.com") == "GOG" and family("gog") == "GOG"
    assert family("Epic games store") == "Epic" and family("Battle.net") == "Battle.net"
    assert family("Humble") is None and family("EA app") == "EA"
    rec = {"stores": [{"store": "Gog.com", "id": "x"}, {"store": "Humble", "id": "h"},
                      {"store": "Epic", "id": "e"}], "gog_ids": ["1", "1"], "steam_appids": ["9"]}
    got = stores_of(rec)
    assert got == {"GOG": ["x", "1"], "Epic": ["e"], "Steam": ["9"]}, got
    print("ok")


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--wiki")
    ap.add_argument("--missing-steam", action="store_true")
    ap.add_argument("--online", action="store_true")
    ap.add_argument("--github", action="store_true")
    ap.add_argument("--self-test", action="store_true")
    args = ap.parse_args()
    self_test() if args.self_test else run(args)


if __name__ == "__main__":
    main()
