#!/usr/bin/env python3
"""Compare a library pack's keys with the game's own default bindings (QA tool T-14).

    python3 -I scripts/binding-compare.py list
    python3 -I scripts/binding-compare.py compare [game ...] [--all]    # installed games, found by search
    python3 -I scripts/binding-compare.py compare deus-ex --file User.ini [--file DefUser.ini]
    python3 -I scripts/binding-compare.py dump-pack packs/deus-ex.padpack
    python3 -I scripts/binding-compare.py dump-game deus-ex [--file path]
    python3 -I scripts/binding-compare.py self-test

Each engine reader turns a binding file into `action -> {keys}`; the per-game table in
`binding-compare.toml` maps pack selectors (an input path like `button:South`, a Guide label like
`Reload`, or a menu item like `menu:Belt/Belt 3`) to the engine's action names. Keys on both sides are
normalised to the pack's spelling (KEY_LEFTBRACE -> `leftbrace`). Read-only: game files are never written.
"""
import argparse
import glob
import os
import re
import sys
import tomllib

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
TABLES = os.path.join(HERE, "binding-compare.toml")
FIXTURES = os.path.join(ROOT, "tests", "fixtures", "bindings")

# ---------------------------------------------------------------- pack side


def strip_tags(s):
    s = re.sub(r"\{[^}]*\}", "", s)
    return re.sub(r"\s+", " ", s).strip()


def canon_pack_key(k):
    return k[4:].lower() if k.startswith("KEY_") else k.lower()


def action_keys(a):
    """Every way an action can be sent, as canonical key strings; chords join with '+', actions sent together (multi) with '&'."""
    if not isinstance(a, dict):
        return []
    out = []
    if "keys" in a:
        out.append("+".join(sorted(canon_pack_key(k) for k in a["keys"])))
    if isinstance(a.get("mouse"), str):
        out.append("mouse_" + a["mouse"].lower())
    if isinstance(a.get("wheel"), str):
        out.append("wheel_" + a["wheel"].lower())
    if "multi" in a:
        flat = []
        for x in a["multi"]:
            p = action_keys(x)
            if p:
                flat.append(p[0])
        if flat:
            out.append("&".join(sorted(flat)))
    for k in ("toggle", "hold", "repeat", "button"):
        v = a.get(k)
        if isinstance(v, dict):
            out += action_keys(v.get("action", v))
    if isinstance(a.get("action"), dict):
        out += action_keys(a["action"])
    return out


def pack_entries(pack, profile=0):
    """Selector -> (label, [keys]) for the pack's base profile."""
    pr = pack["profiles"][profile]
    edits = (pr.get("guide") or {}).get("edits", {})
    ents = {}

    def add(sel, keys):
        if keys:
            ents[sel] = (strip_tags(edits.get(sel, {}).get("text", "")), keys)

    for b, a in pr.get("buttons", {}).items():
        add("button:" + b, action_keys(a))
    for b, g in pr.get("gestures", {}).items():
        for gname, a in g.items():
            add(f"gesture:button:{b}:{gname}", action_keys(a))
    for side in ("left", "right"):
        t = pr.get(side + "_trigger") or {}
        add("trigger:" + side.capitalize(), action_keys(t.get("action")))
        st = pr.get(side + "_stick") or {}
        ring = (st.get("action") or {}).get("ring")
        if ring:
            for i, a in enumerate(ring.get("actions", [])):
                add(f"stick:{side.capitalize()}/ring{i}", action_keys(a))
        elif st.get("action"):
            add("stick:" + side.capitalize(), action_keys(st["action"]))
    for m in pack.get("menus", []):
        for it in m.get("items", []):
            label = strip_tags(it.get("label", ""))
            keys = action_keys(it.get("action"))
            if keys:
                ents[f"menu:{m['name']}/{label}"] = (label, keys)
    return ents


def resolve(ents, sel):
    """Find a selector by exact path, then by Guide or menu-item label."""
    if sel in ents:
        return sel, ents[sel]
    low = sel.lower()
    for k, (label, keys) in ents.items():
        if label and label.lower() == low:
            return k, (label, keys)
    return None, None

# ---------------------------------------------------------------- key names


SYMBOLS = {"'": "apostrophe", ",": "comma", ".": "dot", "/": "slash", ";": "semicolon", "[": "leftbrace",
           "]": "rightbrace", "-": "minus", "=": "equal", "`": "grave", "~": "grave", "\\": "backslash",
           "+": "plus"}
COMMON = {"space": "space", "enter": "enter", "return": "enter", "tab": "tab", "backspace": "backspace",
          "shift": "leftshift", "lshift": "leftshift", "rshift": "rightshift", "ctrl": "leftctrl",
          "control": "leftctrl", "lctrl": "leftctrl", "lcontrol": "leftctrl", "rctrl": "rightctrl",
          "alt": "leftalt", "lalt": "leftalt", "ralt": "rightalt", "pause": "pause", "capslock": "capslock",
          "escape": "esc", "esc": "esc", "home": "home", "end": "end", "up": "up", "down": "down",
          "left": "left", "right": "right", "uparrow": "up", "downarrow": "down", "leftarrow": "left",
          "rightarrow": "right", "insert": "insert", "ins": "insert", "delete": "delete", "del": "delete",
          "pageup": "pageup", "pgup": "pageup", "pagedown": "pagedown", "pgdn": "pagedown",
          "backquote": "grave", "tilde": "grave", "backslash": "backslash", "semicolon": "semicolon",
          "print_screen": "sysrq", "numlock": "numlock", "scrolllock": "scrolllock"}
KEYPAD = {"enter": "kpenter", "up": "kp8", "down": "kp2", "left": "kp4", "right": "kp6", "home": "kp7",
          "end": "kp1", "pgup": "kp9", "pgdn": "kp3", "center": "kp5", "ins": "kp0", "del": "kpdot",
          "plus": "kpplus", "minus": "kpminus", "star": "kpasterisk", "slash": "kpslash"}
MOUSE = {"1": "mouse_left", "2": "mouse_right", "3": "mouse_middle"}


def norm_generic(name):
    n = name.strip().strip('"')
    if n in SYMBOLS:
        return SYMBOLS[n]
    low = n.lower()
    if len(low) == 1 and low.isalnum():
        return low
    if re.fullmatch(r"f\d{1,2}", low):
        return low
    return COMMON.get(low, low)


def norm_unreal(name):
    low = name.lower()
    table = {"leftmouse": "mouse_left", "rightmouse": "mouse_right", "middlemouse": "mouse_middle",
             "mousewheelup": "wheel_up", "mousewheeldown": "wheel_down", "equals": "equal",
             "period": "dot", "leftbracket": "leftbrace", "rightbracket": "rightbrace",
             "singlequote": "apostrophe", "quote": "apostrophe", "greystar": "kpasterisk",
             "greyplus": "kpplus", "greyminus": "kpminus", "greyslash": "kpslash",
             "numpadperiod": "kpdot"}
    if low in table:
        return table[low]
    m = re.fullmatch(r"numpad(\d)", low)
    if m:
        return "kp" + m.group(1)
    return norm_generic(name)


def norm_dark(name):
    low = name.strip().lower()
    if low in ("mouse1", "mouse2", "mouse3"):
        return MOUSE[low[-1]]
    if low == "mouse_wheel_up":
        return "wheel_up"
    if low == "mouse_wheel_down":
        return "wheel_down"
    if low.startswith("keypad_") and low[7:] in KEYPAD:
        return KEYPAD[low[7:]]
    return norm_generic(name)


def norm_source(name):
    n = name.strip().strip('"')
    low = n.lower()
    if n == "\\\\":  # kb_def.lst escapes the backslash
        return "backslash"
    if low in ("mouse1", "mouse2", "mouse3"):
        return MOUSE[low[-1]]
    if low == "mwheelup":
        return "wheel_up"
    if low == "mwheeldown":
        return "wheel_down"
    if low.startswith("kp_"):
        sub = {"multiply": "star", "divide": "slash"}.get(low[3:], low[3:])
        return KEYPAD.get(sub, low)
    return norm_generic(n)


DIK = {0x01: "esc", 0x0c: "minus", 0x0d: "equal", 0x0e: "backspace", 0x0f: "tab", 0x1a: "leftbrace",
       0x1b: "rightbrace", 0x1c: "enter", 0x1d: "leftctrl", 0x27: "semicolon", 0x28: "apostrophe",
       0x29: "grave", 0x2a: "leftshift", 0x2b: "backslash", 0x33: "comma", 0x34: "dot", 0x35: "slash",
       0x36: "rightshift", 0x37: "kpasterisk", 0x38: "leftalt", 0x39: "space", 0x3a: "capslock",
       0x45: "numlock", 0x46: "scrolllock", 0x47: "kp7", 0x48: "kp8", 0x49: "kp9", 0x4a: "kpminus",
       0x4b: "kp4", 0x4c: "kp5", 0x4d: "kp6", 0x4e: "kpplus", 0x4f: "kp1", 0x50: "kp2", 0x51: "kp3",
       0x52: "kp0", 0x53: "kpdot", 0x57: "f11", 0x58: "f12", 0x9c: "kpenter", 0x9d: "rightctrl",
       0xb5: "kpslash", 0xb8: "rightalt", 0xc5: "pause", 0xc7: "home", 0xc8: "up", 0xc9: "pageup",
       0xcb: "left", 0xcd: "right", 0xcf: "end", 0xd0: "down", 0xd1: "pagedown", 0xd2: "insert",
       0xd3: "delete"}
for _i, _c in enumerate("1234567890"):
    DIK[0x02 + _i] = _c
for _i, _c in enumerate("qwertyuiop"):
    DIK[0x10 + _i] = _c
for _i, _c in enumerate("asdfghjkl"):
    DIK[0x1e + _i] = _c
for _i, _c in enumerate("zxcvbnm"):
    DIK[0x2c + _i] = _c
for _i in range(10):
    DIK[0x3b + _i] = f"f{_i + 1}"

# ---------------------------------------------------------------- engine readers
# Each returns {action: set(keys)}.


def read_text(path):
    with open(path, "rb") as f:
        raw = f.read()
    if raw[:2] in (b"\xff\xfe", b"\xfe\xff"):
        return raw.decode("utf-16", errors="replace")
    return raw.decode("utf-8", errors="replace")


def add(d, action, key):
    if action and key:
        d.setdefault(action, set()).add(key)


def read_unreal(paths):
    """Unreal 1/2 User.ini: `[Engine.Input]` with `Key=Alias|Command`; later files replace earlier keys."""
    bykey = {}
    for p in paths:
        sec = None
        for line in read_text(p).splitlines():
            line = line.strip()
            if line.startswith("["):
                sec = line.lower()
            elif sec == "[engine.input]" and "=" in line and \
                    not line.lower().startswith(("aliases[", "mousex", "mousey", "mousew", "mousez")):
                k, v = line.split("=", 1)
                bykey[norm_unreal(k.strip())] = v.strip()
    out = {}
    for k, v in bykey.items():
        for part in v.split("|"):
            add(out, part.strip(), k)
    return out


def read_dark(paths):
    """Dark engine .bnd: `bind key[+mod...] command`; the last bind of a key wins across files."""
    bykey = {}
    for p in paths:
        for line in read_text(p).splitlines():
            m = re.match(r"\s*bind\s+(\S+)\s+(.*?)\s*$", line, re.I)
            if not m:
                continue
            spec, cmd = m.groups()
            parts = re.split(r"(?<=.)\+(?=.)", spec) if len(spec) > 1 else [spec]
            bykey["+".join(sorted(norm_dark(x) for x in parts))] = cmd.strip('"')
    out = {}
    for k, v in bykey.items():
        add(out, v, k)
    return out


def read_source(paths):
    """id Tech / GoldSrc / Source: `bind "key" "cmd"` in config.cfg, `"key" "cmd"` in kb_def.lst."""
    bykey = {}
    for p in paths:
        for line in read_text(p).splitlines():
            line = line.split("//")[0].strip()
            is_bind = line.lower().startswith("bind ")
            if not (is_bind or line.startswith('"')):
                continue
            m = re.match(r'(?:bind\s+)?("[^"]*"|\S+)\s+("[^"]*"|.+)$', line, re.I)
            if m:
                bykey[norm_source(m.group(1))] = m.group(2).strip().strip('"')
    out = {}
    for k, v in bykey.items():
        add(out, v, k)
    return out


def read_maxpayne(paths):
    """Max Payne 1/2 registry (Wine user.reg, `Input Settings`): DirectInput scancodes per action slot.
    Max Payne 1 names actions by number (`N`), 2 by name (`CROUCH`); `NAME.0`.. is one slot."""
    out = {}
    for p in paths:
        text = read_text(p)
        sections = re.findall(r"^\[Software\\\\Remedy Entertainment\\\\Max Payne[^\]]*\\\\Input Settings\][^\n]*\n(.*?)(?=^\[|\Z)",
                              text, re.S | re.M)
        vals = {}
        for sec in sections:
            for n, b, what, v in re.findall(r'^"\s*(\S+) Button(\d) (type|value)"=dword:([0-9a-f]+)', sec, re.M):
                vals[(n, int(b), what)] = int(v, 16)
        for (n, b, what), v in vals.items():
            if what != "value":
                continue
            t = vals.get((n, b, "type"), 0)
            if t == 1 and v in DIK:
                key = DIK[v]
            elif t == 2:
                key = {0: "mouse_left", 1: "mouse_right", 2: "mouse_middle"}.get(v)
            else:
                continue
            add(out, n, key)
            add(out, f"{n}.{b}", key)
    return out


def gdb_tokens(d):
    out, i, n = [], 0, len(d)

    def printable(b):
        return 32 <= b < 127

    while i < n:
        if printable(d[i]):
            if i + 3 < n and d[i + 1] == 0 and printable(d[i + 2]) and d[i + 3] == 0:
                s = ""
                while i + 1 < n and printable(d[i]) and d[i + 1] == 0:
                    s += chr(d[i])
                    i += 2
                out.append(s)
            else:
                j = i
                while j < n and printable(d[j]):
                    j += 1
                out.append(d[i:j].decode())
                i = j
        else:
            i += 1
    return out


def norm_fear(name):
    low = name.lower()
    m = re.fullmatch(r"button (\d)", low)
    if m:
        return {"0": "mouse_left", "1": "mouse_right", "2": "mouse_middle"}.get(m.group(1), low)
    return norm_generic(name)


FEAR_KEY = re.compile(r"^(.|space|tab|end|home|shift|control|alt|left|right|up|down|wheel|return|escape|"
                      r"backspace|delete|insert|f\d+|page ?up|page ?down|[xyz]-axis|button \d)$", re.I)


def read_fear(paths):
    """F.E.A.R. Profile000.gdb: strings, an action name then its key; `Mouse` precedes a mouse object."""
    out = {}
    for p in paths:
        with open(p, "rb") as f:
            toks = gdb_tokens(f.read())
        if "Bindings.0.Scale" not in toks:
            continue
        i = toks.index("Bindings.0.Scale") + 1
        if "Bindings.0.Object" in toks:  # the header holds the first action's (Activate's) key
            add(out, "Activate", norm_fear(toks[toks.index("Bindings.0.Object") + 1]))
        end = toks.index("Controls") if "Controls" in toks else len(toks)
        action = None
        for t in toks[i:end]:
            if t == "Mouse":
                continue
            if FEAR_KEY.match(t):
                if action is not None:
                    add(out, action, norm_fear(t))
                    action = None
            else:
                action = t
    return out


READERS = {"unreal": read_unreal, "dark": read_dark, "source": read_source, "maxpayne": read_maxpayne,
           "fear": read_fear}

# ---------------------------------------------------------------- locating game files


def find_files(spec):
    """spec['files'] are path patterns, read in order (later overrides earlier). Installed means the
    first pattern matches."""
    found = []
    for pat in spec["files"]:
        hits = sorted(glob.glob(os.path.expanduser(pat)))
        if hits:
            found.append(hits[0])
        elif not found:
            return []
    return found


def load_tables():
    with open(TABLES, "rb") as f:
        return tomllib.load(f)


def load_pack(name):
    path = name if name.endswith(".padpack") else os.path.join(ROOT, "packs", name + ".padpack")
    with open(path, "rb") as f:
        return tomllib.load(f)

# ---------------------------------------------------------------- comparison


def compare(spec, files):
    """Rows (status, pack, selector, label, pack keys, game keys, actions)."""
    ents = pack_entries(load_pack(spec["pack"]), spec.get("profile", 0))
    low = {k.lower(): v for k, v in READERS[spec["engine"]](files).items()}
    rows = []
    for item in spec["map"]:
        sel, actions = item["pack"], item["game"]
        if isinstance(actions, str):
            actions = [actions]
        path, found = resolve(ents, sel)
        gkeys = set()
        for a in actions:
            gkeys |= low.get(a.lower(), set())
        if not found:
            rows.append(("no-pack-input", spec["pack"], sel, sel, [], sorted(gkeys), actions))
            continue
        label, pkeys = found
        label = label or sel
        if not gkeys:
            status = "game-unbound"
        elif all(k in gkeys or ("&" in k and all(x in gkeys for x in k.split("&"))) for k in pkeys):
            status = "ok"
        else:
            status = "MISMATCH"
        rows.append((status, spec["pack"], path, label, pkeys, sorted(gkeys), actions))
    return rows


def show(game, rows, files, only_bad):
    print(f"== {game} ({rows[0][1]}.padpack) <- {', '.join(files)}")
    for st, _pack, path, label, pk, gk, acts in rows:
        if only_bad and st == "ok":
            continue
        print(f"  {st:13} {label!r:30} pack {','.join(pk) or '-':16} game {','.join(gk) or '-':22} "
              f"[{'|'.join(acts)}] ({path})")
    print(f"  {sum(r[0] == 'ok' for r in rows)}/{len(rows)} agree")


def self_test(_tables):
    with open(os.path.join(HERE, "binding-compare-selftest.toml"), "rb") as f:
        tables = tomllib.load(f)
    fails = 0
    for name, spec in tables["selftest"].items():
        files = [os.path.join(FIXTURES, f) for f in spec["files"]]
        got = READERS[spec["engine"]](files)
        for act, keys in spec["expect"].items():
            have = got.get(act, set())
            ok = set(keys) == have
            fails += not ok
            print(f"{'ok  ' if ok else 'FAIL'} {name}: {act} -> {sorted(have)} (want {sorted(keys)})")
    spec = tables["selftest_pack"]
    spec["pack"] = os.path.join(FIXTURES, spec["pack"])
    rows = compare(spec, [os.path.join(FIXTURES, f) for f in spec["files"]])
    want = spec["expect"]
    for r, w in zip(rows, want):
        ok = r[0] == w
        fails += not ok
        print(f"{'ok  ' if ok else 'FAIL'} pack compare {r[3]!r}: {r[0]} (want {w})")
    return 1 if fails else 0


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("cmd", choices=["list", "compare", "dump-pack", "dump-game", "self-test"])
    ap.add_argument("what", nargs="*")
    ap.add_argument("--file", action="append", default=[], help="binding file (repeat; read in order)")
    ap.add_argument("--all", action="store_true", help="show agreeing rows too")
    a = ap.parse_args()
    tables = load_tables()
    games = tables["game"]

    if a.cmd == "list":
        for g, s in games.items():
            f = find_files(s)
            print(f"{g:22} {s['engine']:9} {'installed' if f else 'not found':10} {', '.join(f)}")
        return 0
    if a.cmd == "dump-pack":
        for sel, (label, keys) in pack_entries(load_pack(a.what[0])).items():
            print(f"{sel:56} {label:28} {','.join(keys)}")
        return 0
    if a.cmd == "dump-game":
        s = games[a.what[0]]
        for act, ks in sorted(READERS[s["engine"]](a.file or find_files(s)).items()):
            print(f"{act:34} {','.join(sorted(ks))}")
        return 0
    if a.cmd == "self-test":
        return self_test(tables)
    bad = 0
    for g in a.what or games:
        if g not in games:
            sys.exit(f"unknown game {g}; try `list`")
        files = a.file or find_files(games[g])
        if not files:
            if a.what:
                print(f"== {g}: no game files found (pass --file)")
            continue
        rows = compare(games[g], files)
        show(g, rows, files, not a.all)
        bad += sum(r[0] == "MISMATCH" for r in rows)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
