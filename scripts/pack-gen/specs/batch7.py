from helpers import *
S = []
SP = {"North": ("k:SPACE", "Pause", "pause")}
SPN = "Space pausing is a guess."
GUESS = "Typical defaults for the genre, not checked against the game."
# Online RPGs: the pad walks with the left stick and steers or points with the right.
MMO_B = {"North": ("k:SPACE", "Jump", "arrow"), "West": ("k:TAB", "Next target", "arrow"), "LeftBumper": ("k:M", "Map", "book"), "RightBumper": ("k:I", "Inventory", "book")}
SURV_B = {"North": ("k:SPACE", "Jump", "arrow"), "West": ("k:E", "Use / pick up", "arrow"), "LeftBumper": ("k:TAB", "Inventory", "book"), "RightBumper": ("k:R", "Reload / rotate", "rotate")}
def M(name, ids, exes, theme, mood, n="", **kw):
    return G(name, ids, exes, "click", theme, f"The left stick walks on WASD and the right stick points or looks; RT and A click, LT right-clicks. Overlays in {mood}.", left="wasd", b=MMO_B, n=(n + " " if n else "") + "Tab, Space, M and I are common online-RPG defaults. " + GUESS, **kw)
def V(name, ids, exes, theme, mood, n="", **kw):
    return G(name, ids, exes, "click", theme, f"The left stick walks on WASD and the right stick looks or points; RT and A click, LT right-clicks. Overlays in {mood}.", left="wasd", b=SURV_B, n=(n + " " if n else "") + "E, Space, Tab and R are common survival defaults. " + GUESS, **kw)
def A(name, ids, exes, theme, mood, n="", **kw):
    return G(name, ids, exes, "click", theme, f"The left stick moves on WASD and the right stick points; RT and A click, LT right-clicks. Overlays in {mood}.", left="wasd", **{"b": {"North": ("k:SPACE", "Jump", "arrow")}, **kw}, n=(n + " " if n else "") + ("" if "b" in kw else "Space for jump is a guess."))
def P(name, ids, exes, theme, mood, n="", **kw):
    return G(name, ids, exes, "click", theme, f"The right stick points; A and RT click, LT right-clicks. Overlays in {mood}.", n=n or "A mouse-driven game; no keys are mapped beyond the mouse.", **kw)
def T(name, ids, exes, theme, mood, n="", **kw):
    return G(name, ids, exes, "strat", theme, f"The left stick pans, the right stick points; RT and A click, LT right-clicks. Overlays in {mood}.", n=n, **kw)
S += [
# MMOs and online RPGs
M("New World", ["1063730"], ["newworld.exe"], "parchment", "Aeternum gold and sea blue"),
M("Guild Wars 2", ["1284210"], ["gw2-64.exe", "gw2.exe"], "parchment", "Tyrian red and gold"),
M("RuneScape", ["1343400"], ["runescape.exe"], "parchment", "Gielinor brown and gold"),
M("Old School RuneScape", ["1343370"], ["osclient.exe"], "parchment", "Gielinor brown and gold"),
M("Neverwinter", ["109600"], ["gameclient.exe"], "parchment", "Forgotten Realms crimson"),
M("Rift", ["39120"], ["rift_x64.exe", "rift.exe"], "mystic", "rift violet"),
M("The Secret World", ["215280"], ["thesecretworld.exe", "thesecretworlddx11.exe", "secretworldlegends.exe", "secretworldlegendsdx11.exe"], "horror", "occult red"),
M("ArcheAge", ["304030"], [], "parchment", "sea-trade blue and gold"),
M("Aura Kingdom", ["268420"], [], "mystic", "anime pastel violet"),
M("Fallen Earth", ["113420"], [], "industrial", "wasteland rust"),
M("Conqueror's Blade", ["905370"], ["proven_ground_client.exe"], "parchment", "war-banner red", n="Mass battles with a lock-on camera."),
M("Life is Feudal: Your Own", ["290080"], ["yo_cm_client.exe"], "parchment", "medieval wood and iron"),
M("Life is Feudal: MMO", ["700030"], ["cm_client.exe"], "parchment", "medieval wood and iron"),
M("Reign of Kings", ["344760"], [], "parchment", "castle stone and gold"),
M("Naval Action", ["311310"], [], "military", "age-of-sail navy blue", n="Sailing on W and S for sheets, A and D for the helm; remembered."),
M("Star Wars: The Old Republic", ["1286830"], ["swtor.exe"], "scifi", "Republic gold and Sith red"),
M("Age of Conan: Unchained", ["217750"], [], "horror", "Hyborian blood red"),
M("EverQuest II", ["201230"], ["everquest2.exe"], "parchment", "Norrath blue and gold"),
M("Tree of Life", ["361800"], [], "nature", "tree-of-life green"),
M("Gloria Victis", ["327070"], ["gv.exe"], "parchment", "medieval crimson"),
M("Albion Online", ["761890"], ["albion-online.exe"], "parchment", "Albion green and gold", n="Mostly played by clicking to move; the left stick adds WASD."),
M("AdventureQuest 3D", ["429790"], ["aq3d.exe"], "mystic", "storybook purple"),
M("Hellgate: London", ["939520"], ["hellgate_sp_x86.exe"], "horror", "ruined-London red", n="Number keys cast the hotbar; none are mapped."),
M("Stay Out", ["1180380"], ["sogame.exe"], "industrial", "zone rust"),
M("Eternal Return: Black Survival", ["1049590"], ["eternalreturn.exe"], "mystic", "island dusk violet"),
M("Smite", ["386360"], ["smite.exe"], "mystic", "pantheon gold"),
M("Battlerite", ["504370"], ["battlerite.exe"], "mystic", "arena violet"),
# survival and sandbox
V("Creativerse", ["280790"], ["creativerse.exe"], "nature", "block-world green"),
V("CastleMiner Z", ["253430"], ["castleminerz.exe"], "nature", "block-world green"),
V("Sunkenland", ["2080690"], ["sunkenland.exe"], "nature", "flooded-world teal"),
V("Subsistence", ["418030"], [], "nature", "wilderness green"),
V("Savage Lands", ["307880"], ["savagelands.exe"], "nature", "jungle green"),
V("Mist Survival", ["914620"], ["mistsurv-win64-shipping.exe"], "horror", "fog grey and blood red"),
V("Next Day: Survival", ["519190"], ["nextday_game.exe"], "industrial", "post-collapse rust"),
V("Survival: Postapocalypse Now", ["351290"], [], "industrial", "post-collapse rust"),
V("Hurtworld", ["393420"], ["hurtworld.exe"], "industrial", "frontier rust"),
V("Dead Bits", ["303390"], ["dead bits.exe"], "horror", "zombie-yard green"),
V("Dead Frontier 2", ["744900"], ["deadfrontier2.exe"], "horror", "zombie-yard green"),
V("Mitos.is: The Game", ["389570"], ["mitosis.exe"], "nature", "cell-lab green"),
V("Romero's Aftermath", ["349700"], [], "horror", "zombie-yard green"),
V("SurrounDead", ["1645820"], ["surroundead-win64-shipping.exe"], "horror", "outbreak red"),
V("Infestation: The New Z", ["555570"], ["thenewz.exe"], "horror", "outbreak red"),
V("Holdfast: Nations At War", ["589290"], ["holdfast naw.exe"], "military", "Napoleonic blue and red"),
V("Ranch Simulator", ["1119730"], ["ranch_simulator-win64-shipping.exe"], "nature", "barn red and hay"),
V("Drug Dealer Simulator", ["682990"], ["drugdealersimulator-win64-shipping.exe"], "noir", "back-alley dark"),
V("Foxhole", ["505460"], ["war-win64-shipping.exe"], "military", "trench olive"),
V("Russian Fishing 4", ["766570"], ["rf4_x64.exe"], "nature", "lakeside green", n="Casting and reeling are mouse buttons."),
V("Feed and Grow: Fish", ["429050"], ["feed and grow.exe"], "calm", "ocean teal"),
V("Moonbase Alpha", ["39000"], ["moonbasealphagame.exe"], "scifi", "lunar grey and NASA blue"),
V("Derail Valley", ["588030"], ["derailvalley.exe"], "industrial", "rail-yard steel", n="Locomotive controls are mouse and keys; this maps the look and walking only."),
V("Penumbra: Overture", ["22180"], [], "horror", "cellar dark", n="Doors and objects are dragged with the mouse, so the right stick is the whole interface."),
V("Penumbra: Black Plague", ["22120"], [], "horror", "cellar dark", n="Doors and objects are dragged with the mouse, so the right stick is the whole interface."),
V("Blade of Darkness", ["1710170"], [], "parchment", "dungeon stone", n="Also released as Severance; the Steam id is the match."),
# 2D and top-down action
A("Iron Snout", ["424280"], ["ironsnout.exe"], "nature", "pig-farm pink"),
A("Deepest Sword", ["1647160"], ["deepest sword.exe"], "pixel", "pixel night"),
A("Soldat", ["638490"], ["soldat.exe"], "pixel", "arena green"),
A("Teeworlds", ["380840"], ["teeworlds.exe"], "pixel", "tee-world green", b={"North": ("k:SPACE", "Jump", "arrow"), "West": ("k:Q", "Switch weapon", "arrow")}, n="Teeworlds aims and hooks with the mouse; Q is a guess for weapon switching."),
A("CS2D", ["666220"], ["cs2d.exe"], "military", "top-down olive"),
A("Running with Rifles", ["270150"], ["rwr_game.exe"], "military", "field olive"),
A("Alien Shooter 2: Reloaded", ["33120"], [], "scifi", "hive-base green"),
A("Paunch", ["792220"], ["paunch.exe"], "pixel", "arcade amber"),
A("Montaro", ["495890"], [], "paper", "paper grey"),
A("Last Man Standing", ["506540"], ["lms.exe"], "industrial", "arena rust"),
A("Toribash", ["248570"], ["toribash.exe"], "pixel", "dojo white and red", n="Toribash is turn-based joint control by mouse; no keys are mapped beyond Space for the next turn.", b={"North": ("k:SPACE", "Next turn", "hourglass")}),
A("AirMech Strike", ["206500"], ["airmech.exe"], "scifi", "mech blue and orange"),
A("Tanki Online", ["562010"], ["tankionline.exe"], "military", "tank-yard grey"),
A("Hard Truck Apocalypse", ["285500"], ["hta.exe"], "industrial", "wasteland rust"),
A("The Ship", ["2400"], ["ship.exe"], "noir", "ocean-liner brass"),
A("War Robots", ["767560"], ["warrobots.exe"], "scifi", "robot-arena steel"),
A("Star Conflict", ["212070"], [], "scifi", "dogfight blue"),
A("Wobbly Life", ["1211020"], ["wobbly life.exe"], "calm", "toy-box primary colors"),
A("Quaver", ["980610"], ["quaver.exe"], "pixel", "rhythm-grid violet", n="A rhythm game on the keyboard (four or seven lanes); no lane keys are mapped, since you choose them in the game."),
V("Natural Selection 2", ["4920"], ["ns2.exe"], "scifi", "marine blue and alien orange", n="Commanders play it as an RTS by mouse; this maps the on-foot role."),
V("Bigfoot", ["509980"], ["bigfoot-win64-shipping.exe"], "horror", "forest night"),
A("Dark Deception", ["332950"], ["ddeception-win64-shipping.exe"], "horror", "carnival red", b={"North": ("k:SPACE", "Jump", "arrow"), "West": ("k:E", "Use", "arrow")}, n="E to use is a guess."),
A("Who's Your Daddy", ["427730"], ["whosyourdaddy.exe"], "calm", "nursery pastel"),
# strategy, builders and sims
T("Humankind", ["1124300"], ["humankind.exe"], "parchment", "era-bronze and gold"),
T("Gates of Hell", ["400750"], ["call_to_arms_ed.exe"], "military", "war-room khaki", b=SP, n=SPN),
T("Forts", ["410900"], ["forts.exe"], "military", "fortress grey", b=SP, n=SPN),
T("Mindustry", ["1127400"], ["mindustry.exe"], "industrial", "factory orange", b=SP, n=SPN),
T("Kingdoms and Castles", ["569480"], ["kingdomsandcastles.exe"], "parchment", "castle stone and gold", b=SP, n=SPN),
T("Amazing Cultivation Simulator", ["955900"], ["amazing cultivation simulator.exe"], "mystic", "xianxia jade", b=SP, n=SPN),
T("Sands of Salzaar", ["1094520"], ["desertlegend.exe"], "parchment", "desert sand", n="A top-down trading and war RPG; the stick pans the map."),
T("Hero's Adventure: Road to Passion", ["1948980"], ["wulin.exe"], "parchment", "ink and jade"),
T("Wandering Sword", ["1876890"], ["jh-win64-shipping.exe"], "parchment", "ink and jade"),
T("Path of Wuxia", ["1189630"], [], "parchment", "ink and jade"),
T("Tale of Immortal", ["1468810"], [], "mystic", "xianxia jade"),
T("War Selection", ["1022450"], ["glyphengine.exe"], "military", "war-room khaki"),
T("Lobotomy Corporation", ["568220"], ["lobotomycorp.exe"], "noir", "containment grey and red", b=SP, n=SPN),
P("Risk: Global Domination", ["1128810"], [], "military", "war-table khaki"),
P("Business Tour", ["397900"], ["businesstour.exe"], "casino", "board-game green"),
P("Liar's Bar", ["3097560"], ["liar's bar.exe"], "noir", "dive-bar amber"),
P("Chinese Parents", ["736190"], [], "paper", "paper grey"),
P("Leaf Blower Revolution", ["1468260"], [], "calm", "garden green"),
P("Cell to Singularity", ["977400"], ["celltosingularity.exe"], "calm", "evolution teal"),
P("IdleOn", ["1476970"], ["legendsofidleon.exe"], "calm", "idle-world teal"),
P("InZOI", ["2456740"], ["inzoi-win64-shipping.exe"], "calm", "life-sim pastel"),
P("Volcano Princess", ["1669980"], ["volcanoprincess.exe"], "mystic", "volcano ember"),
P("World of Guns: Gun Disassembly", ["262410"], ["disasm.exe"], "industrial", "workbench steel", n="Every part is dragged with the mouse."),
P("Call of Cthulhu: Dark Corners of the Earth", ["22340"], [], "horror", "Innsmouth green", n="A first-person horror game; the right stick is the mouse-look."),
]
