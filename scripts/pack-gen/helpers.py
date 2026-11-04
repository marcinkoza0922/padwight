import re
def slugify(n):
    s = re.sub(r"[^a-z0-9]+", "-", n.lower().replace("'", "")).strip("-")
    return s
STRAT_INTRO = "The right stick {rs} moves the pointer and the left stick {ls} pans the view. {rt} or {south} clicks, {lt} right-clicks (orders, cancel). Scroll zooms on {up} and {down}."
CLICK_INTRO = "The right stick {rs} moves the pointer and the left stick {ls} nudges it slowly for fine work. {rt} or {south} clicks, {lt} right-clicks. {up} and {down} scroll the wheel."
KEYNOTE = "Keys are the game's defaults as remembered, not read from an install; if one doesn't answer, check the game's controls options and change the buttons here to match."
def G(name, ids, exes, arch, theme, tail, b=None, lp=None, cyc=None, menu=None, n="", left=None, gyro=None, slug=None, intro=None, desc=None, pad=None, exenote=None, **kw):
    base = {"South": ("mL", "Click", None), "East": ("k:ESC", "Back / close", None), "Start": ("k:ESC", "Menu", None),
            "DpadUp": ("wU", None, None), "DpadDown": ("wD", None, None)}
    if arch == "click": base["East"] = ("mR", "Right-click / back", None); base.pop("Start"); base["Start"] = ("k:ESC", "Menu", None)
    bb = dict(base)
    for k, v in (b or {}).items():
        if v is None: bb.pop(k, None)
        else: bb[k] = v
    if left is None: left = "wasd" if arch == "strat" else "none"
    if gyro is None: gyro = arch == "strat"
    if intro is None and arch == "click" and left != "none": intro = "The right stick {rs} moves the pointer. {rt} or {south} clicks, {lt} right-clicks. {up} and {down} scroll the wheel. The left stick {ls} moves."
    exe_note = (exenote if exenote is not None else " The Windows executable is from Discord's list.") if exes else ""
    notes = (n + " " if n else "") + KEYNOTE + exe_note
    if arch == "strat" or arch == "click":
        notes += pad or " The game has no controller support, so the pad plays it with the pointer."
    g = dict(name=name, slug=slug or slugify(name), ids=ids, exes=exes, arch=arch, theme=theme, buttons=bb, lp=lp or {}, cyc=cyc, menu=menu,
             left=left, gyro=gyro, notes=notes, intro=intro or (STRAT_INTRO if arch == "strat" else CLICK_INTRO),
             desc=(desc or tail).replace('"', "'"))
    g["desc"] = f"{name} on an Xbox-style pad. " + g["desc"]
    g.update(kw)
    return g

PT_NOTE = "The game has its own controller support, so the controller passes through untouched and the game's own options set the buttons. {guide} still works over the game: {guide} + {west} opens the on-screen keyboard for names and text, and {guide} + {lb} the media controls."
def P(name, ids, exes, theme, palette, n="", **kw):
    """Passthrough pack for a game with native controller support (see PT_NOTE). `palette` is for the author only: the theme sets the look."""
    notes = (n + " " if n else "") + PT_NOTE
    desc = f"{name} with its own controller support. The controller passes through to the game untouched; the game's options set the buttons. Padwight adds the Guide overlay in the game's colors.".replace('"', "'")
    g = dict(name=name, slug=slugify(name), ids=ids, exes=exes, arch="pt", theme=theme, buttons={}, lp={}, cyc=None, menu=None,
             left="none", gyro=False, notes=notes, intro="", desc=desc, pt=True)
    g.update(kw)
    return g
def KB(name, ids, exes, theme, palette, n="", **kw):
    """Keyboard-only game (its pad support predates XInput, so the pad sends keys): arrows walk, face buttons act."""
    b = {"South": ("k:Z", "Confirm", None), "East": ("k:X", "Cancel", None), "West": ("k:C", "Action", None), "DpadUp": None, "DpadDown": None}
    intro = "The arrow keys move. {south} confirms, {east} cancels, {west} acts, and {start} opens the menu."
    return G(name, ids, exes, "click", theme, "The game's own pad support predates XInput, so the pad sends its keyboard keys: arrows walk and the face buttons act.", b=b, left="arrows", intro=intro,
             n=(n + " " if n else "") + "The keys are the usual defaults for the series, not checked on an install.",
             pad=" The game predates XInput, so the pad sends the keys its controls use.", **kw)
