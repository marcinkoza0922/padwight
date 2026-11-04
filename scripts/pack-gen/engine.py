import uuid, re, sys, os
BTN_TOK = {"South":"{south}","East":"{east}","West":"{west}","North":"{north}","LeftBumper":"{lb}","RightBumper":"{rb}",
  "LeftStick":"{l3}","RightStick":"{r3}","DpadUp":"{up}","DpadDown":"{down}","DpadLeft":"{left}","DpadRight":"{right}",
  "Start":"{start}","Select":"{select}"}
MODS = {"CTRL":"LEFTCTRL","SHIFT":"LEFTSHIFT","ALT":"LEFTALT"}
def act(s):
    t, _, v = s.partition(":")
    if s in ("mL","mR","mM"): return '{mouse = "%s"}' % {"mL":"Left","mR":"Right","mM":"Middle"}[s]
    if s in ("wU","wD"): return '{wheel = "%s"}' % {"wU":"Up","wD":"Down"}[s]
    if t == "k":
        ks = ['"KEY_%s"' % MODS.get(p, p) for p in v.split("+")]
        return "{keys = [%s]}" % ", ".join(ks)
    if t == "seq":  # several keys in order: seq:A,B
        return "{multi = [%s]}" % ", ".join(act("k:"+x) for x in v.split(","))
    if t == "menu": return '{open_menu = "%s"}' % v
    raise ValueError(s)

THEMES = {
 "parchment": dict(font="Cinzel", bg="#2b2118", item="#44331f", sel="#c9a24b", border="#b08d3c", grad="#1d1610", itemb="#6b5430", glow=None, corners=0.1, motion="stagger",
    snd=dict(step=("wood",0.07,1.0), pick=("page",0.09,1.0), open=("page",0.07,1.0), close=("wood",0.06,0.8), kbpick=("keys",0.12,1.0)), frame=("brackets",12,"#b08d3c"), case=None),
 "scifi": dict(font="Orbitron", bg="#0b1624", item="#14304a", sel="#35c6e8", border="#2a8fb0", grad="#07101b", itemb="#1f5a7a", glow="#6fe0ff", corners=0.05, motion="swift",
    snd=dict(step=("tick",0.06,1.2), pick=("blip",0.08,1.0), open=("pulse",0.06,1.0), close=("pulse",0.05,0.8), kbpick=("click",0.12,1.0)), frame=("chamfer",10,"#2a8fb0"), case="upper"),
 "calm": dict(font="Comfortaa", bg="#0f3a3a", item="#17504f", sel="#7ccf4a", border="#5fd0c0", grad="#0b2c2c", itemb="#2c6e6a", glow="#9ef0d8", corners=0.18, motion="playful",
    snd=dict(step=("marimba",0.07,1.1), pick=("chime",0.08,1.0), open=("pop",0.06,1.0), close=("pop",0.05,0.8), kbpick=("keys",0.14,1.0)), frame=None, case=None),
 "industrial": dict(font="Kreon", bg="#232323", item="#3b3631", sel="#e08a2e", border="#a8641c", grad="#171717", itemb="#5a4a38", glow=None, corners=0.04, motion="brutal",
    snd=dict(step=("clank",0.06,1.1), pick=("latch",0.09,1.0), open=("switch",0.07,1.0), close=("switch",0.06,0.8), kbpick=("keys",0.13,1.0)), frame=("riveted",0,None), case=None),
 "noir": dict(font="Special Elite", bg="#1c1a18", item="#34302b", sel="#c8b79a", border="#8a7f6c", grad="#121110", itemb="#524b41", glow=None, corners=0.03, motion="grim",
    snd=dict(step=("typewriter",0.05,1.2), pick=("typewriter",0.09,0.9), open=("page",0.06,1.0), close=("page",0.05,0.8), kbpick=("typewriter",0.12,1.0)), frame=None, case=None),
 "pixel": dict(font="Silkscreen", bg="#1a1a2e", item="#2b2b4d", sel="#f4c542", border="#7a7ae0", grad="#10101e", itemb="#3f3f78", glow=None, corners=0.0, motion="retro",
    snd=dict(step=("beep",0.05,1.3), pick=("blip",0.08,1.1), open=("blip",0.06,1.3), close=("blip",0.05,0.8), kbpick=("beep",0.1,1.0)), frame=("brackets",8,"#7a7ae0"), case=None),
 "military": dict(font="Rajdhani", bg="#1f2418", item="#343b27", sel="#b5c25a", border="#7d8a45", grad="#151910", itemb="#4d5736", glow=None, corners=0.03, motion="swift",
    snd=dict(step=("click",0.06,0.9), pick=("radio",0.09,1.0), open=("radio",0.07,1.0), close=("click",0.06,0.8), kbpick=("keys",0.13,1.0)), frame=("brackets",12,"#7d8a45"), case="upper"),
 "mystic": dict(font="Cinzel", bg="#1d1530", item="#33274f", sel="#b58cf0", border="#8661c9", grad="#130d22", itemb="#4e3b7a", glow="#c9a9ff", corners=0.12, motion="dreamy",
    snd=dict(step=("glass",0.06,1.1), pick=("bell",0.08,1.0), open=("chime",0.06,1.0), close=("chime",0.05,0.8), kbpick=("keys",0.12,1.0)), frame=None, case=None),
 "nature": dict(font="Kreon", bg="#1c2b1c", item="#2f4a2a", sel="#d4b84a", border="#7fb069", grad="#122012", itemb="#41643a", glow=None, corners=0.2, motion="subtle",
    snd=dict(step=("wood",0.06,1.1), pick=("pluck",0.09,1.0), open=("pluck",0.06,1.1), close=("wood",0.05,0.8), kbpick=("keys",0.12,1.0)), frame=None, case=None),
 "horror": dict(font="Creepster", bg="#1a0f12", item="#35181e", sel="#c23b3b", border="#7d2a2a", grad="#0e0709", itemb="#55252c", glow=None, corners=0.02, motion="grim",
    snd=dict(step=("thud",0.06,0.9), pick=("knock",0.09,1.0), open=("heartbeat",0.06,1.0), close=("thud",0.05,0.7), kbpick=("keys",0.12,0.9)), frame=None, case=None),
 "paper": dict(font="Inter", bg="#2a2a33", item="#3d3d4a", sel="#f0c05a", border="#9a9ab0", grad="#1e1e26", itemb="#555566", glow=None, corners=0.15, motion="subtle",
    snd=dict(step=("click",0.05,1.1), pick=("select",0.08,1.0), open=("pop",0.06,1.0), close=("pop",0.05,0.8), kbpick=("keys",0.12,1.0)), frame=None, case=None),
 "casino": dict(font="Bangers", bg="#13321f", item="#1f4e31", sel="#e8c547", border="#c9a227", grad="#0c2315", itemb="#2f6b45", glow=None, corners=0.15, motion="playful",
    snd=dict(step=("card",0.07,1.0), pick=("coin",0.09,1.0), open=("card",0.07,1.1), close=("card",0.06,0.8), kbpick=("keys",0.12,1.0)), frame=None, case=None),
}
def style_block(prefix, th, pos, scale, extras=True):
    o = []
    o.append(f"[{prefix}]\ncorners = {th['corners']}\nposition = \"{pos}\"\nscale = {scale}\n")
    o.append(f"[{prefix}.background]\ncolor = \"{th['bg']}\"\nopacity = 0.93\n")
    o.append(f"[{prefix}.items]\ncolor = \"{th['item']}\"\nopacity = 0.95\n")
    o.append(f"[{prefix}.selected]\ncolor = \"{th['sel']}\"\nopacity = 1.0\n")
    o.append(f"[{prefix}.border]\nwidth = 1\ncolor = \"{th['border']}\"\nopacity = 0.8\n")
    o.append(f"[{prefix}.gradient]\ncolor = \"{th['grad']}\"\nopacity = 1.0\nangle = 180\n")
    o.append(f"[{prefix}.item_border]\nwidth = 1\ncolor = \"{th['itemb']}\"\n")
    if extras and th["glow"]:
        o.append(f"[{prefix}.glow]\ncolor = \"{th['glow']}\"\nopacity = 0.45\nblur = 16\n")
    t = f"[{prefix}.type]\nletter_spacing = {1 if th['case'] else 0}\n"
    if th["case"]: t += f"case = \"{th['case']}\"\n"
    o.append(t)
    if extras:
        orn = ""
        if th["frame"] and th["frame"][2]:
            f = th["frame"]
            orn += f"frame = {{kind = \"{f[0]}\", size = {f[1]}, width = 2, color = \"{f[2]}\"}}\n"
        orn += f"dividers = {{width = 1, color = \"{th['itemb']}\", opacity = 0.8}}\n"
        o.append(f"[{prefix}.ornament]\n{orn}")
    return "\n".join(o)

DEFAULT_STICK_R = dict(speed=1400.0, accel=0.3)
def left_stick(mode):
    if mode == "none": return "[profiles.left_stick]\ndeadzone = 0.15\ncurve = 1.7\nkey_threshold = 0.35\n\n[profiles.left_stick.action.mouse]\nspeed = 450.0\naccel = 0.0\n\n"
    k = {"wasd":["W","D","S","A"], "arrows":["UP","RIGHT","DOWN","LEFT"]}[mode]
    def kk(*n): return "[" + ", ".join('{ keys = ["KEY_%s"] }' % x for x in n) + "]"
    sec = [("keys = [\"KEY_%s\"]" % k[0]), ("multi = %s" % kk(k[0],k[1])), ("keys = [\"KEY_%s\"]" % k[1]), ("multi = %s" % kk(k[2],k[1])),
           ("keys = [\"KEY_%s\"]" % k[2]), ("multi = %s" % kk(k[2],k[3])), ("keys = [\"KEY_%s\"]" % k[3]), ("multi = %s" % kk(k[0],k[3]))]
    o = "[profiles.left_stick]\ndeadzone = 0.15\ncurve = 1.0\nkey_threshold = 0.35\n\n[profiles.left_stick.action.ring]\nsectors = 8\nstart_angle = 0.0\ninner_radius = 0.35\nhysteresis = 0.1\n\n"
    for s in sec: o += "[[profiles.left_stick.action.ring.actions]]\n%s\n\n" % s
    return o

PT_BUTTONS = ["South","East","North","West","LeftBumper","RightBumper","LeftStick","RightStick","Select","Start","DpadUp","DpadDown","DpadLeft","DpadRight"]
GYRO_PT_COMBOS = 'combos = [\n    {buttons = ["Select", "Start"], action = "gyro_clutch"},\n    {buttons = ["Select", "RightStick"], action = "gyro_recenter"},\n]'
GYRO_PT_NOTE = ("Gyro aim, an extra profile for a shooter (the game's own scheme stays the default): tilting the controller turns the view through a virtual right stick, "
    "assumed to match a linear stick response with a 0.2 dead zone; change Game's curve and Anti-deadzone under Gyro if the view turns too far or too little. "
    "Hold {select} and {start} together to pause the gyro while you reposition the controller; {select} + {r3} recenters it.")
def profile_pt(g, gyro=False):
    """Passthrough: every control goes to the game as it is; only the Guide overlay sits on top. With gyro, a virtual right stick turns the view."""
    name = g["name"] + (" + Gyro" if gyro else "")
    combos = GYRO_PT_COMBOS if gyro else "combos = []"
    o = [f'[[profiles]]\nname = "{name}"\n{combos}\ncombo_window_ms = 120\ntap_window_ms = 250\nlong_press_ms = 500\n' + ('requires = ["gyro"]\n' if gyro else '') + '\n[profiles.buttons]']
    o += [f'{b} = {{gamepad = "{b}"}}' for b in PT_BUTTONS]
    o.append('Guide = {mode = "Guide"}\n')
    for side in ("left", "right"):
        o.append(f'[profiles.{side}_stick]\ndeadzone = 0.05\ncurve = 1.0\nkey_threshold = 0.3\n\n[profiles.{side}_stick.action]\ngamepad = {{stick = "{side.capitalize()}", invert_y = false}}\n')
    o.append('[profiles.left_trigger]\naction = {gamepad = "Left"}\n\n[profiles.right_trigger]\naction = {gamepad = "Right"}\n')
    if gyro:
        o.append('[profiles.gyro]\nhorizontal = "yaw"\ninvert_x = false\ninvert_y = false\nnoise_threshold = 1.0\n\n[profiles.gyro.mode.stick]\nstick = "Right"\nfull_rate = 360.0\nanti_deadzone = 0.2\ngame_curve = 1.0\n')
    else:
        o.append('[profiles.gyro]\nenabled = false\nhorizontal = "yaw"\ninvert_x = false\ninvert_y = false\nnoise_threshold = 1.0\n')
    notes = g['notes'] + ("\n" + GYRO_PT_NOTE if gyro else "")
    o.append(f"[profiles.guide]\nnotes = '''{notes}'''\n")
    return "\n".join(o) + "\n"

def profile(g, gyro):
    name = g["name"] + (" + Gyro" if gyro else "")
    B = g["buttons"]   # dict Button -> (action, label, icon)
    o = [f'[[profiles]]\nname = "{name}"']
    o.append('combos = [\n    {buttons = ["Select", "Start"], action = "gyro_clutch"},\n]' if gyro else "combos = []")
    o.append("combo_window_ms = 120\ntap_window_ms = 250\nlong_press_ms = 500")
    if gyro: o.append('requires = ["gyro"]')
    o.append("\n[profiles.buttons]\nGuide = {mode = \"Guide\"}")
    for b, (a, _l, _i) in B.items(): o.append(f"{b} = {act(a)}")
    if g.get("cyc"): o.append(f'{g["cyc"][0]} = {{cycle = {{name = "{g["cyc"][1]}"}}}}')
    if g.get("menu"): o.append(f'Select = {{open_menu = "{g["menu"][0]}"}}')
    o.append("")
    out = "\n".join(o) + "\n"
    for b, (a, _l, _i) in g.get("lp", {}).items():
        out += f"[profiles.gestures.{b}]\nlong_press = {act(a)}\n\n"
    out += left_stick(g["left"])
    rs = g.get("rspeed", 1400.0)
    out += f"[profiles.right_stick]\ndeadzone = 0.12\ncurve = 1.7\nkey_threshold = 0.5\n\n[profiles.right_stick.action.mouse]\nspeed = {rs}\naccel = 0.3\n\n"
    out += f"[profiles.left_trigger.action.button]\nthreshold = 0.4\naction = {act(g.get('lt','mR'))}\n\n"
    out += f"[profiles.right_trigger.action.button]\nthreshold = 0.3\naction = {act(g.get('rt','mL'))}\n\n"
    if gyro:
        out += "[profiles.gyro]\nhorizontal = \"yaw\"\ninvert_x = false\ninvert_y = false\nnoise_threshold = 1.0\n\n[profiles.gyro.mode.mouse]\nsensitivity = 20.0\n\n"
    else:
        out += "[profiles.gyro]\nenabled = false\nhorizontal = \"yaw\"\ninvert_x = false\ninvert_y = false\nnoise_threshold = 1.0\n\n"
    # notes
    L = []
    if gyro: L.append("Hold {select} and {start} together to pause the gyro while you reposition the controller. Tilting the controller " + ("aims, for fine adjustments next to the right stick {rs}." if g.get("fps") else "moves the pointer, as a fine pointer next to the right stick {rs}."))
    L.append(g["intro"])
    lines = []
    for b, (a, l, ic) in B.items():
        if l: lines.append(f"{BTN_TOK[b]} {l}")
    if g.get("cyc"): lines.append(f'{BTN_TOK[g["cyc"][0]]} {g["cyc"][3]}')
    for b, (a, l, ic) in g.get("lp", {}).items(): lines.append(f"hold {BTN_TOK[b]}: {l}")
    if g.get("menu"): lines.append(f"{{select}} lists {g['menu'][2]}")
    L.append("; ".join(lines).capitalize() if False else "Buttons: " + "; ".join(lines) + ".")
    L.append(g["notes"])
    L.append("Typing a name or text: {guide} + {west} opens the on-screen keyboard.")
    notes = "\n".join(L).replace("'''", "'")
    out += f"[profiles.guide]\nnotes = '''{notes}'''\n\n[profiles.guide.edits]\n"
    ed = ['"stick:Right" = {text = "%s"}' % ("Look" if g.get("fps") else "Pointer"), '"trigger:Right" = {text = "%s"}' % g.get("rtl","Left click"), '"trigger:Left" = {text = "%s"}' % g.get("ltl","Right click")]
    if g["left"] == "none": ed.insert(0, '"stick:Left" = {text = "Fine pointer"}')
    else: ed.insert(0, '"stick:Left" = {text = "%s"}' % ("Pan the view" if g["left"]=="wasd" and g["arch"]=="strat" else ("Move" if g["arch"]!="strat" else "Pan the view")))
    for b, (a, l, ic) in B.items():
        if b in ("DpadUp","DpadDown") and a in ("wU","wD"): continue
        if l: ed.append(f'"button:{b}" = {{text = "{(chr(123)+"icon:"+ic+chr(125)+" ") if ic else ""}{l}"}}')
    if g.get("cyc"): ed.append(f'"button:{g["cyc"][0]}" = {{text = "{g["cyc"][3]}"}}')
    for b, (a, l, ic) in g.get("lp", {}).items(): ed.append(f'"gesture:button:{b}:long_press" = {{text = "Hold: {l}"}}')
    if g.get("menu"): ed.append(f'"button:Select" = {{text = "{g["menu"][2].capitalize()}"}}')
    out += "\n".join(ed) + "\n"
    if B.get("DpadUp",("",))[0] in ("wU","wD") and B.get("DpadDown",("",))[0] in ("wU","wD"):
        out += f'\n[[profiles.guide.merged]]\ninputs = ["button:DpadUp", "button:DpadDown"]\ntext = "{g.get("zoomtext","Zoom in / out")}"\n'
    return out + "\n\n"

def build(g):
    th = THEMES[g["theme"]]
    slug = g["slug"]
    pid = str(uuid.uuid5(uuid.NAMESPACE_URL, "padwight-pack/" + slug))
    o = [f'format = 20\nmacros = []\noverlay_font = "{th["font"]}"\n']
    o.append(f'[pack]\nid = "{pid}"\nname = "{g["name"]}"\nversion = "1.0"\nauthor = "Padwight"\ndescription = "{g["desc"]}"\nmade_with = "Xbox controller"\n')
    for i in g["ids"]: o.append(f'[[rules]]\nkind = "steam_app_id"\nvalue = "{i}"\nprofile = "{g["name"]}"\n')
    for e in g["exes"]: o.append(f'[[rules]]\nkind = "executable"\nvalue = "{e}"\nprofile = "{g["name"]}"\n')
    if g.get("pt"):
        o.append(profile_pt(g))
        if g.get("gyro_pt"): o.append(profile_pt(g, True))
    else:
        o.append(profile(g, False))
        if g.get("gyro"): o.append(profile(g, True))
    if g.get("cyc"):
        steps = "\n".join(f"    {act(s)}," for s in g["cyc"][2])
        o.append(f'[[cycles]]\nname = "{g["cyc"][1]}"\nsteps = [\n{steps}\n]\n')
    if g.get("menu"):
        o.append(f'[[menus]]\nname = "{g["menu"][0]}"\nkind = "list"\n')
        for ic, lab, a in g["menu"][1]:
            o.append(f'[[menus.items]]\nlabel = "{{icon:{ic}}} {lab}"\naction = {act(a)}\n')
        o.append(style_block("menus.style", th, "center", 1.0))
    o.append(style_block("keyboard_style", th, "bottom_center", 1.0, False))
    o.append(style_block("numpad_style", th, "center", 1.0, False))
    o.append(style_block("media_style", th, "top_right", 0.9, False))
    o.append(style_block("menu_style", th, "center", 1.0))
    m = th["motion"]
    o.append("[motion]\n" + "".join(f'{k} = "{m}"\n' for k in ["keyboard","numpad","menu","media","offer","info","log"]))
    s = th["snd"]
    def sd(k): return f'{{kind = "{s[k][0]}", volume = {s[k][1]}, pitch = {s[k][2]}}}' if s[k][2] != 1.0 else f'{{kind = "{s[k][0]}", volume = {s[k][1]}}}'
    o.append(f'[sounds.menu]\nstep = {sd("step")}\npick = {sd("pick")}\nopen = {sd("open")}\nclose = {sd("close")}\n')
    o.append(f'[sounds.numpad]\nstep = {sd("step")}\npick = {sd("pick")}\n')
    o.append(f'[sounds.keyboard]\nstep = {sd("step")}\npick = {sd("kbpick")}\n')
    o.append(f'[sounds.media]\npick = {sd("pick")}\n')
    return "\n".join(o)

def write_all(specs, outdir):
    for g in specs:
        open(os.path.join(outdir, g["slug"] + ".padpack"), "w").write(build(g))
