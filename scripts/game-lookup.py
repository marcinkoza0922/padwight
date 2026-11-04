#!/usr/bin/env python3
"""Look a game up in the live store APIs when the PCGamingWiki snapshot leaves something unclear.

    python3 -I scripts/game-lookup.py "Portal"            # search by title (Steam, GOG)
    python3 -I scripts/game-lookup.py 400 620             # Steam app ids
    python3 -I scripts/game-lookup.py "Portal" --discord  # also Discord's detectable-games executables
    python3 -I scripts/game-lookup.py "Doom" --github     # also the best GitHub project (source ports; needs `gh`)
    python3 -I scripts/game-lookup.py --self-test

What each source answers:
  Steam store (appdetails)   name, type, release date, platforms, genres, controller support
                             (`full`/`partial`/none) and Steam's own category flags (Steam Input,
                             Full/Partial controller support, Remote Play), plus DLC and parent app
                             (a DLC or demo id must not become a pack rule)
  Steam app info (steamcmd)  the executables the developer set in `config.launch`, per OS, with arguments
  Steam store search         app ids for a title, to find editions and remasters
  GOG catalog                product ids, slug, release date, features (controller support), OS support
  Discord detectable list    executable names Discord matches for the game (--discord)
  GitHub (--github)          best repository, latest release and asset names

Everything is data from publishers, stores and communities: treat it as unverified, and say in the
pack's notes when a rule came only from one of these. Answers are cached under
pcgamingwiki/lookup/ (untracked); `--refresh` fetches again. One request per second at most.
"""

import argparse
import json
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CACHE = ROOT / "pcgamingwiki" / "lookup"
UA = {"User-Agent": "padwight-game-lookup"}
STEAM_DETAILS = "https://store.steampowered.com/api/appdetails?cc=us&l=en&appids="
STEAM_SEARCH = "https://store.steampowered.com/api/storesearch/?cc=us&l=en&term="
STEAM_INFO = "https://api.steamcmd.net/v1/info/"
GOG_CATALOG = "https://catalog.gog.com/v1/catalog?limit=5&productType=in%3Agame&query=like%3A"
DISCORD = "https://discord.com/api/v9/applications/detectable"
CONTROLLER_CATEGORIES = {28: "Full controller support", 18: "Partial controller support",
                         43: "Native Steam Controller support", 44: "Remote Play on TV"}
_last = [0.0]


def fetch(url, key, refresh=False):
    """JSON from `url`, cached as lookup/<key>.json. None when the request fails."""
    path = CACHE / f"{key}.json"
    if path.is_file() and not refresh:
        return json.loads(path.read_text())
    wait = 1.0 - (time.monotonic() - _last[0])
    if wait > 0:
        time.sleep(wait)
    _last[0] = time.monotonic()
    try:
        with urllib.request.urlopen(urllib.request.Request(url, headers=UA), timeout=30) as r:
            data = json.load(r)
    except (urllib.error.URLError, TimeoutError, json.JSONDecodeError) as e:
        print(f"  ! {url}: {e}", file=sys.stderr)
        return None
    CACHE.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data))
    return data


def summarize_steam(appid, raw):
    entry = (raw or {}).get(str(appid)) or {}
    d = entry.get("data")
    if not entry.get("success") or not d:
        return None
    cats = [c["description"] for c in d.get("categories", [])]
    return {
        "appid": d.get("steam_appid", appid), "name": d.get("name"), "type": d.get("type"),
        "parent": (d.get("fullgame") or {}).get("appid"),
        "release": (d.get("release_date") or {}).get("date"),
        "platforms": [p for p, on in (d.get("platforms") or {}).items() if on],
        "genres": [g["description"] for g in d.get("genres", [])],
        "controller_support": d.get("controller_support"),
        "controller_flags": [c for c in cats if "ontroller" in c or "Steam Input" in c],
        "dlc": d.get("dlc", [])[:8],
    }


def launch_entries(raw, appid):
    app = ((raw or {}).get("data") or {}).get(str(appid)) or {}
    out = []
    for e in (app.get("config", {}).get("launch") or {}).values():
        if not e.get("executable"):
            continue
        os_ = (e.get("config") or {}).get("oslist", "all")
        out.append((e["executable"], os_, e.get("arguments", ""), e.get("description", "")))
    return out


def gog_products(raw):
    out = []
    for p in (raw or {}).get("products", []):
        out.append({
            "id": p.get("id"), "title": p.get("title"), "slug": p.get("slug"),
            "release": p.get("releaseDate"), "os": p.get("operatingSystems"),
            "controller": any(f.get("slug") == "controller_support" for f in p.get("features", [])),
        })
    return out


def discord_matches(raw, title):
    t = title.lower()
    out = []
    for app in raw or []:
        names = [app.get("name", "")] + app.get("aliases", [])
        if any(t in n.lower() for n in names):
            exes = [(e["name"], e.get("os")) for e in app.get("executables", [])]
            out.append((app["name"], exes))
    return out


def github_best(title):
    q = urllib.parse.quote(f"{title} in:name,description")
    r = subprocess.run(["gh", "api", f"search/repositories?q={q}&sort=stars&per_page=1"],
                       capture_output=True, text=True)
    if r.returncode:
        return f"gh failed: {r.stderr.strip()[:100]}"
    items = json.loads(r.stdout).get("items", [])
    if not items:
        return "no repository"
    repo = items[0]["full_name"]
    rel = subprocess.run(["gh", "api", f"repos/{repo}/releases/latest"], capture_output=True, text=True)
    assets = []
    if not rel.returncode:
        j = json.loads(rel.stdout)
        assets = [j.get("tag_name", "")] + [a["name"] for a in j.get("assets", [])][:8]
    return f"{repo} ({items[0]['stargazers_count']} stars) {' '.join(assets)}"


def show_steam(appid, refresh):
    s = summarize_steam(appid, fetch(STEAM_DETAILS + str(appid), f"steam-{appid}", refresh))
    if not s:
        print(f"  Steam {appid}: no store page (delisted, region-locked or not an app)")
        return
    kind = s["type"] + (f" of {s['parent']}" if s["parent"] else "")
    print(f"  Steam {s['appid']}: {s['name']} [{kind}] released {s['release']}")
    print(f"    platforms {','.join(s['platforms'])}; genres {', '.join(s['genres'])}")
    print(f"    controller_support={s['controller_support']}; flags: {'; '.join(s['controller_flags']) or 'none'}")
    if s["dlc"]:
        print(f"    DLC ids: {s['dlc']}")
    info = fetch(STEAM_INFO + str(appid), f"steaminfo-{appid}", refresh)
    for exe, os_, args, desc in launch_entries(info, appid):
        print(f"    launch: {exe}  os={os_}  args={args!r}  {desc}")


def lookup(query, args):
    print(f"== {query}")
    if query.isdigit():
        show_steam(query, args.refresh)
        return
    key = urllib.parse.quote(query.lower(), safe="")
    found = fetch(STEAM_SEARCH + urllib.parse.quote(query), f"search-{key}", args.refresh)
    items = (found or {}).get("items", [])[:5]
    for it in items:
        show_steam(it["id"], args.refresh)
    if not items:
        print("  Steam: no search results")
    gog = fetch(GOG_CATALOG + urllib.parse.quote(query), f"gog-{key}", args.refresh)
    words = query.lower().split()
    for p in gog_products(gog):
        if not all(w in (p["title"] or "").lower() for w in words):
            continue  # the catalog search is fuzzy; keep titles that contain every word
        print(f"  GOG {p['id']}: {p['title']} ({p['slug']}) released {p['release']} os={p['os']} "
              f"controller={'yes' if p['controller'] else 'no'}")
    if args.discord:
        for name, exes in discord_matches(fetch(DISCORD, "discord", args.refresh), query):
            print(f"  Discord: {name}: {', '.join(f'{n} ({o})' for n, o in exes)}")
    if args.github:
        print(f"  GitHub: {github_best(query)}")


def self_test():
    raw = {"400": {"success": True, "data": {
        "type": "game", "name": "Portal", "steam_appid": 400, "controller_support": "full",
        "release_date": {"date": "10 Oct, 2007"}, "platforms": {"windows": True, "mac": False, "linux": True},
        "genres": [{"description": "Action"}], "categories": [
            {"id": 28, "description": "Full controller support"}, {"id": 2, "description": "Single-player"}]}}}
    s = summarize_steam(400, raw)
    assert s["controller_support"] == "full" and s["platforms"] == ["windows", "linux"], s
    assert s["controller_flags"] == ["Full controller support"], s
    assert summarize_steam(1, {"1": {"success": False}}) is None
    info = {"data": {"400": {"config": {"launch": {"0": {"executable": "hl2.exe", "arguments": "-game portal",
                                                         "config": {"oslist": "windows"}}}}}}}
    assert launch_entries(info, 400) == [("hl2.exe", "windows", "-game portal", "")]
    gog = {"products": [{"id": "1", "title": "X", "slug": "x", "features": [{"slug": "controller_support"}]}]}
    assert gog_products(gog)[0]["controller"] is True
    disc = [{"name": "Portal", "aliases": [], "executables": [{"name": "hl2p.exe", "os": "win32"}]}]
    assert discord_matches(disc, "portal") == [("Portal", [("hl2p.exe", "win32")])]
    print("self-test ok")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("queries", nargs="*", help="game titles or Steam app ids")
    ap.add_argument("--refresh", action="store_true", help="ignore cached answers")
    ap.add_argument("--discord", action="store_true", help="also Discord's detectable executables")
    ap.add_argument("--github", action="store_true", help="also the best GitHub project (needs gh)")
    ap.add_argument("--self-test", action="store_true")
    args = ap.parse_args()
    if args.self_test:
        return self_test()
    if not args.queries:
        ap.error("give a title or a Steam app id")
    for q in args.queries:
        lookup(q, args)


if __name__ == "__main__":
    main()
