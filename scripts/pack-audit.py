#!/usr/bin/env python3
"""Pack identity audit and status table (QA tooling T-13).

For every library pack (packs/*.padpack) find the game in the local PCGamingWiki snapshot and
report, offline and without guessing:

  * Steam app ids the wiki doesn't list for the game (or doesn't know at all),
  * executable names Discord's detectable-games list never saw for the game,
  * a release year or native controller support that disagrees with the pack's own text and shape,
  * packs whose game can't be found, or matches several games (reported, never picked).

It writes docs/development/pack-qa.tsv: one row per pack with the audit result, plus the manual
columns of the pack QA (bindings checked, last smoke, last full play, notes). Manual columns are
read back from the existing file and kept; only the audit columns are rewritten.

    python3 -I scripts/pack-audit.py                 # audit, write pack-qa.tsv, print a summary
    python3 -I scripts/pack-audit.py -v              # also print every finding
    python3 -I scripts/pack-audit.py --check         # exit 1 if pack-qa.tsv is out of date
    python3 -I scripts/pack-audit.py --self-test     # test the matching logic on a tiny snapshot
    python3 -I scripts/pack-audit.py --wiki DIR --packs DIR --out FILE

The snapshot (pcgamingwiki/, see its README.md) is not tracked. It is looked for under --wiki,
then ./pcgamingwiki, then the main checkout of this repository (for git worktrees).
games.jsonl is never opened; only the TSV files are read.
"""

import argparse
import csv
import difflib
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MANUAL = ["bindings_checked", "last_smoke", "last_full_play", "manual_notes"]
AUDIT = ["pack", "name", "match", "game", "wiki_file", "steam_ids", "executables",
         "status", "findings", "details"]
COLUMNS = AUDIT + MANUAL
SEVERITY = {"info": 1, "warn": 2, "error": 3}
XINPUT_YEAR = 2006  # game-setup-guidelines.md: games before 2006 don't count as native XInput


def norm(text):
    text = text.lower().replace("&", " and ")
    text = re.sub(r"[^a-z0-9]+", " ", text)
    return re.sub(r"^the ", "", text.strip()).strip()


def strip_paren(title):
    return re.sub(r"\s*\([^)]*\)\s*", " ", title).strip()


def years(text):
    return sorted({int(y) for y in re.findall(r"\b(19[89]\d|20[0-3]\d)\b", text or "")})


class Snapshot:
    """The slices of the PCGamingWiki snapshot the audit needs."""

    def __init__(self, games, appids, redirects, exes):
        self.games = {g["file"]: g for g in games}
        self.by_title = {}   # normalized title -> {file}
        self.by_loose = {}   # normalized title without "(...)" -> {file}
        for g in games:
            self.by_title.setdefault(norm(g["title"]), set()).add(g["file"])
            self.by_loose.setdefault(norm(strip_paren(g["title"])), set()).add(g["file"])
        self.title_file = {g["title"]: g["file"] for g in games}
        for src, dst in redirects:
            f = self.title_file.get(dst)
            if f:
                self.by_title.setdefault(norm(src), set()).add(f)
                self.by_loose.setdefault(norm(strip_paren(src)), set()).add(f)
        self.appid_files = {}
        for appid, _title, f in appids:
            if f in self.games:
                self.appid_files.setdefault(appid, set()).add(f)
        self.known_appids = {a for a, _t, _f in appids}
        self.exes_by_file = {}
        self.exes_by_appid = {}
        for e in exes:
            name = e["executable"].lower()
            if e["pcgw_file"]:
                self.exes_by_file.setdefault(e["pcgw_file"], set()).add(name)
            for a in filter(None, e["steam_appids"].split(",")):
                self.exes_by_appid.setdefault(a, set()).add(name)

    def name_candidates(self, name):
        key = norm(name)
        return self.by_title.get(key) or self.by_loose.get(key) or set()

    def suggestions(self, name, n=3):
        words = [w for w in norm(name).split() if len(w) > 2]
        if not words:
            return []
        first = words[0]
        pool = [g["title"] for g in self.games.values() if first in norm(g["title"]).split()]
        return difflib.get_close_matches(name, pool, n=n, cutoff=0.6)

    def exes_for(self, files, appids):
        seen = set()
        for f in files:
            seen |= self.exes_by_file.get(f, set())
        for a in set(appids) | {a for f in files for a in
                                filter(None, self.games[f]["steam_appids"].split(","))}:
            seen |= self.exes_by_appid.get(a, set())
        return seen


def load_snapshot(wiki):
    def tsv(name):
        with open(wiki / name, newline="", encoding="utf-8") as fh:
            return list(csv.DictReader(fh, delimiter="\t", quoting=csv.QUOTE_NONE))

    games = tsv("games.tsv")
    appids = [(r["steam_appid"], r["title"], r["file"]) for r in tsv("steam-appids.tsv")]
    redirects = [(r["from"], r["to"]) for r in tsv("redirects.tsv")]
    return Snapshot(games, appids, redirects, tsv("executables.tsv"))


def find_wiki(arg):
    cands = [Path(arg)] if arg else [Path("pcgamingwiki"), ROOT / "pcgamingwiki"]
    if not arg:
        for parent in ROOT.parents:  # worktree under <main>/.claude/worktrees/<name>
            cands.append(parent / "pcgamingwiki")
    for c in cands:
        if (c / "games.tsv").is_file():
            return c
    sys.exit("pcgamingwiki snapshot not found (see pcgamingwiki/README.md); pass --wiki DIR")


# ---- reading a pack -------------------------------------------------------------------------

def read_pack(path):
    with open(path, "rb") as fh:
        d = tomllib.load(fh)
    rules = d.get("rules", [])
    profiles = d.get("profiles", [])
    texts = [d.get("pack", {}).get("description", "")]
    texts += [p.get("notes", "") for p in profiles if p.get("notes")]
    passthrough = False
    if profiles:
        buttons = profiles[0].get("buttons", {})
        same = sum(1 for k, v in buttons.items() if isinstance(v, dict) and v.get("gamepad") == k)
        passthrough = same >= 10
    return {
        "file": path.name,
        "name": d.get("pack", {}).get("name", path.stem),
        "ids": [r["value"] for r in rules if r.get("kind") == "steam_app_id"],
        "exes": [r["value"] for r in rules if r.get("kind") == "executable"],
        "text": "\n".join(texts),
        "description": d.get("pack", {}).get("description", ""),
        "passthrough": passthrough,
    }


# ---- the audit ------------------------------------------------------------------------------

def audit(pack, snap):
    """Returns (match, [game files], [findings]); a finding is (kind, severity, detail)."""
    out = []

    def add(kind, sev, detail):
        out.append((kind, sev, detail))

    def title(f):
        return snap.games[f]["title"]

    ids = pack["ids"]
    id_files = {}
    for a in ids:
        files = snap.appid_files.get(a, set())
        id_files[a] = files
        if not files:
            add("id_unknown", "warn", f"{a} is not in the wiki's Steam id list")
    all_id_games = set().union(*id_files.values()) if id_files else set()
    name_games = snap.name_candidates(pack["name"])

    games, match = [], "none"
    if len(name_games) == 1:
        (g,) = name_games
        games, match = [g], "name"
        if all_id_games and g not in all_id_games:
            add("id_wrong_game", "error", "no Steam id is listed for " + repr(title(g)) +
                "; they belong to " + "; ".join(sorted(title(f) for f in all_id_games)))
        else:
            for a, files in id_files.items():
                if files and g not in files:
                    add("id_other_game", "warn", f"{a} is listed for "
                        + "; ".join(sorted(title(f) for f in files)) + f", not {title(g)!r}")
    elif len(name_games) > 1:
        narrowed = name_games & all_id_games
        if len(narrowed) == 1:
            games, match = sorted(narrowed), "name+ids"
        else:
            match = "ambiguous"
            add("ambiguous", "warn", "name matches " +
                "; ".join(sorted(f"{title(f)} [{snap.games[f]['steam_appids']}]" for f in name_games)))
    elif all_id_games:
        games, match = sorted(all_id_games), "ids"
        if len(games) == 1:
            if norm(title(games[0])) != norm(pack["name"]):
                add("name_differs", "info", f"wiki title is {title(games[0])!r}")
        else:
            add("multi_game", "info", "ids span " + "; ".join(sorted(title(f) for f in games)))
    else:
        add("no_match", "warn", "no wiki article by name or Steam id"
            + (" (maybe " + "; ".join(snap.suggestions(pack["name"])) + ")"
               if snap.suggestions(pack["name"]) else ""))

    if not ids:
        add("no_steam_id", "info", "pack has no steam_app_id rule")
    if not pack["exes"]:
        add("no_exe", "info", "pack has no executable rule")

    if games:
        check_executables(pack, snap, games, add)
        check_release_and_controller(pack, snap, games, add)
    return match, games, out


def exe_ok(name, seen):
    name = name.lower()
    base = name.rsplit("/", 1)[-1]
    return any(name == s or base == s.rsplit("/", 1)[-1] for s in seen)


def check_executables(pack, snap, games, add):
    if not pack["exes"]:
        return
    seen = snap.exes_for(games, pack["ids"])
    if not seen:
        add("exe_no_data", "info", "Discord's list has no executables for this game")
        return
    for e in pack["exes"]:
        if not exe_ok(e, seen):
            add("exe_unseen", "warn", f"{e} (seen: {', '.join(sorted(seen)[:6])}"
                + (", ..." if len(seen) > 6 else "") + ")")


def check_release_and_controller(pack, snap, games, add):
    g = snap.games[games[0]] if len(games) == 1 else None
    rel = sorted({y for f in games for y in years(snap.games[f]["released"])})
    text_years = years(" ".join(re.findall(r"\(\d{4}\)", pack["description"])))
    if rel and text_years and not set(text_years) & set(rel):
        add("year_mismatch", "warn", f"pack says {text_years}, wiki says {rel}")
    if not rel:
        return
    first = min(rel)
    if g is None:
        return
    native = g["controller_support"] == "true" and g["xinput"] in ("true", "limited") \
        or g["full_controller"] == "true" and g["xinput"] == "true"
    cs = g["controller_support"]
    facts = f"controller_support={cs or '-'} xinput={g['xinput'] or '-'} released={first}"
    if pack["passthrough"]:
        if first < XINPUT_YEAR:
            add("passthrough_pre2006", "warn", f"passthrough pack, but {facts}")
        elif cs in ("false", "n/a") or g["xinput"] == "false":
            add("passthrough_no_support", "warn", f"passthrough pack, but {facts}")
        elif not native:
            add("passthrough_unconfirmed", "info", f"passthrough pack, wiki unsure: {facts}")
    elif native and first >= XINPUT_YEAR and g["full_controller"] == "true":
        add("mapped_but_native", "warn", f"remapped pack, but wiki lists full controller support: {facts}")


# ---- the table ------------------------------------------------------------------------------

def audit_row(pack, snap):
    match, games, findings = audit(pack, snap)
    status = max((s for _k, s, _d in findings), key=SEVERITY.get, default="ok")
    return {
        "pack": pack["file"], "name": pack["name"], "match": match,
        "game": "; ".join(snap.games[f]["title"] for f in games),
        "wiki_file": "; ".join(games),
        "steam_ids": ",".join(pack["ids"]), "executables": ",".join(pack["exes"]),
        "status": status,
        "findings": ",".join(sorted({k for k, _s, _d in findings})),
        "details": " | ".join(f"{k}: {d}" for k, _s, d in findings),
    }, findings


def clean(v):
    return (v or "").replace("\t", " ").replace("\n", " ")


def read_table(path):
    if not path.is_file():
        return {}
    with open(path, newline="", encoding="utf-8") as fh:
        return {r["pack"]: r for r in csv.DictReader(fh, delimiter="\t")}


def render(rows, old):
    lines = ["\t".join(COLUMNS)]
    for r in rows:
        keep = old.get(r["pack"], {})
        full = {**r, **{c: keep.get(c, "") for c in MANUAL}}
        lines.append("\t".join(clean(full.get(c, "")) for c in COLUMNS))
    return "\n".join(lines) + "\n"


def run(args):
    wiki = find_wiki(args.wiki)
    snap = load_snapshot(wiki)
    packs_dir = Path(args.packs)
    paths = sorted(packs_dir.glob("*.padpack"))
    if not paths:
        sys.exit(f"no packs in {packs_dir}")
    out = Path(args.out)
    old = read_table(out)
    rows, by_kind, flagged = [], {}, 0
    for p in paths:
        row, findings = audit_row(read_pack(p), snap)
        rows.append(row)
        if any(s != "info" for _k, s, _d in findings):
            flagged += 1
        for k, s, d in findings:
            by_kind.setdefault(k, []).append((p.name, s, d))
            if args.verbose:
                print(f"{p.name}: [{s}] {k}: {d}")
    text = render(rows, old)
    if args.check:
        if not out.is_file() or out.read_text(encoding="utf-8") != text:
            print(f"{out} is out of date; run scripts/pack-audit.py")
            return 1
    else:
        out.write_text(text, encoding="utf-8")
        print(f"wrote {out} ({len(rows)} packs, snapshot {wiki})")
    print(f"{flagged} of {len(rows)} packs flagged (warn or error)")
    for k, items in sorted(by_kind.items(), key=lambda kv: -len(kv[1])):
        print(f"  {k}: {len(items)}")
        for name, s, d in items[:3]:
            print(f"      {name}: {d[:110]}")
    return 0


# ---- self-test ------------------------------------------------------------------------------

def self_test():
    def g(title, ids, released, cs="", xin="", full=""):
        return {"title": title, "file": f"pages/{title}.wiki", "steam_appids": ids,
                "released": released, "controller_support": cs, "xinput": xin,
                "full_controller": full}
    games = [
        g("Alpha", "10", "Windows: May 1, 2019", "true", "true", "true"),
        g("Beta (2005)", "20", "Windows: May 1, 2005"),
        g("Beta (2020)", "21", "Windows: May 1, 2020"),
        g("Gamma Remastered", "30,31", "Windows: May 1, 2015", "false", "false"),
        g("Delta", "40", "Windows: May 1, 2010"),
    ]
    appids = [(a, x["title"], x["file"]) for x in games for a in x["steam_appids"].split(",")]
    redirects = [("Gamma", "Gamma Remastered")]
    exes = [
        {"executable": "alpha.exe", "steam_appids": "10", "pcgw_file": "pages/Alpha.wiki"},
        {"executable": "alpha/bin/alpha64.exe", "steam_appids": "10", "pcgw_file": "pages/Alpha.wiki"},
        {"executable": "delta.exe", "steam_appids": "40", "pcgw_file": ""},
    ]
    snap = Snapshot(games, appids, redirects, exes)

    def pk(name, ids=(), exes=(), passthrough=False, desc=""):
        return {"file": "x.padpack", "name": name, "ids": list(ids), "exes": list(exes),
                "text": desc, "description": desc, "passthrough": passthrough}

    def kinds(p):
        m, gs, f = audit(p, snap)
        return m, [x.split("/")[-1] for x in gs], sorted(k for k, _s, _d in f)

    cases = [
        # exact match, listed id, case-insensitive exe and folder-stripped exe: clean
        (pk("Alpha", ["10"], ["ALPHA.EXE", "alpha64.exe"], True), ("name", ["Alpha.wiki"], [])),
        # id of another game, exe never seen
        (pk("Alpha", ["40"], ["nope.exe"], True),
         ("name", ["Alpha.wiki"], ["exe_unseen", "id_wrong_game"])),
        # one right id, one belonging to another game
        (pk("Alpha", ["10", "40"], ["alpha.exe"], True), ("name", ["Alpha.wiki"], ["id_other_game"])),
        # unknown id
        (pk("Alpha", ["10", "999"], [], True), ("name", ["Alpha.wiki"], ["id_unknown", "no_exe"])),
        # ambiguous name is reported, not guessed ...
        (pk("Beta"), ("ambiguous", [], ["ambiguous", "no_exe", "no_steam_id"])),
        # ... unless an id picks one
        (pk("Beta", ["21"], ["b.exe"]), ("name+ids", ["Beta (2020).wiki"], ["exe_no_data"])),
        # redirect title, passthrough on a game the wiki says has no controller support
        (pk("Gamma", ["30"], [], True), ("name", ["Gamma Remastered.wiki"],
                                         ["no_exe", "passthrough_no_support"])),
        # match by id only
        (pk("Delta Deluxe", ["40"], ["delta.exe"]), ("ids", ["Delta.wiki"], ["name_differs"])),
        # nothing at all
        (pk("Zzz Unheard", [], ["z.exe"]), ("none", [], ["no_match", "no_steam_id"])),
        # passthrough on a pre-2006 game
        (pk("Beta (2005)", [], [], True), ("name", ["Beta (2005).wiki"],
                                           ["no_exe", "no_steam_id", "passthrough_pre2006"])),
        # a remapped pack on a game with full native support, and a wrong year in the text
        (pk("Alpha", ["10"], ["alpha.exe"], False, "Alpha (2012) on a pad."),
         ("name", ["Alpha.wiki"], ["mapped_but_native", "year_mismatch"])),
    ]
    bad = 0
    for p, want in cases:
        got = kinds(p)
        if got != want:
            bad += 1
            print(f"FAIL {p['name']} {p['ids']}: got {got}, want {want}")
    # manual columns survive a rerun
    rows = [audit_row(pk("Alpha", ["10"], ["alpha.exe"]), snap)[0]]
    old = {"x.padpack": {"bindings_checked": "yes", "last_smoke": "2026-11-01",
                         "last_full_play": "", "manual_notes": "ok"}}
    cells = render(rows, old).splitlines()[1].split("\t")
    if cells[COLUMNS.index("bindings_checked")] != "yes" or cells[COLUMNS.index("manual_notes")] != "ok":
        bad += 1
        print("FAIL manual columns were not kept")
    print("self-test:", "FAILED" if bad else f"ok ({len(cases) + 1} checks)")
    return 1 if bad else 0


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--wiki")
    ap.add_argument("--packs", default=str(ROOT / "packs"))
    ap.add_argument("--out", default=str(ROOT / "docs/development/pack-qa.tsv"))
    ap.add_argument("-v", "--verbose", action="store_true")
    ap.add_argument("--check", action="store_true")
    ap.add_argument("--self-test", action="store_true")
    args = ap.parse_args()
    sys.exit(self_test() if args.self_test else run(args))


if __name__ == "__main__":
    main()
