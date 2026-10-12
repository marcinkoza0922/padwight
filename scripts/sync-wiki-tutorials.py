#!/usr/bin/env python3
"""Writes the manual's tutorials into wiki/ as wiki pages (Tutorial-*.md), with links and pictures rewritten:
chapter links become wiki page links (or links into the repository, for chapters the wiki has no page for), and
pictures become raw.githubusercontent.com URLs. Run it after editing docs/manual/tutorial-*.md, then commit wiki/."""
import re
from pathlib import Path

root = Path(__file__).resolve().parent.parent
repo = "https://github.com/marcinkoza0922/padwight"
raw = "https://raw.githubusercontent.com/marcinkoza0922/padwight/main/docs/images/manual/"

# The wiki's page for each manual chapter; chapters it has none for link into the repository instead.
pages = {
    "buttons.md": "Buttons-and-Actions", "sticks.md": "Sticks-and-Triggers", "gyro.md": "Gyro", "modes.md": "Modes",
    "layers.md": "Layers", "macros.md": "Macros", "menus.md": "Menus", "overlays.md": "On-Screen-Keyboard-and-Numpad",
    "guide-button.md": "Guide-Button", "in-game-menu.md": "In-Game-Menu", "sharing.md": "Packs-and-the-Library",
    "setups.md": "Games-and-Profiles", "troubleshooting.md": "Troubleshooting",
}
# Tutorial files and their wiki page names (the Deus Ex one keeps the name it always had).
ORDER = ["first-setup", "buttons", "sticks", "gyro", "touchpad", "modes", "layers", "macros", "menus", "overlays", "guide", "sharing", "deus-ex"]
files = sorted((root / "docs/manual").glob("tutorial-*.md"), key=lambda f: ORDER.index(f.stem.removeprefix("tutorial-")))
tutorials = {f.name: "Tutorial-" + "-".join(w.capitalize() for w in f.stem.removeprefix("tutorial-").split("-")) for f in files}
tutorials["tutorial-deus-ex.md"] = "Tutorial-Deus-Ex-Pack"


def link(match):
    target = match.group(2)
    if target.startswith(("http", "#")):
        return match.group(0)
    if target.startswith("../images/manual/"):
        return f"{match.group(1)}({raw}{target.rsplit('/', 1)[1]}{match.group(3)})"
    name = tutorials.get(target) or pages.get(target)
    url = name or f"{repo}/blob/main/docs/manual/{target}"
    return f"{match.group(1)}({url}{match.group(3)})"


names = []
for file, page in tutorials.items():
    text = (root / "docs/manual" / file).read_text()
    # Markdown links and images: [text](target "optional title")
    text = re.sub(r'(\]|!\[[^\]]*\])\(([^) "]+)((?: "[^"]*")?)\)', lambda m: link(m), text)
    text = text.replace("](../images", "](../images")
    (root / "wiki" / f"{page}.md").write_text(text)
    names.append((page, text.splitlines()[0].removeprefix("# ")))
    print(page)

sidebar = root / "wiki/_Sidebar.md"
side = sidebar.read_text()
block = "**Tutorials**\n" + "".join(f"- [{title.removeprefix('Tutorial: ')[0].upper()}{title.removeprefix('Tutorial: ')[1:]}]({page})\n" for page, title in names)
side = re.sub(r"\*\*Tutorials?\*\*\n(?:- .*\n)*", block, side)
sidebar.write_text(side)
