from helpers import *
# Games HypeTrain Digital publishes, not covered by an earlier pack.
# Selection: a live Steam game whose Steam publisher is HypeTrain Digital (wiki names it too), no pack by Steam id or name.
# P = passthrough, for games with controller support (PS = passthrough plus a gyro profile, for shooters); T = strategy with a pointer;
# C / CW = pointer (and WASD) pack; F = mouse-and-keyboard shooter with a gyro profile; KB = keyboard game (pad support before XInput or none).
# Controller support: Steam's flag (full or partial) when it has one, else the PCGamingWiki snapshot; "dualshock" / "switch_pro" are ticked only where the wiki says so.
# Steam ids are each game's base app; executables come from Steam's launch data (Discord's list where Steam has none) and are left out when the name is generic
# or another pack already uses it (that pack then matches by Steam id alone). Genre-based kind and theme were assigned by script and corrected by hand for the
# pointer and keyboard games.
# Left out everywhere: VR games, DLC, demos, tools and soundtracks, games with no release date or not out yet, games delisted from Steam, and games that already have a pack.
S = []
S += [
PS("12 Is Better than 6", ["410110"], ["12ibt6.exe"], "coldsteel", "steel blue and grey."),
P("Barrier X", ["463150"], ["barrierx.exe"], "coldsteel", "steel blue and grey."),
P("Black Book", ["1138660"], ["black book.exe"], "mystic", "violet and gold."),
P("Breathedge", ["738520"], ["breathedge.exe"], "nature", "leaf green and earth brown."),
P("Breathedge 2", ["2412960"], ["breathedge2.exe"], "nature", "leaf green and earth brown.", ctl=["dualshock"]),
P("Desolate", ["671510"], ["sh-win64-shipping.exe"], "horror", "blood red and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Intravenous 2", ["2608270"], ["intravenous2.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
P("Intravenous 2: Mercenarism", ["2651330"], ["intravenous2_mercenarism.exe"], "coldsteel", "steel blue and grey."),
P("Jack Move", ["1099640"], ["jack_move_win.exe"], "mystic", "violet and gold."),
T("Nordhold", ["3028310"], ["nordhold.exe"], "parchment", "parchment brown and gold", exenote=" The Windows executable is from Steam's launch data."),
PS("Outbreak Island", ["1442530"], ["oi_world.exe"], "horror", "blood red and grey."),
PS("Paperhead", ["2680280"], ["paperhead.exe"], "paper", "paper cream and ink."),
P("Pathologic 3", ["3199650"], [], "horror", "blood red and grey.", ctl=["dualshock"]),
C("Pathologic 3: Quarantine", ["3389330"], [], "horror", "blood red and grey"),
PS("Police Stories", ["539470"], [], "scifi", "cold blue and cyan."),
PS("Police Stories: The Academy", ["1986760"], [], "scifi", "cold blue and cyan."),
P("StormEdge", ["2321350"], ["stormedge.exe"], "coldsteel", "steel blue and grey."),
P("SuchArt: Genius Artist Simulator", ["1293180"], ["suchart.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("The Wild Eight", ["526160"], ["wildeight.exe"], "mystic", "violet and gold."),
P("There Is No Light", ["1132980"], ["thereisnolight.exe"], "mystic", "violet and gold."),
P("Tunche", ["887450"], ["tunche.exe"], "coldsteel", "steel blue and grey."),
PS("Voidtrain", ["1159690"], ["voidtrain.exe"], "nature", "leaf green and earth brown.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
]
