from helpers import *
SPD = lambda b="RightBumper": (b, "Game speed", ["k:2", "k:3", "k:1"], "Next speed")
GUESS = "Typical defaults for the genre, not checked against the game."
S = []
NAT = " The game has some controller support of its own; this pack plays it with the pointer and keyboard keys for the parts the pad handles badly."
def X(name, ids, exes, arch, theme, mood, b=None, n="", left=None, cyc=None, tail=None, **kw):
    """Pointer-driven games that do have some controller support."""
    base = {"North": ("k:SPACE", "Pause", "pause")} if arch == "strat" else {}
    if not tail:
        if arch == "strat": tail = f"The left stick pans, the right stick points; RT and A click, LT right-clicks. Overlays in {mood}."
        elif left == "wasd": tail = f"The left stick walks on WASD and the right stick points; RT and A click, LT right-clicks. Overlays in {mood}."
        else: tail = f"The right stick points; RT and A click, LT right-clicks. Overlays in {mood}."
    return G(name, ids, exes, arch, theme, tail, b={**base, **(b or {})}, n=(n + " " if n else "") + GUESS, left=left, cyc=cyc, pad=NAT, **kw)
ADV = {"North": ("k:ENTER", "Advance", "arrow")}
JUMP = {"North": ("k:SPACE", "Jump", "arrow"), "West": ("k:E", "Use", "hand")}
VEH = {"West": ("k:E", "Enter vehicle", "car")}
CHAR = {"West": ("k:C", "Character", "person"), "LeftBumper": ("k:I", "Inventory", "bag"), "RightBumper": ("k:M", "Map", "map")}
S += [
X("Cities: Skylines", ["255710"], [], "strat", "paper", "city-plan blue", cyc=SPD(), n="Space pauses and 1 to 3 set the speed."),
X("Cities: Skylines II", ["949230"], ["cities2.exe"], "strat", "paper", "city-plan blue", cyc=SPD(), n="Space pauses and 1 to 3 set the speed."),
X("Kerbal Space Program", ["220200"], ["ksp_x64.exe", "ksp.exe"], "click", "scifi", "mission-control blue", left="wasd", n="WASD pitch and yaw the craft in flight; Space stages."),
X("Tabletop Simulator", ["286160"], [], "click", "casino", "felt green", n="Pick up with a click; R rotates a held piece.", b={"West": ("k:R", "Rotate held piece", "rotate")}),
X("XCOM 2", ["268500"], ["xcom2.exe"], "strat", "scifi", "ADVENT blue", b={"North": ("k:SPACE", "Next soldier", "person"), "West": ("k:TAB", "Cycle target", "crosshair")}, n="Tab cycles targets; Space jumps to the next soldier."),
X("XCOM: Enemy Unknown", ["200510"], ["xcomgame.exe"], "strat", "scifi", "XCOM blue", b={"North": ("k:SPACE", "Next soldier", "person")}),
X("XCOM: Chimera Squad", ["882100"], [], "strat", "scifi", "neon city blue", b={"North": ("k:SPACE", "Next agent", "person")}),
X("Shadow Tactics: Blades of the Shogun", ["418240"], ["shadow tactics.exe"], "strat", "noir", "ink and Edo red", b={"West": ("k:TAB", "Next character", "person")}, n="Space starts Shadow Mode planning."),
X("Sid Meier's Civilization VII", ["1295660"], [], "strat", "paper", "atlas blue and gold", b={"North": ("k:ENTER", "Next turn", "hourglass")}),
X("Fallout Shelter", ["588430"], ["falloutshelter.exe"], "click", "industrial", "Vault-Tec yellow and blue"),
X("Dota Underlords", ["1046930"], ["underlords.exe"], "click", "mystic", "underlord violet"),
X("Minion Masters", ["489520"], ["minionmasters.exe"], "click", "calm", "card-table teal"),
X("Uno (2016)", ["470220"], ["uno.exe"], "click", "casino", "card red and yellow"),
X("House Flipper", ["613100"], ["houseflipper.exe"], "click", "calm", "home-reno pastels", left="wasd", b=JUMP),
X("PC Building Simulator", ["621060"], [], "click", "scifi", "circuit-board green"),
X("Car Mechanic Simulator 2018", ["645630"], ["cms2018.exe"], "click", "industrial", "garage orange", left="wasd"),
X("Car Mechanic Simulator 2021", ["1190000"], [], "click", "industrial", "garage orange", left="wasd"),
X("Supermarket Simulator", ["2670630"], [], "click", "calm", "shop-floor pastels", left="wasd", b=JUMP),
X("TCG Card Shop Simulator", ["3070070"], [], "click", "casino", "card-shop gold", left="wasd"),
X("Potion Craft", ["1210320"], [], "click", "mystic", "alchemist violet"),
X("Farming Simulator 22", ["1248130"], [], "click", "nature", "farm green and yellow", left="wasd", b=VEH),
X("Farming Simulator 19", ["787860"], [], "click", "nature", "farm green and yellow", left="wasd", b=VEH),
X("Farming Simulator 25", ["2300320"], [], "click", "nature", "farm green and yellow", left="wasd", b=VEH),
X("Farming Simulator 17", ["447020"], ["farmingsimulator2017game.exe"], "click", "nature", "farm green and yellow", left="wasd", b=VEH),
X("Jurassic World Evolution", ["648350"], ["jwe.exe"], "strat", "nature", "jungle green and amber", cyc=SPD()),
X("Jurassic World Evolution 2", ["1244460"], [], "strat", "nature", "jungle green and amber", cyc=SPD()),
X("Space Engineers", ["244850"], ["spaceengineers.exe"], "click", "scifi", "hull grey and amber", left="wasd", b={"North": ("k:SPACE", "Up / jump", "arrow")}),
X("This War of Mine", ["282070"], ["this war of mine.exe"], "click", "noir", "ash grey", left="wasd"),
X("For the King", ["527230"], ["ftk.exe"], "click", "parchment", "board-game gold"),
X("Phoenix Wright: Ace Attorney Trilogy", ["787480"], [], "click", "casino", "courtroom blue and gold", b=ADV),
X("The Hungry Lamb: Traveling in the Late Ming Dynasty", ["2593370"], [], "click", "parchment", "ink-wash grey and red", b=ADV),
X("Needy Streamer Overload", ["1451940"], [], "click", "mystic", "streaming pink and violet"),
X("To the Moon", ["206440"], ["to the moon.exe"], "click", "calm", "moonlit blue", left="wasd"),
X("Disco Elysium", ["632470"], [], "click", "noir", "ochre and nicotine", b={"West": ("k:TAB", "Thought cabinet", "brain"), "LeftBumper": ("k:J", "Journal", "book"), "RightBumper": ("k:I", "Inventory", "bag")}),
X("Forager", ["751780"], [], "click", "pixel", "island green", left="wasd"),
X("Stormworks: Build and Rescue", ["573090"], ["stormworks64.exe"], "click", "industrial", "rescue orange", left="wasd"),
X("PowerWash Simulator", ["1290000"], [], "click", "calm", "clean-water blue", left="wasd"),
X("Medieval Dynasty", ["1129580"], [], "click", "nature", "village brown and green", left="wasd", b=JUMP),
X("Graveyard Keeper", ["599140"], ["graveyard keeper.exe"], "click", "horror", "graveyard purple", left="wasd"),
X("My Time at Portia", ["666140"], ["portia.exe"], "click", "nature", "workshop green", left="wasd"),
X("My Time at Sandrock", ["1084600"], [], "click", "industrial", "desert workshop orange", left="wasd"),
X("Pathfinder: Wrath of the Righteous", ["1184370"], ["wrath.exe"], "strat", "parchment", "crusader gold"),
X("Divinity: Original Sin - Enhanced Edition", ["373420"], [], "strat", "parchment", "Rivellon gold"),
X("Warhammer 40,000: Rogue Trader", ["2186680"], ["wh40krt.exe"], "strat", "scifi", "Imperial gold"),
X("Loop Hero", ["1282730"], [], "click", "pixel", "loop gold", b={"North": ("k:SPACE", "Pause", "pause")}),
X("Total War: Attila", ["325610"], ["attila.exe"], "strat", "parchment", "Dark Ages grey"),
X("Gwent: The Witcher Card Game", ["1284410"], ["gwent.exe"], "click", "parchment", "Witcher gold"),
X("Magic Duels", ["316010"], ["magicduels.exe"], "click", "mystic", "planeswalker violet"),
X("Transport Fever 2", ["1066780"], [], "strat", "industrial", "railway brown", cyc=SPD()),
X("Tropico 6", ["492720"], [], "strat", "calm", "island pastels", cyc=SPD()),
X("Surviving Mars", ["464920"], ["marssteam.exe"], "strat", "industrial", "Mars rust", cyc=SPD()),
X("Workers & Resources: Soviet Republic", ["784150"], [], "strat", "industrial", "Soviet rust", cyc=SPD()),
X("Dorfromantik", ["1455840"], [], "click", "nature", "village green", b={"West": ("k:R", "Rotate", "rotate")}),
X("Kingdom: Classic", ["368230"], ["kingdom.exe"], "click", "pixel", "dusk purple", left="wasd"),
X("Beholder", ["475550"], ["beholder.exe"], "click", "noir", "totalitarian grey"),
X("Unpacking", ["1135690"], [], "click", "pixel", "cardboard brown", b={"West": ("k:R", "Rotate", "rotate")}),
X("The Escapists 2", ["641990"], ["theescapists2.exe"], "click", "pixel", "prison orange", left="wasd"),
X("ShellShock Live", ["326460"], ["shellshocklive.exe"], "click", "pixel", "artillery green", left="wasd"),
X("Touhou Mystia's Izakaya", ["1584090"], [], "click", "mystic", "izakaya lantern red"),
X("Gas Station Simulator", ["1149620"], [], "click", "industrial", "forecourt orange", left="wasd"),
X("Danganronpa: Trigger Happy Havoc", ["413410"], ["danganronpa.exe"], "click", "horror", "despair pink and black", b=ADV),
X("Danganronpa 2: Goodbye Despair", ["413420"], ["dr2_us.exe"], "click", "horror", "despair pink and black", b=ADV),
X("Doki Doki Literature Club Plus!", ["1388880"], [], "click", "calm", "club pastels", b=ADV),
X("Slay the Princess", ["1989270"], [], "click", "noir", "ink grey", b=ADV),
X("Titan Quest Anniversary Edition", ["475150"], ["tq.exe"], "click", "parchment", "Greek marble and gold", b=CHAR),
X("Halls of Torment", ["2218750"], [], "click", "horror", "dungeon red", left="wasd"),
X("Dredge", ["1562430"], [], "click", "noir", "fog grey and teal", left="wasd"),
X("Bendy and the Ink Machine", ["622650"], ["batim.exe"], "click", "noir", "ink sepia", left="wasd"),
X("Superliminal", ["1049410"], [], "click", "paper", "dream pastels", left="wasd"),
X("The Talos Principle", ["257510"], ["talos.exe"], "click", "mystic", "ancient marble", left="wasd"),
X("Return of the Obra Dinn", ["653530"], [], "click", "pixel", "1-bit black and white", left="wasd"),
X("The Coffin of Andy and Leyley", ["2378900"], [], "click", "horror", "flesh pink and black", b=ADV),
X("Football Manager 2020", ["1100600"], [], "click", "paper", "touchline grey and green"),
]
