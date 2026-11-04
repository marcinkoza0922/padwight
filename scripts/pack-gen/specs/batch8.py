from helpers import *
S = []
SPD = lambda b="RightBumper": (b, "Game speed", ["k:2", "k:3", "k:1"], "Next speed")
PAUSE = {"North": ("k:SPACE", "Pause", "pause")}
OLD = "The game predates controller support and was played with the mouse; hotkeys are the ones the era's games shared. "
GUESS = "Typical defaults for the genre, not checked against the game."
def R(name, ids, exes, theme, mood, n="", **kw):
    """Real-time strategy: pan with the left stick, point with the right."""
    b = {**PAUSE, "West": ("k:A", "Attack-move", "attack"), "LeftBumper": ("k:H", "Select base", "home"), "RightBumper": ("k:CTRL+A", "Select all", "formation"), **kw.pop("b", {})}
    return G(name, ids, exes, "strat", theme, f"The left stick pans, the right stick points; RT and A click, LT right-clicks. Overlays in {mood}.", b=b, n=(n + " " if n else "") + "Hotkeys are guesses from the genre. " + GUESS, **kw)
def B(name, ids, exes, theme, mood, n="", **kw):
    """Builders and life sims: speed cycle and the build keys."""
    b = {**PAUSE, **kw.pop("b", {})}
    return G(name, ids, exes, "strat", theme, f"The left stick pans, the right stick points; RT and A click, LT right-clicks. Y pauses and the right bumper steps the speed. Overlays in {mood}.", b=b, cyc=kw.pop("cyc", SPD()), n=(n + " " if n else "") + "Space to pause and 1 to 3 for speed are guesses from similar games.", **kw)
def T(name, ids, exes, theme, mood, n="", **kw):
    """Turn-based games on the pointer."""
    b = {"North": ("k:ENTER", "End turn", "hourglass"), **kw.pop("b", {})}
    return G(name, ids, exes, "strat", theme, f"The left stick pans, the right stick points; RT and A click, LT right-clicks. Y ends the turn. Overlays in {mood}.", b=b, n=(n + " " if n else "") + "Enter to end the turn is a guess. " + GUESS, **kw)
def C(name, ids, exes, theme, mood, n="", **kw):
    """Point-and-click and mouse-driven games."""
    return G(name, ids, exes, "click", theme, f"The right stick points; RT and A click, LT right-clicks, the wheel scrolls on the d-pad. Overlays in {mood}.", n=n or "A mouse-driven game; no keys are mapped beyond the mouse.", **kw)
def W(name, ids, exes, theme, mood, n="", **kw):
    """Walk with the left stick, point or look with the right (third person, online, survival)."""
    b = {"North": ("k:SPACE", "Jump", "arrow"), "West": ("k:E", "Use", "hand"), "LeftBumper": ("k:TAB", "Map / list", "map"), "RightBumper": ("k:I", "Inventory", "bag"), **kw.pop("b", {})}
    return G(name, ids, exes, "click", theme, f"The left stick walks on WASD and the right stick points or looks; RT and A click, LT right-clicks. Overlays in {mood}.", left="wasd", b=b, n=(n + " " if n else "") + "Space, E, Tab and I are common defaults. " + GUESS, **kw)
def D(name, ids, exes, theme, mood, n="", **kw):
    """Diablo-style action RPGs: the mouse does the work, the shoulders open screens."""
    b = {"North": ("k:SHIFT", "Hold position", "fortify"), "West": ("k:C", "Character", "person"), "LeftBumper": ("k:I", "Inventory", "bag"), "RightBumper": ("k:TAB", "Map", "map"), **kw.pop("b", {})}
    return G(name, ids, exes, "click", theme, f"The right stick points; RT and A click, LT right-clicks (the second skill). Overlays in {mood}.", b=b, n=(n + " " if n else "") + "C, I and Tab are the usual screens. " + GUESS, **kw)
S += [
# Real-time strategy
R("The Lord of the Rings: The Battle for Middle-earth", [], [], "mystic", "Middle-earth gold and shadow", n=OLD),
R("The Lord of the Rings: The Battle for Middle-earth II", [], [], "mystic", "Middle-earth gold and shadow", n=OLD),
R("Age of Empires II: The Age of Kings", [], [], "parchment", "stone and banner red", n=OLD + "Pause is F3 in this game, not Space.", b={"North": ("k:F3", "Pause", "pause")}),
R("Age of Empires", [], [], "parchment", "sand and bronze", n=OLD, b={"North": ("k:F3", "Pause", "pause")}),
R("Age of Mythology", [], [], "mystic", "Olympian marble and blue", n=OLD + "The Extended Edition has its own pack."),
R("Dune II: The Building of a Dynasty", [], [], "industrial", "spice orange and rust", n=OLD),
R("Dune 2000", [], [], "industrial", "spice orange and rust", n=OLD),
R("Emperor: Battle for Dune", [], [], "industrial", "spice orange and rust", n=OLD),
R("Warcraft II: Battle.net Edition", [], [], "parchment", "Azeroth iron and crimson", n=OLD),
R("Warcraft: Orcs & Humans", [], [], "pixel", "early-90s pixel blue", n=OLD),
R("Rise of Nations", [], [], "parchment", "empire brown and gold", n=OLD),
R("Rise of Nations: Rise of Legends", [], [], "mystic", "steampunk copper and violet", n=OLD),
R("The Settlers III", [], [], "nature", "meadow green", n=OLD),
R("The Settlers IV", [], [], "nature", "meadow green", n=OLD),
R("The Settlers II: Veni, Vidi, Vici", [], [], "nature", "meadow green", n=OLD),
R("Star Trek: Armada", [], [], "scifi", "LCARS orange and blue", n=OLD),
R("Star Trek: Armada II", [], [], "scifi", "LCARS orange and blue", n=OLD),
R("Star Trek: New Worlds", [], [], "scifi", "LCARS orange and blue", n=OLD),
R("Total Annihilation: Kingdoms", [], [], "military", "fantasy-war green and steel", n=OLD),
R("ParaWorld", [], [], "nature", "jungle green and amber", n=OLD),
R("Homeworld: Cataclysm", [], [], "scifi", "deep-space blue", n=OLD + "Space is a guess for pausing.", b={"LeftBumper": ("k:F", "Focus", "eye")}),
R("Lego Rock Raiders", [], [], "paper", "brick yellow and red", n=OLD),
R("Hammer & Sickle", [], [], "military", "Soviet khaki and red", n=OLD),
# Builders and life sims
B("SimCity 2000", [], [], "paper", "city-plan blue and grey", n=OLD),
B("Theme Hospital", [], [], "calm", "hospital teal", n=OLD),
B("Theme Park World", [], [], "calm", "funfair pastels", n=OLD),
B("Zoo Tycoon", [], [], "nature", "safari green", n=OLD),
B("Monopoly Tycoon", [], [], "casino", "board-game green and red", n=OLD),
B("Alex Builds His Farm", [], [], "nature", "farm green", n=OLD),
B("Black & White", [], [], "mystic", "godly gold and violet", n=OLD + "The camera is steered with the mouse in this game."),
B("Black & White 2", [], [], "mystic", "godly gold and violet", n=OLD + "The camera is steered with the mouse in this game."),
B("The Sims", [], [], "calm", "plumbob green", n=OLD),
B("The Sims 2", [], [], "calm", "plumbob green", n=OLD),
B("The Sims Castaway Stories", [], [], "nature", "island green and sea blue", n=OLD),
B("The Sims Life Stories", [], [], "calm", "plumbob green", n=OLD),
B("The Sims Pet Stories", [], [], "calm", "plumbob green", n=OLD),
B("Supermarket Together", ["2709570"], ["supermarket together.exe"], "calm", "shop-floor pastels", n="Online co-op shop sim."),
B("TerraTech", ["285920"], ["terratechwin32.exe"], "industrial", "scrap orange", n="Block-building and driving on WASD is a guess."),
B("Football Manager 2021", ["1263850"], [], "paper", "touchline grey and green", n="A menu-driven management game: the pointer does everything."),
B("Football Manager 2024", ["2252570"], [], "paper", "touchline grey and green", n="A menu-driven management game: the pointer does everything."),
# Turn-based
T("Heroes of Might and Magic: A Strategic Quest", [], [], "parchment", "castle crimson", n=OLD),
T("Heroes of Might and Magic II", [], [], "parchment", "castle crimson", n=OLD),
T("Heroes of Might and Magic IV", [], [], "parchment", "castle crimson", n=OLD),
T("Sid Meier's Civilization II", [], [], "paper", "atlas blue and gold", n=OLD),
T("Civilization II: Test of Time", [], [], "paper", "atlas blue and gold", n=OLD),
T("Worms 2", [], [], "calm", "worm pink and sky blue", n=OLD),
T("Worms World Party", [], [], "calm", "worm pink and sky blue", n=OLD),
# RPGs and action RPGs
D("Diablo", [], [], "horror", "hell red and bone", n=OLD),
D("Diablo II", [], [], "horror", "hell red and bone", n=OLD + "A, T and Q opening the character, skill and quest screens also work; F1 to F8 are the skill slots."),
D("Nox", [], [], "mystic", "arcane violet", n=OLD),
D("Dungeon Siege: Legends of Aranna", [], [], "parchment", "dungeon brown and gold", n=OLD),
D("Pirates of the Caribbean", [], [], "parchment", "sea blue and treasure gold", n=OLD),
C("Might and Magic VII: For Blood and Honor", [], [], "parchment", "Enroth blue and gold", left="wasd", n=OLD + "Walking on WASD or the arrow keys is the default."),
C("Might and Magic VIII: Day of the Destroyer", [], [], "parchment", "Jadame green and gold", left="wasd", n=OLD + "Walking on WASD or the arrow keys is the default."),
W("Ultima Underworld: The Stygian Abyss", [], [], "horror", "dungeon torchlight", n=OLD),
W("Toontown Online", [], [], "calm", "cartoon pastels"),
C("觅长生", ["1189490"], [], "mystic", "xianxia jade and violet", n="A mouse-driven cultivation game."),
W("Freelancer", [], [], "scifi", "deep-space blue", n=OLD + "Flight uses the mouse; W and S set the throttle."),
# Adventure and action
C("The Secret of Monkey Island", [], [], "pixel", "island blue and rum brown", n=OLD),
C("Atlantis: The Lost Tales", [], [], "mystic", "Atlantean turquoise", n=OLD),
W("The Mysterious Island", [], [], "nature", "castaway green and sand", n=OLD),
W("Harry Potter and the Prisoner of Azkaban", [], [], "mystic", "Hogwarts crimson and gold", n=OLD),
W("Barnyard", [], [], "nature", "barnyard green and red", n=OLD),
W("Shrek 2: The Game", [], [], "nature", "swamp green", n=OLD),
W("Spider-Man 2: The Game", [], [], "scifi", "Spider-Man red and blue", n=OLD),
W("Sanity: Aiken's Artifact", [], [], "horror", "lab-green and warning red", n=OLD),
W("Abuse", [], [], "pixel", "bunker grey and green", n=OLD + "Aim is with the mouse; WASD or the arrows move."),
W("Oni", [], [], "scifi", "neon blue and anime red", n=OLD),
W("Thief: Deadly Shadows", ["6980"], ["t3main.exe"], "noir", "moonlit grey and gold", n="Crouch on C and lean on Q and E are the series defaults.", b={"West": ("k:C", "Crouch", "footprints"), "LeftBumper": ("k:Q", "Lean left", "rotate-left"), "RightBumper": ("k:E", "Lean right", "rotate")}),
W("Pro Rally 2001", [], [], "industrial", "rally orange", n=OLD),
W("Just Cause", ["6880"], ["justcause.exe"], "military", "tropical green and khaki"),
W("Postal", [], [], "horror", "blood red", n=OLD),
W("Tom Clancy's Splinter Cell", ["13560"], ["splintercell.exe"], "noir", "night-vision green", n="Crouch on C is a guess.", b={"West": ("k:C", "Crouch", "footprints")}),
W("Manhunt 2", [], [], "horror", "grimy yellow and red"),
W("Stranglehold", [], [], "noir", "Hong Kong red and gold"),
W("Total Overdose: A Gunslinger's Tale in Mexico", [], [], "casino", "Mexican red and gold"),
W("Freedom Fighters", ["1347780"], [], "military", "Soviet red and khaki"),
W("The Lord of the Rings: The Fellowship of the Ring (2002)", [], [], "mystic", "Shire green and gold"),
W("American McGee's Alice", [], [], "horror", "twisted Wonderland violet"),
W("Blockland", ["250340"], ["blockland.exe"], "paper", "brick colors"),
W("WWII Tank Commander", [], [], "military", "tank khaki"),
W("Dr. Brain: Action Reaction", [], [], "paper", "lab-coat blue"),
# Online and survival
W("World of Tanks", ["1407200"], ["worldoftanks.exe"], "military", "tank olive and brass", b={"West": ("k:X", "Sniper mode", "crosshair")}, n="X for sniper mode is a guess."),
W("World of Tanks Blitz", ["444200"], ["wotblitz.exe"], "military", "tank olive and brass"),
W("World of Warships", ["552990"], ["worldofwarships.exe"], "military", "navy blue and brass"),
W("Heroes & Generals", ["227940"], ["hng.exe", "heroesandgeneralsdesktop.exe"], "military", "field khaki"),
W("Just Survive", ["295110"], ["h1z1.exe"], "industrial", "survival rust"),
W("Scum", ["513710"], ["scum.exe"], "industrial", "prison orange"),
W("Deadside", ["895400"], [], "industrial", "post-Soviet rust"),
W("The Long Drive", ["1017180"], [], "industrial", "desert road orange"),
W("Escape the Backrooms", ["1943950"], [], "horror", "yellow wallpaper"),
W("Minecraft: Java Edition", [], [], "nature", "block green and dirt brown", b={"West": ("k:Q", "Drop", "drop-item"), "LeftBumper": ("k:E", "Inventory", "bag"), "RightBumper": ("k:F", "Swap hands", "flip")}, n="Java Edition has no controller support (Bedrock does). E opens the inventory and Q drops."),
W("The Hunter", ["253710"], ["thehunter.exe"], "nature", "forest green"),
W("Propnight", ["1549180"], [], "horror", "night blue and red"),
W("100% Orange Juice!", ["282800"], ["100orange.exe"], "calm", "juicy orange", n="A board game played on the pointer."),
C("Wartales", ["1527950"], ["wartales.exe"], "parchment", "mercenary leather", n="Space pauses real time in the game.", b={"North": ("k:SPACE", "Pause", "pause")}),
C("ATRI -My Dear Moments-", ["1230140"], [], "calm", "seaside blue", n="A visual novel: the pointer clicks, Enter advances.", b={"North": ("k:ENTER", "Advance", "arrow")}),
C("Unheard", ["942970"], ["unheard.exe"], "noir", "noir grey and amber"),
W("Pico Park", ["1509960"], ["pico_park.exe"], "pixel", "party pastels", n="Keyboard play: arrows or WASD, Space jumps.", b={"North": ("k:SPACE", "Jump", "arrow")}),
W("We Were Here Expeditions: The FriendShip", ["2296990"], [], "mystic", "ice blue"),
]
# Old games with neither a Steam id nor a known executable cannot get an auto-switch rule: left out until one is known.
S = [g for g in S if g["ids"] or g["exes"]]
