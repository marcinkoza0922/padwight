from helpers import *
# Q.U.B.E. (Toxic Games), QUByte Interactive's games, and Guacamelee! / Nobody Saves the World (DrinkBox Studios).
# Selection: Q.U.B.E. games and QUByte Interactive's releases from the wiki's publisher field, DrinkBox's Steam games; Glover (Piko) left out.
# P = passthrough, for games with controller support (PS = passthrough plus a gyro profile, for shooters); T = strategy with a pointer;
# C / CW = pointer (and WASD) pack; F = mouse-and-keyboard shooter with a gyro profile; KB = keyboard game (pad support before XInput or none).
# Controller support: Steam's flag (full or partial) when it has one, else the PCGamingWiki snapshot; "dualshock" / "switch_pro" are ticked only where the wiki says so.
# Steam ids are each game's base app (editions the wiki lists too); executables come from Steam's launch data and are left out when the name is generic
# or another pack already uses it (that pack then matches by Steam id alone). Kind and theme were assigned by script from the wiki's genres and corrected by hand for the first groups.
# Left out everywhere: VR games, DLC, demos, tools and soundtracks, games with no release date or not out yet, delisted games, adult games, the publishers listed in the user's exclusions, and games that already have a pack.
S = []
S += [
P("Q.U.B.E.: Director's Cut", ["239430"], ["qubegame.exe"], "scifi", "cold blue and cyan.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Q.U.B.E. 10th Anniversary", ["1564220"], ["qube.exe"], "scifi", "cold blue and cyan."),
P("Q.U.B.E. 2", ["359100"], [], "scifi", "cold blue and cyan."),
P("Qube Qross", ["1301580"], ["qube qross.exe"], "calm", "soft pastel tones."),
P("Breakers Collection", ["1786230"], ["breakerscollection.exe"], "fighter", "deep purple with red and yellow."),
P("VASARA Collection", ["1085630"], ["vasara collection.exe"], "pixel", "night blue and gold.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Top Racer Collection", ["2214000"], ["topracercollection.exe"], "industrial", "steel grey and orange.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Raccoo Venture", ["849210"], ["raccooventure.exe"], "nature", "leaf green and earth brown.", ctl=["dualshock"]),
P("Mars 2120", ["1424290"], [], "scifi", "cold blue and cyan."),
P("Rage of the Dragons NEO", ["2892130"], ["rage_of_the_dragons_neo.exe"], "fighter", "deep purple with red and yellow.", ctl=["dualshock"]),
P("Recruits", ["246880"], [], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("99Vidas", ["557040"], ["99vidasgame_x86.exe", "99vidasgame.exe"], "fighter", "deep purple with red and yellow."),
P("Biomechanical Toy", ["4448040"], ["biomechanical_toy.exe"], "pixel", "night blue and gold.", ctl=["dualshock"]),
P("HTR+ Slot Car Simulation", ["288650"], ["htr_plus.exe"], "industrial", "steel grey and orange.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Guacamelee! Gold Edition", ["214770"], ["guac.exe"], "fighter", "deep purple with red and yellow."),
P("Guacamelee! Super Turbo Championship Edition", ["275390"], [], "fighter", "deep purple with red and yellow."),
P("Guacamelee! 2", ["534550"], ["guac2_x64.exe", "guac2_x86.exe"], "fighter", "deep purple with red and yellow."),
P("Nobody Saves the World", ["1432050"], ["nstw_x64.exe"], "mystic", "violet and gold."),
]
