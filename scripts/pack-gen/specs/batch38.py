from helpers import *
# Games Deep Silver (Plaion, Koch Media) publishes, not covered by an earlier pack.
# Selection: a live Steam game whose Steam publisher is a Deep Silver, Plaion or Koch Media label.
# P = passthrough, for games with controller support (PS = passthrough plus a gyro profile, for shooters); T = strategy with a pointer;
# C / CW = pointer (and WASD) pack; F = mouse-and-keyboard shooter with a gyro profile; KB = keyboard game (pad support before XInput or none).
# Controller support: Steam's flag (full or partial) when it has one, else the PCGamingWiki snapshot; "dualshock" / "switch_pro" are ticked only where the wiki says so.
# Steam ids are each game's base app; executables come from Steam's launch data (Discord's list where Steam has none) and are left out when the name is generic
# or another pack already uses it (that pack then matches by Steam id alone). Genre-based kind and theme were assigned by script and corrected by hand for the
# pointer and keyboard games.
# Left out everywhere: VR games, DLC, demos, tools and soundtracks, games with no release date or not out yet, games delisted from Steam, and games that already have a pack.
S = []
S += [
PS("Agents of Mayhem", ["304530"], ["aom_release_final.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
PS("Akimbot", ["1843540"], ["akimbot.exe"], "coldsteel", "steel blue and grey."),
P("Beyond Contact", ["877010"], ["beyond.exe"], "mystic", "violet and gold.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
PS("Chorus", ["1153640"], ["chorus-win64-shipping.exe", "chorus.exe"], "industrial", "steel grey and orange."),
PS("Clid the Snail", ["1597580"], ["clid the snail.exe"], "coldsteel", "steel blue and grey."),
P("Dead Island 2", ["934700"], ["deadisland.exe"], "horror", "blood red and grey.", ctl=["dualshock"]),
PS("Dead Island", ["91310"], [], "horror", "blood red and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
PS("Dead Island: Definitive Edition", ["383150"], [], "horror", "blood red and grey."),
CW("Dead Island: Epidemic", ["222900"], [], "horror", "blood red and grey"),
PS("Dead Island: Retro Revenge", ["337360"], ["deadislandretrorevenge.exe"], "horror", "blood red and grey."),
P("Dead Island: Riptide Definitive Edition", ["383180"], ["deadislandriptidegame.exe"], "horror", "blood red and grey."),
PS("Dead Island: Riptide", ["216250"], ["deadislandgame_x86_rwdi.exe"], "horror", "blood red and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Deadlight: Director's Cut", ["423950"], [], "coldsteel", "steel blue and grey."),
P("Echoes of the End", ["2821610"], ["thedarken.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard.", ctl=["dualshock"]),
T("Eisenbahn X", ["335310"], [], "industrial", "steel grey and orange"),
T("Emergency 2012", ["40370"], ["em2012.exe"], "industrial", "steel grey and orange", exenote=" The Windows executable is from Steam's launch data."),
T("Emergency 2016", ["404920"], [], "industrial", "steel grey and orange"),
T("Emergency 5", ["328140"], [], "industrial", "steel grey and orange"),
PS("Escape Dead Island", ["226560"], ["escapedeadisland.exe"], "horror", "blood red and grey."),
P("Gods Will Fall", ["1243690"], ["gods will fall milestone 10.exe", "gods will fall milestone 9.exe", "gods will fall.exe"], "coldsteel", "steel blue and grey."),
PS("Homefront", ["55100"], ["homefront.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
PS("Homefront: The Revolution", ["223100"], ["homefront2_release.exe"], "coldsteel", "steel blue and grey."),
F("Iron Front: Digital War Edition", ["91330"], ["ironfront.exe"], "military", "olive drab and khaki", exenote=" The Windows executable is from Steam's launch data."),
P("Iron Harvest", ["826630"], ["ironharvest.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
P("Jane's Advanced Strike Fighters", ["207790"], [], "industrial", "steel grey and orange."),
PS("Killer Is Dead", ["261110"], ["kidgame.exe"], "fighter", "arena red and gold."),
P("Kingdom Come: Deliverance II", ["1771300"], [], "mystic", "violet and gold.", ctl=["dualshock"]),
P("Kingdom Come: Deliverance", ["379430"], [], "mystic", "violet and gold."),
PS("Metal Eden", ["990380"], ["metaleden.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard.", ctl=["dualshock"]),
PS("Mighty No. 9", ["314710"], ["mn9game.exe"], "coldsteel", "steel blue and grey."),
P("Nail'd", ["40380"], ["naild_x86.exe"], "fighter", "arena red and gold.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
PS("Narco Terror", ["225180"], ["nt_win.exe"], "coldsteel", "steel blue and grey."),
P("Nobody Wants to Die", ["1939970"], ["detnoir.exe"], "coldsteel", "steel blue and grey."),
P("Outward", ["794260"], ["outward.exe", "outward definitive edition.exe"], "mystic", "violet and gold."),
T("Railroad X", ["251020"], [], "industrial", "steel grey and orange"),
PS("Red Faction Guerrilla Re-Mars-tered", ["667720"], [], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
F("Red Faction II", ["20550"], ["red faction ii.exe"], "scifi", "cold blue and cyan", exenote=" The Windows executable is from Steam's launch data."),
PS("Red Faction: Armageddon", ["55110"], [], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
PS("Red Faction: Guerrilla", ["20500"], [], "coldsteel", "steel blue and grey."),
PS("Ride to Hell: Retribution", ["209340"], ["ride_to_hell_retribution.exe"], "coldsteel", "steel blue and grey."),
PS("Saints Row IV", ["206420"], ["saintsrowiv.exe"], "coldsteel", "steel blue and grey."),
PS("Saints Row", ["742420"], ["game_dev.exe", "game_dev_dx12.exe", "saintsrow.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
PS("Saints Row: Gat out of Hell", ["301910"], ["saintsrowgatoutofhell.exe"], "coldsteel", "steel blue and grey."),
PS("Saints Row: The Third Remastered", ["978300"], ["srttr.exe"], "coldsteel", "steel blue and grey."),
P("The Siege and the Sandfox", ["653060"], ["sandfox.exe"], "coldsteel", "steel blue and grey."),
P("Undisputed", ["1451190"], ["undisputed.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
C("Who Wants to Be a Millionaire? Special Editions", ["206740"], ["shippingpc-wwtbamgame.exe"], "calm", "soft pastel tones", exenote=" The Windows executable is from Steam's launch data."),
P("Windbound", ["1162130"], ["windboundwindows.exe"], "mystic", "violet and gold."),
]
