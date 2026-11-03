#!/usr/bin/env python3
"""Executables of the library packs' games, read from GOG's depots and installed Steam copies.

    python3 -I scripts/find-executables.py [pack ...]       # report; names filter packs (substring)
    python3 -I scripts/find-executables.py --write          # also write pcgamingwiki/store-executables.tsv
    python3 -I scripts/find-executables.py --no-gog         # installed Steam copies only
    python3 -I scripts/find-executables.py --self-test

Two sources, neither needing a login:
  * GOG: the wiki's GOG ids for the pack's game -> content-system.gog.com latest Windows build ->
    its depot manifests (zlib JSON) -> every `.exe` path. Cached in pcgamingwiki/gog/<id>.json.
  * Steam, installed here: every library in libraryfolders.vdf, the appmanifest for the pack's
    steam_app_id rules, then the `.exe` files at most five folders deep in its install folder.
For each pack the report says which of its `executable` rules were found and which were not,
and lists other executables near the top of the install folder as candidates (installers,
uninstallers and redistributables left out). Wiki, GOG and local files are data: unverified.
`--write` saves the finds in executables.tsv's columns so pack-audit.py reads them too.
"""

import argparse
import csv
import json
import re
import sys
import time
import urllib.request
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BUILDS = "https://content-system.gog.com/products/{}/os/windows/builds?generation=2"
DEPOT = "https://cdn.gog.com/content-system/v2/meta/{}/{}/{}"
COLUMNS = ["executable", "os", "launcher", "arguments", "discord_name", "steam_appids",
           "other_skus", "pcgw_title", "pcgw_file"]
SKIP = re.compile(r"(unins|setup|install|redist|vcredist|dxsetup|dotnet|crash|helper|"
                  r"launcher_helper|ue4prereq|vc_redist|dxwebsetup|updater|patch)", re.I)


def load(name):
    import importlib.util
    spec = importlib.util.spec_from_file_location(name.replace("-", "_"), ROOT / "scripts" / f"{name}.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def http(url):
    req = urllib.request.Request(url, headers={"User-Agent": "padwight-pack-audit"})
    with urllib.request.urlopen(req, timeout=30) as resp:
        return resp.read()


def exe_names(paths):
    """Lowercased file names, with the parent folder kept: 'system/swat4.exe'."""
    out = set()
    for p in paths:
        parts = [x for x in re.split(r"[\\/]", p) if x]
        if parts and parts[-1].lower().endswith(".exe"):
            out.add("/".join(parts[-2:]).lower() if len(parts) > 1 else parts[0].lower())
    return out


def gog_executables(gid, cache_dir, refresh=False):
    cache = cache_dir / f"{gid}.json"
    if cache.is_file() and not refresh:
        return sorted(json.loads(cache.read_text()))
    builds = json.loads(http(BUILDS.format(gid)))
    if not builds.get("items"):
        return None
    meta = json.loads(zlib.decompress(http(builds["items"][0]["link"])))
    paths, seen = [], set()
    for depot in meta.get("depots", []):
        m = depot["manifest"]
        if m in seen:
            continue
        seen.add(m)
        data = json.loads(zlib.decompress(http(DEPOT.format(m[:2], m[2:4], m))))
        paths += [i["path"] for i in data.get("depot", {}).get("items", [])]
        time.sleep(0.2)
    names = sorted(exe_names(paths))
    cache_dir.mkdir(parents=True, exist_ok=True)
    cache.write_text(json.dumps(names))
    return names


def steam_roots():
    roots = [Path.home() / ".local/share/Steam", Path.home() / ".steam/steam"]
    libs = []
    for r in roots:
        vdf = r / "steamapps/libraryfolders.vdf"
        if vdf.is_file():
            libs += [Path(p) for p in re.findall(r'"path"\s+"([^"]+)"', vdf.read_text(errors="replace"))]
    seen, out = set(), []
    for p in libs + roots:
        rp = p.resolve() if p.exists() else p
        if rp not in seen and (rp / "steamapps").is_dir():
            seen.add(rp)
            out.append(rp)
    return out


def steam_installed(appid, roots):
    for r in roots:
        mf = r / "steamapps" / f"appmanifest_{appid}.acf"
        if mf.is_file():
            m = re.search(r'"installdir"\s+"([^"]+)"', mf.read_text(errors="replace"))
            folder = r / "steamapps/common" / m.group(1) if m else None
            if folder and folder.is_dir():
                base = folder
                paths = [str(p.relative_to(base)) for p in base.rglob("*")
                         if p.suffix.lower() == ".exe" and len(p.relative_to(base).parts) <= 6]
                return exe_names(paths)
    return None


def compare(rules, found):
    """(rules found, rules not found, candidates not in rules)."""
    have = {f.lower() for f in found}
    bare = {f.split("/")[-1] for f in have}

    def hit(rule):
        r = rule.lower().replace("\\", "/")
        return r in have or r.split("/")[-1] in bare and "/" not in r
    ok = [r for r in rules if hit(r)]
    miss = [r for r in rules if not hit(r)]
    names = {r.lower().split("/")[-1] for r in rules}
    cand = sorted(f for f in have if f.split("/")[-1] not in names and not SKIP.search(f))
    return ok, miss, cand


def run(args):
    pa, sc = load("pack-audit"), load("store-check")
    wiki = pa.find_wiki(args.wiki)
    snap = pa.load_snapshot(wiki)
    packs = [pa.read_pack(p) for p in sorted((ROOT / "packs").glob("*.padpack"))]
    if args.names:
        packs = [p for p in packs if any(n in p["file"] for n in args.names)]
    matched = {p["file"]: pa.audit(p, snap)[1] for p in packs}
    stores = sc.wiki_stores(wiki, {f for g in matched.values() for f in g})
    roots = steam_roots()
    rows = []
    for pack in packs:
        found, where = set(), []
        for a in pack["ids"]:
            got = steam_installed(a, roots)
            if got is not None:
                found |= got
                where.append(f"Steam {a} installed")
        if not args.no_gog:
            gids = {g for f in matched[pack["file"]] for g in stores.get(f, {}).get("GOG", [])
                    if g.isdigit()}
            for g in sorted(gids):
                try:
                    got = gog_executables(g, wiki / "gog", args.refresh)
                except Exception as err:  # network, bad JSON: say so and go on
                    print(f"  {pack['file']}: GOG {g}: {err}", file=sys.stderr)
                    continue
                if got:
                    found |= set(got)
                    where.append(f"GOG {g}")
        if not where:
            continue
        ok, miss, cand = compare(pack["exes"], found)
        print(f"{pack['file']}  [{', '.join(where)}]")
        print(f"  rules found: {', '.join(ok) or '-'}")
        if miss:
            print(f"  rules NOT found: {', '.join(miss)}")
        if cand:
            print(f"  other executables: {', '.join(cand[:12])}" + (" ..." if len(cand) > 12 else ""))
        for exe in sorted(found):
            title = snap.games[matched[pack["file"]][0]]["title"] if matched[pack["file"]] else pack["name"]
            file = matched[pack["file"]][0] if matched[pack["file"]] else ""
            rows.append([exe, "win32", "", "", pack["name"], ",".join(pack["ids"]), "", title, file])
    print(f"\n{len({r[4] for r in rows})} packs with a source", file=sys.stderr)
    if args.write:
        out = wiki / "store-executables.tsv"
        with open(out, "w", newline="", encoding="utf-8") as fh:
            w = csv.writer(fh, delimiter="\t", lineterminator="\n")
            w.writerow(COLUMNS)
            w.writerows(rows)
        print(f"wrote {out} ({len(rows)} rows)", file=sys.stderr)


def self_test():
    assert exe_names([r"Content\System\Swat4.exe", "readme.txt", "a.EXE"]) == {"system/swat4.exe", "a.exe"}
    ok, miss, cand = compare(["Swat4.exe", "gone.exe"], {"system/swat4.exe", "system/ucc.exe",
                                                         "unins000.exe"})
    assert ok == ["Swat4.exe"] and miss == ["gone.exe"] and cand == ["system/ucc.exe"], (ok, miss, cand)
    ok, miss, _ = compare(["system/swat4.exe"], {"system/swat4.exe"})
    assert ok == ["system/swat4.exe"] and not miss
    print("ok")


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("names", nargs="*")
    ap.add_argument("--wiki")
    ap.add_argument("--write", action="store_true")
    ap.add_argument("--no-gog", action="store_true")
    ap.add_argument("--refresh", action="store_true")
    ap.add_argument("--self-test", action="store_true")
    args = ap.parse_args()
    self_test() if args.self_test else run(args)


if __name__ == "__main__":
    main()
