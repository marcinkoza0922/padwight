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

def PS(name, ids, exes, theme, palette, n="", **kw):
    """Passthrough for a shooter: the plain profile plus a '+ Gyro' one that turns the view through a virtual right stick."""
    return P(name, ids, exes, theme, palette, n=n, gyro_pt=True, **kw)

# Mouse-and-keyboard shooters (also used by specs/batch10.py, which defines the same names for itself)
GUESS = "Typical defaults for the genre, not checked against the game."
NAT = " The game has some controller support of its own; this pack plays it with the pointer and keyboard keys for the parts the pad handles badly."
NOPAD = " The game has no controller support, so the pad plays it with the mouse and keyboard keys."
FPS_INTRO = "The left stick {ls} walks on WASD and the right stick {rs} looks around. {rt} fires and {lt} aims or fires the alternate mode. {up} and {down} change weapon on the wheel."
FB = {"South": ("k:SPACE", "Jump", "arrow"), "East": ("k:LEFTCTRL", "Crouch", None), "West": ("k:R", "Reload", "rotate"),
      "North": ("k:E", "Use", "hand"), "LeftBumper": ("k:Q", "Last weapon", None), "RightBumper": ("k:G", "Grenade", None),
      "LeftStick": ("k:LEFTSHIFT", "Sprint", None), "RightStick": ("k:V", "Melee", None), "Select": ("k:TAB", "Scoreboard", None)}
def F(name, ids, exes, theme, mood, b=None, n="", nat=False, **kw):
    """Mouse-and-keyboard shooters: stick look plus a gyro profile."""
    tail = f"The left stick walks on WASD and the right stick looks; RT fires, LT aims, the gyro aims finely. Overlays in {mood}."
    return G(name, ids, exes, "click", theme, tail, b={**FB, **(b or {})}, n=(n + " " if n else "") + GUESS, left="wasd", gyro=True,
             intro=FPS_INTRO, pad=NAT if nat else NOPAD, fps=True, rspeed=900.0, rtl="Fire", ltl="Aim / alt fire", **kw)
