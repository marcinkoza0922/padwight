"""Write library packs from compact specs: python3 scripts/pack-gen/run.py [--skip slug,...] specs/batchN.py ...
(slugs in specs/skip.txt are never written: specs written but left out.)

Each spec file defines S, a list of helpers.G(...) dicts; packs go to packs/<slug>.padpack.
See "Generating packs from specs" in docs/development/game-setup-guidelines.md.
"""
import importlib.util, os, sys
sys.dont_write_bytecode = True
here = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, here)
from engine import write_all

args = sys.argv[1:]
skip = []
if args[:1] == ["--skip"]:
    skip, args = args[1].split(","), args[2:]
specs = []
for path in args:
    spec = importlib.util.spec_from_file_location("specmod", path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    specs += mod.S
skipfile = os.path.join(here, "specs", "skip.txt")
if os.path.exists(skipfile):
    skip += open(skipfile).read().split()
specs = [g for g in specs if g["slug"] not in skip]
out = os.path.join(here, "..", "..", "packs")
write_all(specs, out)
print(f"wrote {len(specs)} packs to {os.path.normpath(out)}")
