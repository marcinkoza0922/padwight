from helpers import *
# Games DANGEN Entertainment publishes, not covered by an earlier pack.
# Selection: a live Steam game whose Steam publisher is DANGEN Entertainment (wiki names it too), no pack by Steam id or name.
# P = passthrough, for games with controller support (PS = passthrough plus a gyro profile, for shooters); T = strategy with a pointer;
# C / CW = pointer (and WASD) pack; F = mouse-and-keyboard shooter with a gyro profile; KB = keyboard game (pad support before XInput or none).
# Controller support: Steam's flag (full or partial) when it has one, else the PCGamingWiki snapshot; "dualshock" / "switch_pro" are ticked only where the wiki says so.
# Steam ids are each game's base app; executables come from Steam's launch data (Discord's list where Steam has none) and are left out when the name is generic
# or another pack already uses it (that pack then matches by Steam id alone). Genre-based kind and theme were assigned by script and corrected by hand for the
# pointer and keyboard games.
# Left out everywhere: VR games, DLC, demos, tools and soundtracks, games with no release date or not out yet, games delisted from Steam, and games that already have a pack.
S = []
S += [
P("Ara Fell", ["440540"], [], "mystic", "violet and gold."),
P("Astalon: Tears of the Earth", ["1046400"], ["astalon.exe"], "coldsteel", "steel blue and grey."),
P("Batbarian: Testament of the Primordials", ["837460"], ["batbarian testament of the primordials.exe"], "coldsteel", "steel blue and grey."),
P("Bug Fables: The Everlasting Sapling", ["1082710"], ["bug fables.exe"], "mystic", "violet and gold."),
P("Disc Creatures", ["1083190"], [], "mystic", "violet and gold."),
P("Dungeon Drafters", ["1824580"], ["dungeon drafters.exe"], "mystic", "violet and gold."),
P("Evertried", ["1607040"], ["evertried.exe"], "coldsteel", "steel blue and grey."),
P("Hunt the Night", ["1649740"], ["hunt the night.exe"], "mystic", "violet and gold."),
P("Iconoclasts", ["393520"], ["iconoclasts.exe"], "coldsteel", "steel blue and grey."),
P("Kingdom of Night", ["1094600"], ["kingdomofnight.exe"], "mystic", "violet and gold."),
P("Loretta", ["1592540"], ["loretta.exe"], "coldsteel", "steel blue and grey."),
P("Lost Ruins", ["1306630"], ["lostruins.exe"], "nature", "leaf green and earth brown."),
P("Lovish", ["2959010"], ["lovish.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
P("Minoria", ["940910"], ["minoria.exe"], "scifi", "cold blue and cyan."),
P("Rise of the Third Power", ["698700"], ["rise of the third power.exe"], "mystic", "violet and gold."),
P("RoadOut", ["1829270"], ["roadout.exe"], "mystic", "violet and gold."),
P("Smelter", ["1094540"], ["smelter.exe"], "coldsteel", "steel blue and grey."),
P("Super Sportmatchen", ["749860"], ["sportmatchen.exe"], "coldsteel", "steel blue and grey."),
P("The Witch's House MV", ["885810"], [], "mystic", "violet and gold."),
PS("TurretGirls", ["3029750"], ["turretgirls.exe"], "coldsteel", "steel blue and grey."),
P("Ultra Age", ["1683100"], ["pu.exe"], "coldsteel", "steel blue and grey."),
P("Vestaria Saga I: War of the Scions", ["694770"], [], "scifi", "cold blue and cyan.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Vestaria Saga II: The Sacred Sword of Silvanister", ["1811600"], ["vestariasaga2.exe"], "scifi", "cold blue and cyan.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
]
