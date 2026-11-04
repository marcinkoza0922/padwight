from helpers import *
SPD = lambda b="RightBumper": (b, "Game speed", ["k:2", "k:3", "k:1"], "Next speed")
GUESS = "Typical defaults for the genre, not checked against the game."
S = []
NAT = " The game has some controller support of its own; this pack plays it with the pointer and keyboard keys for the parts the pad handles badly."
NOPAD = " The game has no controller support, so the pad plays it with the mouse and keyboard keys."
def X(name, ids, exes, arch, theme, mood, b=None, n="", left=None, cyc=None, tail=None, nat=True, **kw):
    """Pointer and keyboard games; nat=True when the game has some pad support of its own."""
    base = {"North": ("k:SPACE", "Pause", "pause")} if arch == "strat" else {}
    if not tail:
        if arch == "strat": tail = f"The left stick pans, the right stick points; RT and A click, LT right-clicks. Overlays in {mood}."
        elif left == "wasd": tail = f"The left stick walks on WASD and the right stick points; RT and A click, LT right-clicks. Overlays in {mood}."
        else: tail = f"The right stick points; RT and A click, LT right-clicks. Overlays in {mood}."
    return G(name, ids, exes, arch, theme, tail, b={**base, **(b or {})}, n=(n + " " if n else "") + GUESS, left=left, cyc=cyc, pad=NAT if nat else None, **kw)
FPS_INTRO = "The left stick {ls} walks on WASD and the right stick {rs} looks around. {rt} fires and {lt} aims or fires the alternate mode. {up} and {down} change weapon on the wheel."
FB = {"South": ("k:SPACE", "Jump", "arrow"), "East": ("k:LEFTCTRL", "Crouch", None), "West": ("k:R", "Reload", "rotate"),
      "North": ("k:E", "Use", "hand"), "LeftBumper": ("k:Q", "Last weapon", None), "RightBumper": ("k:G", "Grenade", None),
      "LeftStick": ("k:LEFTSHIFT", "Sprint", None), "RightStick": ("k:V", "Melee", None), "Select": ("k:TAB", "Scoreboard", None)}
def F(name, ids, exes, theme, mood, b=None, n="", nat=False, **kw):
    """Mouse-and-keyboard shooters: stick look plus a gyro profile."""
    tail = f"The left stick walks on WASD and the right stick looks; RT fires, LT aims, the gyro aims finely. Overlays in {mood}."
    return G(name, ids, exes, "click", theme, tail, b={**FB, **(b or {})}, n=(n + " " if n else "") + GUESS, left="wasd", gyro=True,
             intro=FPS_INTRO, pad=NAT if nat else NOPAD, fps=True, rspeed=900.0, rtl="Fire", ltl="Aim / alt fire", **kw)
JUMP = {"North": ("k:SPACE", "Jump", "arrow"), "West": ("k:E", "Use", "hand")}
VEH = {"West": ("k:E", "Enter vehicle", "car")}
CHAR = {"West": ("k:C", "Character", "person"), "LeftBumper": ("k:I", "Inventory", "bag"), "RightBumper": ("k:M", "Map", "map")}
INV = {"West": ("k:E", "Use", "hand"), "LeftBumper": ("k:TAB", "Inventory", "bag"), "RightBumper": ("k:M", "Map", "map")}
ADV = {"North": ("k:ENTER", "Advance", "arrow")}
PSF = {"South": ("k:SPACE", "Jump", "arrow")}
S += [
# --- MMOs and online RPGs
X("Path of Exile", ["238960"], ["pathofexile_x64steam.exe"], "click", "mystic", "exile violet and gold", b={"West": ("k:I", "Inventory", "bag"), "LeftBumper": ("k:C", "Character", "person"), "RightBumper": ("k:P", "Passive tree", "map")}, n="Flasks are on 1 to 5 and skills on Q, W, E, R, T; the panel keys are the defaults as remembered."),
X("Path of Exile 2", ["2694490"], [], "click", "mystic", "ruin violet and bone", left="wasd", b=CHAR, n="Move by click or on WASD; skills and flasks sit on the mouse and number keys."),
X("Lost Ark", ["1599340"], ["lostark.exe"], "click", "parchment", "Arkesia gold", b=CHAR, n="Move by clicking the ground; skills are on Q, W, E, R, A, S, D, F."),
X("Black Desert", ["582660"], [], "click", "parchment", "desert sandstone", left="wasd", b=CHAR, n="WASD moves and Space jumps; the pad leaves combos to the mouse buttons."),
X("Trove", ["304050"], [], "click", "pixel", "voxel cyan", left="wasd", b=JUMP),
X("Temtem", ["745920"], [], "click", "nature", "island teal", left="wasd", b={"West": ("k:E", "Interact", "hand"), "LeftBumper": ("k:I", "Inventory", "bag")}),
X("Star Trek Online", ["9900"], [], "click", "scifi", "LCARS orange and blue", left="wasd", b={"West": ("k:F", "Interact", "hand"), "RightBumper": ("k:M", "Map", "map")}),
X("Spiral Knights", ["99900"], [], "click", "pixel", "clockwork blue", left="wasd", b={"South": ("k:SPACE", "Dodge", "arrow")}),
X("Phantasy Star Online 2 New Genesis", ["1056640"], [], "click", "scifi", "ARKS neon", left="wasd", b=CHAR),
X("APB Reloaded", ["113400"], [], "click", "industrial", "city grey and orange", left="wasd", b={"West": ("k:F", "Use", "hand")}),
# --- survival and sandbox
X("The Forest", ["242760"], [], "click", "nature", "pine green", left="wasd", b={**INV, "South": ("k:SPACE", "Jump", "arrow")}),
X("DayZ", ["221100"], [], "click", "military", "Chernarus drab", left="wasd", b={**INV, "East": ("k:C", "Crouch", None)}, nat=False),
X("Subnautica", ["264710"], [], "click", "calm", "ocean teal", left="wasd", b={"South": ("k:SPACE", "Swim up", "arrow"), "West": ("k:E", "Use", "hand"), "LeftBumper": ("k:TAB", "PDA", "book")}),
X("Green Hell", ["815370"], [], "click", "nature", "jungle green", left="wasd", b={**INV, "LeftBumper": ("k:TAB", "Inventory", "bag")}),
X("Icarus", ["1149460"], [], "click", "scifi", "frontier orange", left="wasd", b=INV),
X("Atlas", ["834910"], [], "click", "nature", "pirate sea blue", left="wasd", b=INV, nat=False),
X("Empyrion - Galactic Survival", ["383120"], [], "click", "scifi", "hull grey and cyan", left="wasd", b=INV, nat=False),
X("Core Keeper", ["1621690"], [], "click", "pixel", "cavern green", left="wasd", b={"West": ("k:E", "Interact", "hand"), "LeftBumper": ("k:TAB", "Inventory", "bag")}),
X("Miscreated", ["299740"], [], "click", "military", "wasteland khaki", left="wasd", b=INV, nat=False),
X("Last Oasis", ["903950"], [], "click", "nature", "dune sand", left="wasd", b=INV),
X("The Isle", ["376210"], [], "click", "nature", "primeval green", left="wasd", b={"West": ("k:E", "Interact", "hand")}),
X("Don't Starve", ["219740"], [], "click", "horror", "gaslight sepia", left="wasd", b={"West": ("k:SPACE", "Act", "hand"), "LeftBumper": ("k:TAB", "Craft", "hammer"), "RightBumper": ("k:M", "Map", "map")}, n="Space acts and the mouse is also needed for selecting."),
X("My Summer Car", ["516750"], [], "click", "industrial", "Finnish garage grey", left="wasd", b={"West": ("k:F", "Use", "hand"), "RightBumper": ("k:C", "Crouch", None)}),
X("Blade & Sorcery", ["629730"], [], "click", "mystic", "ember red", left="wasd", b=PSF, nat=False),
X("Hydroneer", ["1106840"], [], "click", "industrial", "mine amber", left="wasd", b={"South": ("k:SPACE", "Jump", "arrow"), "West": ("k:E", "Use", "hand")}),
X("Sun Haven", ["1432860"], [], "click", "nature", "meadow pastels", left="wasd", b={"West": ("k:E", "Interact", "hand"), "LeftBumper": ("k:TAB", "Inventory", "bag")}),
# --- vehicles and simulators
X("SnowRunner", ["1465360"], [], "click", "industrial", "mud brown and orange", left="wasd", b={"West": ("k:ENTER", "Switch vehicle", "car"), "North": ("k:SPACE", "Handbrake", "stop")}, n="The game plays well on its own pad; this pack is for the map and menu parts."),
X("American Truck Simulator", ["270880"], ["amtrucks.exe"], "click", "industrial", "Route 66 red and chrome", left="wasd", b={"North": ("k:SPACE", "Handbrake", "stop"), "West": ("k:F", "Park brake / hook", "hand"), "RightBumper": ("k:F1", "Camera", "eye")}),
X("Universe Sandbox", ["230290"], [], "click", "scifi", "deep-space blue", left="wasd", b={"North": ("k:SPACE", "Pause", "pause"), "West": ("k:T", "Time controls", "hourglass")}),
X("Microsoft Flight Simulator X", ["314160"], ["fsx.exe"], "click", "scifi", "cockpit blue", left="arrows", b={"North": ("k:P", "Pause", "pause"), "West": ("k:G", "Gear", "wrench"), "LeftBumper": ("k:F", "Flaps down", None), "RightBumper": ("k:V", "Next view", "eye")}),
X("PlateUp!", ["1599600"], [], "click", "casino", "diner red and yellow", left="wasd", b={"West": ("k:E", "Grab / drop", "hand"), "South": ("k:SPACE", "Interact", "hand")}),
X("Fishing Planet", ["380600"], [], "click", "nature", "lake blue", left="wasd", b={"West": ("k:E", "Use", "hand"), "LeftBumper": ("k:TAB", "Rods", "bag")}),
X("Zoo Tycoon: Ultimate Animal Collection", ["613880"], [], "strat", "nature", "savanna gold", cyc=SPD(), nat=False),
# --- strategy and tactics
X("Wasteland 3", ["719040"], [], "strat", "industrial", "Colorado ice and rust", b={"West": ("k:TAB", "Next squad member", "person")}, n="Space pauses real-time moves; Tab cycles characters."),
X("Library of Ruina", ["1256670"], [], "click", "noir", "library grey and gold"),
X("Dungeon Defenders II", ["236110"], [], "click", "mystic", "crystal blue and gold", left="wasd", b={"West": ("k:E", "Interact", "hand"), "North": ("k:TAB", "Build menu", "hammer")}),
X("Railroad Tycoon 3", ["7610"], [], "strat", "industrial", "locomotive brown", nat=False, cyc=SPD()),
X("Cossacks: European Wars", ["4880"], [], "strat", "parchment", "musket gold", nat=False),
X("Etherlords", ["270770"], [], "click", "mystic", "ether violet", nat=False),
X("Totally Accurate Battle Simulator", ["508440"], [], "click", "casino", "wobbly yellow", left="wasd", b={"North": ("k:SPACE", "Start battle", "arrow"), "West": ("k:TAB", "Next unit", "person")}),
X("Ravenfield", ["636480"], [], "click", "military", "battlefield green", left="wasd", b={**FB, "West": ("k:R", "Reload", "rotate")}, nat=False),
X("Robocraft", ["301520"], [], "click", "industrial", "robot orange", left="wasd", b=PSF, nat=False),
X("Crossout", ["386180"], [], "click", "industrial", "wasteland rust", left="wasd", b={"South": ("k:SPACE", "Handbrake", "stop")}, nat=False),
# --- adventure, RPG, visual novels
X("Steins;Gate", ["412830"], [], "click", "mystic", "phone-trigger green", b=ADV),
X("Final Fantasy VII (2012)", ["39140"], [], "click", "mystic", "Mako green", left="arrows", b={"South": ("k:ENTER", "Confirm", "arrow"), "East": ("k:ESC", "Cancel", None)}, n="This is the 2012 Steam release; arrows move and Enter confirms.", nat=False),
X("Final Fantasy X/X-2 HD Remaster", ["359870"], [], "click", "mystic", "Spira blue", left="arrows", b={"South": ("k:ENTER", "Confirm", "arrow")}, n="Arrows move and Enter confirms."),
X("Persona 5 Royal", ["1687950"], [], "click", "noir", "phantom red and black", left="wasd", b={"South": ("k:ENTER", "Confirm", "arrow"), "West": ("k:Q", "Map / third eye", "map")}),
X("South Park: The Fractured but Whole", ["488790"], [], "click", "casino", "Colorado paper cut-out", left="wasd", b={"West": ("k:E", "Interact", "hand")}),
X("GuJian 3", ["994280"], [], "click", "parchment", "ink-wash jade", left="wasd", b={"West": ("k:F", "Interact", "hand")}),
X("Return to Monkey Island", ["2060130"], [], "click", "calm", "Caribbean turquoise", n="Left click uses an item, right click looks."),
X("Tales of Monkey Island", ["31170"], [], "click", "calm", "Caribbean turquoise", nat=False),
X("Buckshot Roulette", ["2835570"], [], "click", "horror", "nightclub green", n="Everything is a click on the table."),
X("OneShot", ["420530"], [], "click", "pixel", "sun amber", left="arrows", b={"South": ("k:ENTER", "Confirm", "arrow")}),
X("Hand Simulator", ["657200"], [], "click", "casino", "slapstick yellow", left="wasd"),
X("Devour", ["1274570"], [], "click", "horror", "cult red", left="wasd", b={"West": ("k:E", "Interact", "hand")}),
X("Content Warning", ["2881650"], [], "click", "horror", "tape-grain red", left="wasd", b={"West": ("k:E", "Use", "hand")}),
X("Inside the Backrooms", ["1987080"], [], "click", "horror", "fluorescent yellow", left="wasd", b={"West": ("k:E", "Interact", "hand")}),
X("20 Minutes Till Dawn", ["1966900"], [], "click", "horror", "night violet", left="wasd", b={"South": ("k:SPACE", "Dash", "arrow")}),
X("Soulstone Survivors", ["2066020"], [], "click", "mystic", "soul teal", left="wasd"),
X("HoloCure - Save the Fans!", ["2420510"], [], "click", "pixel", "idol pink", left="wasd"),
# --- mouse-and-keyboard shooters
F("Alien Swarm", ["630"], [], "scifi", "hazard orange", n="Top-down; the right stick aims the crosshair."),
F("Team Fortress Classic", ["20"], [], "military", "team red and blue", n="Class keys 1 to 9 are on the keyboard."),
F("Deathmatch Classic", ["40"], [], "military", "arena grey"),
F("Quake Live", ["282440"], [], "industrial", "arena rust and steel", b={"South": ("k:SPACE", "Jump", "arrow")}),
F("Quake 4", ["2210"], [], "scifi", "Strogg steel", b={"RightBumper": ("k:F", "Flashlight", None)}),
F("Brink", ["22350"], [], "industrial", "Ark steel and teal", b={"RightBumper": ("k:Q", "Objective wheel", None)}),
F("Zombie Panic! Source", ["17500"], [], "horror", "blood red", b={"North": ("k:E", "Use", "hand")}),
F("Call of Juarez", ["3020"], [], "parchment", "frontier sepia"),
F("Sniper Elite", ["3700"], [], "military", "Berlin 1945 grey", b={"RightStick": ("k:LEFTSHIFT", "Hold breath", None)}),
F("Red Faction", ["20530"], [], "industrial", "Mars rust", b={"RightBumper": ("k:F", "Scope", None)}),
F("Painkiller", ["39530"], [], "horror", "hell red", b={"RightBumper": ("k:F", "Taunt", None)}),
F("Red Orchestra: Ostfront 41-45", ["1200"], [], "military", "Eastern Front grey", b={"RightStick": ("k:LEFTSHIFT", "Sprint", None), "LeftStick": ("k:C", "Prone", None)}),
F("Tom Clancy's Rainbow Six 3: Raven Shield", ["19830"], [], "military", "team black and steel", b={"North": ("k:SPACE", "Go / use", "hand")}),
F("Brothers in Arms: Road to Hill 30", ["15190"], [], "military", "Normandy olive", b={"North": ("k:SPACE", "Order squad", "person")}),
F("Sniper: Art of Victory", ["271500"], [], "military", "field olive"),
F("NecroVisioN", ["7860"], [], "horror", "trench mud and red"),
F("Nosferatu: The Wrath of Malachi", ["283290"], [], "horror", "gothic crimson"),
F("Call of Duty", ["2620"], [], "military", "Allied olive", n="The original 2003 game; its Steam app also holds the United Offensive expansion."),
F("Cruelty Squad", ["1388770"], [], "horror", "gore pink and green", n="WASD moves; the pad's triggers fire and aim."),
F("Hired Ops", ["374280"], [], "military", "arena khaki"),
F("Double Action: Boogaloo", ["317360"], [], "casino", "action-movie red", b={"East": ("k:LEFTCTRL", "Crouch", None), "South": ("k:SPACE", "Dive / jump", "arrow")}),
F("Gotham City Impostors", ["206210"], [], "casino", "comic purple and green"),
F("Tribes: Ascend", ["17080"], [], "scifi", "skiing blue and orange", b={"South": ("k:SPACE", "Ski", "arrow")}),
F("Devil Daggers", ["422970"], [], "horror", "dagger red", n="Fire and jump are all there is."),
F("Dirty Bomb", ["333930"], [], "industrial", "London grey and orange"),
F("Black Squad", ["550650"], [], "military", "squad blue"),
F("Survarium", ["355840"], [], "military", "Chernobyl drab"),
F("Block N Load", ["299360"], [], "pixel", "block blue and red"),
F("Ballistic Overkill", ["296300"], [], "industrial", "arena orange"),
F("Intruder", ["518150"], [], "noir", "stealth grey"),
F("Red Crucible: Firestorm", ["298240"], [], "scifi", "crucible red"),
F("Alliance of Valiant Arms", ["224244"], [], "military", "desert tan"),
F("Line of Sight", ["436520"], [], "military", "tactical grey"),
F("Codename CURE", ["355180"], [], "horror", "outbreak green"),
F("One-armed robber", ["2551020"], [], "casino", "heist gold"),
F("Due Process", ["753650"], [], "noir", "tactical steel"),
F("GRAV", ["332500"], [], "scifi", "survival grey"),
F("Ace of Spades: Battle Builder", ["224540"], [], "pixel", "voxel olive"),
F("Freeman: Guerrilla Warfare", ["773951"], [], "military", "guerrilla green"),
F("Marauders", ["1789480"], [], "scifi", "pirate steel"),
]
