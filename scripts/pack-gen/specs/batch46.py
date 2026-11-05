from helpers import *
# Games Raw Fury publishes, not covered by an earlier pack.
# Selection: a live Steam game whose Steam publisher is Raw Fury (wiki names it too), no pack by Steam id or name.
# P = passthrough, for games with controller support (PS = passthrough plus a gyro profile, for shooters); T = strategy with a pointer;
# C / CW = pointer (and WASD) pack; F = mouse-and-keyboard shooter with a gyro profile; KB = keyboard game (pad support before XInput or none).
# Controller support: Steam's flag (full or partial) when it has one, else the PCGamingWiki snapshot; "dualshock" / "switch_pro" are ticked only where the wiki says so.
# Steam ids are each game's base app; executables come from Steam's launch data (Discord's list where Steam has none) and are left out when the name is generic
# or another pack already uses it (that pack then matches by Steam id alone). Genre-based kind and theme were assigned by script and corrected by hand for the
# pointer and keyboard games.
# Left out everywhere: VR games, DLC, demos, tools and soundtracks, games with no release date or not out yet, games delisted from Steam, and games that already have a pack.
S = []
S += [
P("American Arcadia", ["1249040"], ["aarcadia.exe"], "calm", "soft pastel tones."),
PS("Atomicrops", ["757320"], ["atomicrops.exe"], "coldsteel", "steel blue and grey."),
P("Bad North", ["688420"], ["badnorth.exe"], "coldsteel", "steel blue and grey."),
P("Ballionaire", ["2667120"], ["ballionaire-windows.exe"], "coldsteel", "steel blue and grey."),
P("Blue Prince", ["1569580"], ["blue prince.exe"], "calm", "soft pastel tones.", ctl=["dualshock"]),
P("Call of the Sea", ["1042490"], ["cots.exe"], "military", "olive drab and khaki."),
P("Cassette Beasts", ["1321440"], ["cassettebeasts.exe", "workshoputility.exe"], "mystic", "violet and gold."),
P("Dandara", ["612390"], ["dandara.exe"], "coldsteel", "steel blue and grey."),
P("Deep Dish Dungeon", ["2871520"], ["deep dish dungeon.exe"], "mystic", "violet and gold."),
PS("Dome Keeper", ["1637320"], ["domekeeper.exe", "workshopuploader.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
P("Dream Cycle", ["1105590"], ["dreamcycle.exe"], "coldsteel", "steel blue and grey."),
P("Esoteric Ebb", ["2057760"], ["esoteric ebb.exe"], "mystic", "violet and gold.", ctl=["dualshock"]),
T("Flat Eye", ["1358840"], ["flat-eye-win64.exe"], "scifi", "cold blue and cyan", exenote=" The Windows executable is from Steam's launch data."),
PS("Friends vs Friends", ["1785150"], [], "coldsteel", "steel blue and grey."),
P("Gonner", ["437570"], ["gonner.exe"], "coldsteel", "steel blue and grey."),
PS("Gonner2", ["1117670"], ["gonner2.exe"], "coldsteel", "steel blue and grey."),
C("Kathy Rain", ["370910"], ["kathyrain.exe"], "noir", "noir grey and amber", exenote=" The Windows executable is from Steam's launch data."),
C("Kathy Rain 2: Soothsayer", ["1466390"], ["kathyrain2.exe"], "noir", "noir grey and amber", exenote=" The Windows executable is from Steam's launch data."),
P("Kathy Rain: Director's Cut", ["1395030"], ["kathyraindirectorscut.exe"], "coldsteel", "steel blue and grey."),
P("Kingdom: Eighties", ["1956040"], ["kingdom80s.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
P("Kingdom: New Lands", ["496300"], [], "coldsteel", "steel blue and grey."),
P("Kingdom: Two Crowns", ["701160"], ["kingdomtwocrowns.exe"], "coldsteel", "steel blue and grey."),
P("Knights in Tight Spaces", ["2315400"], ["knightsintightspaces.exe"], "scifi", "cold blue and cyan.", ctl=["dualshock"]),
P("KuloNiku: Bowl Up!", ["3357960"], ["kuloniku.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
PS("Last Man Sitting", ["774291"], ["lmsgame.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Monsters are Coming! Rock & Road", ["2934220"], ["monsterscoming.exe"], "coldsteel", "steel blue and grey."),
P("Moonstone Island", ["1658150"], ["moonstone island.exe"], "mystic", "violet and gold."),
P("Mosaic", ["349270"], ["mosaic.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Mr. Sun's Hatbox", ["1214670"], ["hatbox.exe"], "coldsteel", "steel blue and grey."),
P("Night Call", ["680380"], ["night call.exe"], "coldsteel", "steel blue and grey."),
P("Norco", ["1221250"], ["norco.exe"], "coldsteel", "steel blue and grey."),
T("Per Aspera", ["803050"], ["per aspera.exe"], "scifi", "cold blue and cyan", exenote=" The Windows executable is from Steam's launch data."),
P("Pizza Possum", ["1951230"], ["pizzapossum.exe"], "coldsteel", "steel blue and grey."),
P("Post Trauma", ["1750030"], ["ue5_posttrauma.exe"], "horror", "blood red and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard.", ctl=["dualshock"]),
P("Regions of Ruin: Runegate", ["1375750"], ["regions of ruin runegate.exe"], "mystic", "violet and gold."),
P("Rivage", ["4094660"], ["rivage.exe"], "calm", "soft pastel tones."),
P("Routine", ["606160"], ["routine.exe"], "horror", "blood red and grey."),
P("Sable", ["757310"], ["sable.exe"], "fighter", "arena red and gold."),
P("Sinner: Sacrifice for Redemption", ["691280"], ["blackstar_uproject.exe"], "mystic", "violet and gold.", ctl=["dualshock"]),
P("Skald: Against the Black Priory", ["1069160"], ["skald against the black priory.exe"], "mystic", "violet and gold."),
P("Snufkin: Melody of Moominvalley", ["1808680"], ["snufkin.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
P("Star Renegades", ["651670"], ["star renegades.exe", "starrenegades.exe"], "scifi", "cold blue and cyan."),
P("Star Trucker", ["2380050"], ["star trucker.exe"], "scifi", "cold blue and cyan."),
CW("Superfuse", ["1770080"], ["superfuse.exe"], "scifi", "cold blue and cyan", exenote=" The Windows executable is from Steam's launch data."),
P("Tails Noir", ["865610"], ["detective.exe"], "mystic", "violet and gold."),
P("Tails Noir Preludes", ["2020030"], ["backbonestories.exe"], "noir", "noir grey and amber."),
P("The Longest Road on Earth", ["1295790"], ["the longest road on earth.exe"], "coldsteel", "steel blue and grey."),
P("The Signifier", ["1082930"], ["thesignifier.exe"], "calm", "soft pastel tones."),
P("The S\u00e9ance of Blake Manor", ["1395520"], ["the seance of blake manor.exe"], "calm", "soft pastel tones.", ctl=["dualshock"], slug="the-seance-of-blake-manor"),
PS("Tormentor X Punisher", ["500670"], ["tormentorxpunisher.exe"], "mystic", "violet and gold.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Townscaper", ["1291340"], ["townscaper.exe"], "coldsteel", "steel blue and grey."),
P("Uurnog Uurnlimited", ["678850"], ["uurnog.exe"], "coldsteel", "steel blue and grey."),
PS("West of Dead", ["1016790"], ["westofdead.exe"], "noir", "noir grey and amber.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
C("Whispers of a Machine", ["631570"], ["whispers.exe"], "noir", "noir grey and amber", exenote=" The Windows executable is from Steam's launch data."),
P("Wolfstride", ["1331210"], ["wolfstride.exe"], "mystic", "violet and gold."),
]
