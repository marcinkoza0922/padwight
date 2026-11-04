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
M = " The executable name is from memory, not checked against an install."
D = dict(exenote=M)
OLD = " The game is from before controllers mattered on PC, so the pad plays it with the pointer and keyboard keys."
def O(name, exes, arch, theme, mood, b=None, n="", left=None, cyc=None, ids=None, **kw):
    """Older pointer and keyboard games, executable names from memory."""
    return X(name, ids or [], exes, arch, theme, mood, b=b, n=n, left=left, cyc=cyc, nat=False, **{"exenote": M, **kw})
def FO(name, exes, theme, mood, b=None, n="", ids=None, **kw):
    """Older shooters, executable names from memory."""
    return F(name, ids or [], exes, theme, mood, b=b, n=n, **{"exenote": M, **kw})
HERO = {"West": ("k:ENTER", "End turn", "hourglass"), "North": ("k:SPACE", "Next hero", "person")}
SIM = {"North": ("k:SPACE", "Pause", "pause")}
S += [
# --- Sims and building
O("The Sims", ["sims.exe"], "click", "calm", "suburb pastels", n="Click a Sim, then pick from the pie menu; the speed keys are 1 to 3."),
O("The Sims 2", ["sims2.exe"], "click", "calm", "suburb pastels", left="wasd", n="Camera moves on the arrow keys here; WASD is a guess from other builders."),
O("SimCity 2000", ["sc2000.exe", "simcity 2000.exe"], "strat", "paper", "city-plan blue", cyc=SPD()),
O("Theme Hospital", ["hospital.exe", "theme hospital.exe"], "click", "calm", "ward mint and white", n="Everything is on the mouse; the pad's pointer is enough."),
O("Theme Park World", ["themepark.exe", "tpw.exe"], "click", "casino", "funfair red and yellow"),
O("Zoo Tycoon", ["zoo tycoon/zootycoon.exe"], "strat", "nature", "savanna gold", cyc=SPD(), ids=[], exenote=" The Windows executable is from Discord's list."),
O("Black & White", ["black & white.exe", "runblack.exe"], "click", "mystic", "creature green and gold", n="The mouse does the gestures; hold LT to drag and gesture."),
O("Black & White 2", ["white.exe", "black & white 2.exe"], "click", "mystic", "creature green and gold", left="wasd"),
# --- RTS and strategy
O("Age of Empires", ["empires.exe", "empiresx.exe"], "strat", "parchment", "bronze-age sand"),
O("Age of Empires II: The Age of Kings", ["empires2.exe", "age2_x1.exe"], "strat", "parchment", "castle stone and gold", ids=[], exenote=" The executable names are from Discord's list for the HD Edition."),
O("Age of Mythology", ["age of mythology.exe", "aom.exe"], "strat", "mystic", "Olympian blue and gold", ids=[], exenote=M),
O("Rise of Nations", ["rise of nations.exe"], "strat", "parchment", "empire red and gold"),
O("Rise of Nations: Rise of Legends", ["riseoflegends.exe", "rol.exe"], "strat", "mystic", "steam-and-sorcery brass"),
O("Warcraft: Orcs & Humans", ["war.exe", "warcraft.exe"], "strat", "parchment", "orc green and iron"),
O("Warcraft II: Battle.net Edition", ["warcraft ii bne.exe", "war2.exe"], "strat", "parchment", "alliance blue and gold"),
O("Dune II: The Building of a Dynasty", ["dune2.exe", "dune ii.exe"], "strat", "scifi", "Arrakis sand and spice"),
O("Dune 2000", ["dune2000.exe", "dune 2000.exe"], "strat", "scifi", "Arrakis sand and spice"),
O("Emperor: Battle for Dune", ["emperor.exe"], "strat", "scifi", "Arrakis sand and spice"),
O("The Lord of the Rings: The Battle for Middle-earth", ["lotrbfme.exe"], "strat", "parchment", "Middle-earth gold"),
O("The Lord of the Rings: The Battle for Middle-earth II", ["lotrbfme2.exe"], "strat", "parchment", "Middle-earth gold"),
O("Homeworld: Cataclysm", ["homeworld.exe", "cataclysm.exe"], "click", "scifi", "deep-space blue", left="wasd", n="Orders go with the pointer; WASD and the wheel rotate and zoom the camera."),
O("Star Trek: Armada", ["armada.exe", "st armada.exe"], "strat", "scifi", "LCARS orange and blue"),
O("Star Trek: Armada II", ["armada2.exe", "armada ii.exe"], "strat", "scifi", "LCARS orange and blue"),
O("Star Trek: New Worlds", ["newworlds.exe", "st new worlds.exe"], "strat", "scifi", "LCARS orange and blue"),
O("Total Annihilation: Kingdoms", ["tak.exe"], "strat", "parchment", "realm gold and green"),
O("ParaWorld", ["paraworld.exe"], "strat", "nature", "prehistoric green and amber"),
O("The Settlers II: Veni, Vidi, Vici", ["s2.exe", "settlers2.exe"], "click", "parchment", "meadow green and brown"),
O("The Settlers III", ["s3_alleinstehend.exe", "settlers3.exe"], "strat", "parchment", "meadow green and brown"),
O("The Settlers IV", ["s4_main.exe", "settlers4.exe"], "strat", "parchment", "meadow green and brown"),
O("Hammer & Sickle", ["hammer&sickle.exe", "hs.exe"], "strat", "military", "Cold War olive and red", b={"West": ("k:TAB", "Next unit", "person")}),
O("Worms 2", ["worms2.exe"], "click", "casino", "cartoon green and pink", n="Aim with the pointer, fire with a click."),
O("Worms World Party", ["wwp.exe"], "click", "casino", "cartoon green and pink", n="Aim with the pointer, fire with a click."),
O("WWII Tank Commander", ["tankcommander.exe", "wwii tank commander.exe"], "click", "military", "armour olive", left="wasd"),
# --- turn-based, 4X, RPG
O("Heroes of Might and Magic: A Strategic Quest", ["heroes.exe", "homm1.exe"], "click", "parchment", "heraldic red and gold", b=HERO),
O("Heroes of Might and Magic II", ["heroes2.exe", "homm2.exe"], "click", "parchment", "heraldic red and gold", b=HERO),
O("Heroes of Might and Magic IV", ["heroes4.exe", "homm4.exe"], "click", "parchment", "heraldic red and gold", b=HERO),
O("Sid Meier's Civilization II", ["civ2.exe"], "click", "paper", "atlas sepia", b={"North": ("k:ENTER", "End turn", "hourglass")}),
O("Civilization II: Test of Time", ["civ2tot.exe"], "click", "paper", "atlas sepia", b={"North": ("k:ENTER", "End turn", "hourglass")}),
O("Diablo", ["diablo.exe"], "click", "horror", "ember red and iron", ids=[], exenote=" The executable name is from Discord's list.", b={"West": ("k:I", "Inventory", "bag"), "LeftBumper": ("k:C", "Character", "person"), "RightBumper": ("k:Q", "Quests", "book")}, n="Potions are on the number keys; Tab shows the automap."),
O("Diablo II", ["diablo ii.exe"], "click", "horror", "ember red and iron", exenote=" The executable names are from Discord's list.", b={"West": ("k:I", "Inventory", "bag"), "LeftBumper": ("k:C", "Character", "person"), "RightBumper": ("k:T", "Skill tree", "map"), "North": ("k:TAB", "Automap", "map")}, n="Potions are on 1 to 4; hold Alt to see items on the ground."),
O("Neverwinter Nights", ["nwmain.exe"], "click", "parchment", "Forgotten Realms gold", exenote=" The executable name is from Discord's list for the Enhanced Edition.", left="wasd", b={"West": ("k:C", "Character", "person"), "LeftBumper": ("k:I", "Inventory", "bag"), "RightBumper": ("k:M", "Map", "map")}),
O("Nox", ["nox.exe"], "click", "mystic", "occult violet and green", left="wasd", b={"West": ("k:I", "Inventory", "bag")}),
O("Baldur's Gate", ["baldur.exe", "bgmain.exe"], "strat", "parchment", "Faerûn parchment and ink", b={"West": ("k:SPACE", "Pause", "pause"), "North": ("k:I", "Inventory", "bag"), "LeftBumper": ("k:J", "Journal", "book"), "RightBumper": ("k:M", "Map", "map")}),
O("Baldur's Gate II: Shadows of Amn", ["baldur.exe", "bgmain.exe"], "strat", "parchment", "Faerûn parchment and ink", b={"West": ("k:TAB", "Highlight objects", "eye"), "North": ("k:I", "Inventory", "bag"), "LeftBumper": ("k:J", "Journal", "book"), "RightBumper": ("k:M", "Map", "map")}),
O("Dungeon Siege: Legends of Aranna", ["aranna.exe"], "click", "parchment", "hero gold and green", b={"West": ("k:I", "Inventory", "bag"), "LeftBumper": ("k:S", "Spells", "book"), "RightBumper": ("k:M", "Map", "map")}, exenote=" The executable name is from Discord's list for Dungeon Siege."),
O("Might and Magic VII: For Blood and Honor", ["mm7.exe"], "click", "parchment", "royal blue and gold", left="arrows", b={"West": ("k:SPACE", "Interact", "hand")}),
O("Might and Magic VIII: Day of the Destroyer", ["mm8.exe"], "click", "parchment", "royal blue and gold", left="arrows", b={"West": ("k:SPACE", "Interact", "hand")}),
O("Might and Magic IX", ["mm9.exe"], "click", "parchment", "royal blue and gold", left="wasd", b={"West": ("k:SPACE", "Interact", "hand")}),
O("Ultima Underworld: The Stygian Abyss", ["uw.exe", "uw1.exe"], "click", "horror", "dungeon torchlight", left="wasd", b={"West": ("k:F", "Fight", "crosshair")}),
O("Pirates of the Caribbean", ["pirates.exe", "potc.exe"], "click", "nature", "Caribbean sea and gold", left="wasd", b={"West": ("k:F", "Draw sword", "crosshair"), "RightBumper": ("k:M", "Map", "map")}),
O("Toontown Online", ["toontown.exe", "ttoon.exe"], "click", "casino", "toon yellow and blue", left="arrows"),
# --- adventure
O("The Secret of Monkey Island", ["monkey.exe", "monkey1.exe"], "click", "calm", "Caribbean dusk", n="Left click walks and uses; right click looks.", b={"East": ("mR", "Look at", "eye")}),
O("Atlantis: The Lost Tales", ["atlantis.exe"], "click", "mystic", "Atlantean blue and gold"),
O("The Mysterious Island", ["mysterious island.exe", "island.exe"], "click", "nature", "Verne sepia and sea green"),
O("American McGee's Alice", ["alice.exe"], "click", "horror", "twisted Wonderland red and blue", left="wasd", b={"South": ("k:SPACE", "Jump", "arrow"), "West": ("k:E", "Use", "hand")}),
O("Harry Potter and the Prisoner of Azkaban", ["harry potter and the prisoner of azkaban.exe", "hp3.exe"], "click", "mystic", "Hogwarts maroon and gold", left="wasd", b={"South": ("k:SPACE", "Jump", "arrow"), "West": ("k:E", "Use", "hand")}),
O("Shrek 2: The Game", ["shrek2.exe"], "click", "nature", "swamp green and gold", left="wasd", b={"South": ("k:SPACE", "Jump", "arrow"), "West": ("k:E", "Use", "hand")}),
O("The Lord of the Rings: The Fellowship of the Ring (2002)", ["fellowship.exe", "lotr.exe"], "click", "parchment", "Shire green and gold", left="wasd", b={"South": ("k:SPACE", "Jump", "arrow"), "West": ("k:E", "Use", "hand")}),
O("Spider-Man 2: The Game", ["spider-man2.exe", "spiderman2.exe"], "click", "noir", "web red and blue", left="wasd", b={"South": ("k:SPACE", "Jump", "arrow"), "West": ("k:E", "Use", "hand")}),
O("Abuse", ["abuse.exe"], "click", "industrial", "bunker green", left="wasd", b={"South": ("k:SPACE", "Jump", "arrow")}),
O("Postal", ["postal plus.exe", "postal.exe"], "click", "noir", "grimy red and grey", left="wasd", exenote=" The executable name is from Discord's list.", b={"South": ("k:SPACE", "Use", "hand")}),
O("Oni", ["oni.exe"], "click", "scifi", "cyber-anime magenta and steel", left="wasd", b={"South": ("k:SPACE", "Jump", "arrow"), "West": ("k:E", "Action", "hand"), "RightBumper": ("k:R", "Reload", "rotate")}),
O("Manhunt 2", ["manhunt2.exe"], "click", "horror", "asylum grey and blood", left="wasd", b={"West": ("k:E", "Action", "hand"), "South": ("k:SPACE", "Jump", "arrow")}),
O("Stranglehold", ["stranglehold.exe"], "click", "noir", "Hong Kong neon", left="wasd", b={"South": ("k:SPACE", "Jump", "arrow"), "West": ("k:E", "Action", "hand")}),
O("Total Overdose: A Gunslinger's Tale in Mexico", ["totaloverdose.exe", "to.exe"], "click", "casino", "lucha libre red and yellow", left="wasd", b={"South": ("k:SPACE", "Jump", "arrow"), "West": ("k:E", "Action", "hand")}),
O("Starship Troopers", ["starship troopers.exe", "sst.exe"], "click", "military", "Federation khaki", left="wasd", b={"South": ("k:SPACE", "Jump", "arrow"), "West": ("k:E", "Use", "hand")}),
# --- shooters (mouse-and-keyboard, gyro profile)
FO("Soldier of Fortune II: Double Helix", ["sof2mp.exe", "sof2.exe"], "military", "mercenary olive and red"),
FO("Clive Barker's Undying", ["undying.exe"], "horror", "gothic purple and bone", exenote=" The executable name is from Discord's list.", ids=[]),
FO("The Suffering", ["suffering.exe"], "horror", "prison rust and blood"),
FO("The Suffering: Ties That Bind", ["tiesthatbind.exe"], "horror", "prison rust and blood"),
FO("Medal of Honor: Pacific Assault", ["mohpa.exe"], "military", "Pacific jungle green"),
FO("The Wheel of Time", ["wot.exe", "unreal.exe"], "mystic", "Aes Sedai blue and gold"),
FO("Land of the Dead: Road to Fiddler's Green", ["lotd.exe", "land of the dead.exe"], "horror", "undead grey and amber"),
FO("James Bond 007: Nightfire", ["nightfire.exe", "bond.exe"], "noir", "Bond gold and black"),
FO("World War II Combat: Iwo Jima", ["iwojima.exe"], "military", "Pacific sand and olive"),
FO("World War II Combat: Road to Berlin", ["roadtoberlin.exe"], "military", "winter-front grey"),
FO("Combat: Task Force 121", ["ctf121.exe", "taskforce121.exe"], "military", "special-ops grey"),
FO("Area 51 (2005)", ["area51.exe", "area 51.exe"], "scifi", "hangar green and grey"),
FO("Mortyr 2: For Ever", ["mortyr2.exe"], "military", "wartime sepia and olive"),
FO("Star Trek: Elite Force II", ["stvoy.exe", "ef2.exe"], "scifi", "LCARS orange and blue"),
FO("Sanity: Aiken's Artifact", ["sanity.exe"], "mystic", "occult violet and steel"),
FO("Vivisector: Beast Within", ["vivisector.exe"], "horror", "island green and blood"),
FO("Close Quarters Conflict", ["cqc.exe", "closequarters.exe"], "military", "urban grey and olive"),
FO("I.G.I.-2: Covert Strike", ["igi2.exe"], "military", "covert olive and grey"),
FO("Project I.G.I.: I'm Going In", ["igi.exe"], "military", "covert olive and grey"),
FO("Devastation", ["devastation.exe"], "industrial", "dystopian rust and grey"),
FO("Dead Man's Hand", ["dmh.exe", "deadmanshand.exe"], "parchment", "Old West dust and brass"),
FO("Marine Sharpshooter 3", ["ms3.exe", "marinesharpshooter3.exe"], "military", "jungle green"),
FO("Marine Sharpshooter 4: Locked and Loaded", ["ms4.exe", "marinesharpshooter4.exe"], "military", "desert tan"),
FO("Marine Heavy Gunner: Vietnam", ["mhg.exe", "marineheavygunner.exe"], "military", "Vietnam jungle green"),
FO("Day of the Zombie", ["dotz.exe", "dayofthezombie.exe"], "horror", "dead-town grey and red"),
FO("Jurassic Park: Trespasser", ["trespass.exe"], "nature", "island green and amber", n="The game's body is controlled with the mouse; expect to fight the controls."),
FO("Battlestrike: Force of Resistance", ["battlestrike.exe"], "military", "resistance olive"),
FO("Hellforces", ["hellforces.exe"], "horror", "hell red and black"),
FO("Tribes: Vengeance", ["tribesv.exe", "tribes vengeance.exe"], "scifi", "tribal orange and steel"),
FO("Freelancer", ["freelancer.exe"], "scifi", "deep-space blue and amber", n="Flight on the mouse follows the pointer; the right stick steers the ship's nose."),
# --- from the no-controller list
F("Warface", ["291480", "585030"], ["bin64release/game.exe", "warface/bin32release/game.exe"], "military", "special-ops olive"),
F("Counter-Strike Nexon", ["273110"], ["bin/cstrike-online.exe"], "military", "counter-terror blue and tan"),
F("Mass Effect 2", ["2362420", "24980", "47760", "901242"], ["me2game.exe", "masseffect2.exe"], "scifi", "Normandy blue and orange"),
X("Dead Rising 2", ["45740"], ["deadrising2.exe"], "click", "horror", "mall-zombie yellow and red", left="wasd", b={"South": ("k:SPACE", "Jump", "arrow"), "West": ("k:E", "Use", "hand")}, nat=False),
F("Blockstorm", ["263060"], ["blockstorm/blockstorm.exe"], "military", "voxel battlefield green"),
X("Secret Neighbor", ["859570", "999730"], ["secret neighbor/secret neighbour.exe"], "click", "horror", "suburban night blue", left="wasd", b={"South": ("k:SPACE", "Jump", "arrow"), "West": ("k:E", "Use", "hand")}, nat=False),
F("Creative Destruction", ["893520"], ["creativedestruction/client.exe"], "scifi", "island neon blue and orange"),
]
