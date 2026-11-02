#!/usr/bin/env python3
"""Writes the seed corpus for the fuzz targets (fuzz/corpus/<target>/), from the library packs,
the built-in icons, sounds and fonts, and the text of the debug scenes. Usage:
python3 -I scripts/fuzz-seeds.py [fuzz-folder]. The config_load seeds need the program itself;
scripts/fuzz.sh makes those."""
import hashlib
import pathlib
import re
import sys

root = pathlib.Path(__file__).resolve().parent.parent
fuzz = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else root / "fuzz"


def put(target, data, name=None):
    if isinstance(data, str):
        data = data.encode()
    folder = fuzz / "corpus" / target
    folder.mkdir(parents=True, exist_ok=True)
    (folder / (name or hashlib.sha1(data).hexdigest()[:16])).write_bytes(data)


for pack in sorted((root / "packs").glob("*.padpack")):
    put("pack_parse", pack.read_bytes(), pack.stem)

for svg in sorted((root / "assets/icons").glob("*.svg")):
    put("pack_assets", b"\x00" + svg.read_bytes(), "icon-" + svg.stem)
for wav in sorted((root / "assets/sounds").glob("*.wav")):
    put("pack_assets", b"\x01" + wav.read_bytes(), "sound-" + wav.stem)
for font in sorted((root / "assets/fonts").glob("*.[to]tf"))[:6]:
    put("pack_assets", b"\x02" + font.read_bytes(), "font-" + font.stem)
put("pack_assets", "icon-a <svg/>\nsound-a UklGRg==\nfont-a AAEAAAA=\n", "assets-lines")
put("pack_assets", b"\x03" + b"icon-a <svg/>\nsound-a UklGRg==\nfont-a AAEAAAA=\n")

# Text with tokens and formatting tags: every string in the packs and the debug scenes with a brace.
strings = set()
literal = re.compile(r'"((?:[^"\\\n]|\\.)*\{(?:[^"\\\n]|\\.)*)"')
sources = list((root / "packs").glob("*.padpack")) + list((root / "src/debug").rglob("*.rs"))
for source in sources:
    for found in literal.findall(source.read_text(errors="replace")):
        strings.add(found.replace('\\"', '"').replace("\\n", "\n").replace("\\\\", "\\"))
for text in sorted(strings)[:3000]:
    put("tokens", text)
put("tokens", "{b}{color:red}{icon:bow}{/b}{current_input_device_1:400:2000} {keyboard:a}{mouse:left}{dpad:up}")

requests = [
    '"Version"', '"Status"', '"GetConfig"', '"Reload"', '{"SetEnabled":true}', '{"SetProfile":"General"}',
    '"NextProfile"', '"WatchInput"', '{"TestRumble":"/dev/input/event3"}', '{"TestTurn":-40}',
    '{"OpenMenu":"Tools"}', '"CloseOverlays"', '{"WatchFeed":null}', '{"WatchFeed":"abc"}',
    '{"DebugAttach":{"model":null,"live":false}}',
    '{"DebugInput":{"path":"x","events":[{"Button":["South",true]},{"Axis":["LeftX",0.5]},{"Motion":{"gyro":[1,2,3],"ms":8}}]}}',
    '{"SetNintendoLayout":{"id":"a","on":true}}', '{"Activate":{"game":null,"profile":"General"}}',
    '{"Error":"x"}', '"Ok"', '{"Lines":["a","b"]}', '{"Attached":"/dev/x"}',
]
for i, line in enumerate(requests):
    put("ipc_request", line, f"request-{i:02}")

put("calculator", bytes([1, 10, 2, 12, 3, 18]), "sum")
put("calculator", bytes([5, 13, 0, 18]), "divide-by-zero")
put("calculator", bytes(range(20)) * 4, "every-key")
put("calculator", bytes([9] * 40 + [12] + [9] * 40 + [18]), "huge")

# Engine input: a profile byte, then (kind, argument) pairs; buttons press and release in turn.
import random

rng = random.Random(7)
for profile in range(4):
    put("engine_input", bytes([profile] + [rng.randrange(256) for _ in range(800)]), f"random-{profile}")
put("engine_input", bytes([3, 0, 1, 0, 0, 1, 2, 4, 3, 5, 3, 7, 3, 9, 3, 8, 0, 8, 0, 0, 0]), "busy-chords")
