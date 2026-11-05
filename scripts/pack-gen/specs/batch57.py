from helpers import *
# Games Iceberg Interactive publishes, not covered by an earlier pack.
# Selection: a live Steam game whose Steam publisher is Iceberg Interactive (wiki names it too), no pack by Steam id or name.
# P = passthrough, for games with controller support (PS = passthrough plus a gyro profile, for shooters); T = strategy with a pointer;
# C / CW = pointer (and WASD) pack; F = mouse-and-keyboard shooter with a gyro profile; KB = keyboard game (pad support before XInput or none).
# Controller support: Steam's flag (full or partial) when it has one, else the PCGamingWiki snapshot; "dualshock" / "switch_pro" are ticked only where the wiki says so.
# Steam ids are each game's base app; executables come from Steam's launch data (Discord's list where Steam has none) and are left out when the name is generic
# or another pack already uses it (that pack then matches by Steam id alone). Genre-based kind and theme were assigned by script and corrected by hand for the
# pointer and keyboard games.
# Left out everywhere: VR games, DLC, demos, tools and soundtracks, games with no release date or not out yet, games delisted from Steam, and games that already have a pack.
S = []
S += [
P("Adam's Venture Episode 1: The Search for the Lost Garden", ["108100"], [], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Adam's Venture Episode 2: Solomon's Secret", ["210390"], [], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Adam's Venture Episode 3: Revelations", ["214310"], [], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
C("Ambition: A Minuet in Power", ["949200"], ["ambition a minuet in power.exe"], "parchment", "parchment brown and gold", exenote=" The Windows executable is from Steam's launch data."),
P("Antigraviator", ["621020"], ["antigraviator.exe"], "fighter", "arena red and gold."),
T("Armada 2526", ["46790"], ["armada2526.exe"], "scifi", "cold blue and cyan", exenote=" The Windows executable is from Steam's launch data."),
P("Blazing Sails: Pirate Battle Royale", ["1158940"], ["blazingsails.exe", "blazingsails-win64-shipping.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
CW("Ceres", ["351300"], ["ceres.exe"], "scifi", "cold blue and cyan", exenote=" The Windows executable is from Steam's launch data."),
T("Circle Empires", ["834740"], ["circle empires.exe"], "parchment", "parchment brown and gold", exenote=" The Windows executable is from Steam's launch data."),
T("Circle Empires Rivals", ["1100260"], ["circle empires rivals.exe"], "parchment", "parchment brown and gold", exenote=" The Windows executable is from Steam's launch data."),
P("Conarium", ["313780"], ["conarium.exe"], "coldsteel", "steel blue and grey."),
PS("Dark Matter", ["251410"], ["darkmatter.exe"], "calm", "soft pastel tones."),
C("Darkness Within 2: The Dark Lineage", ["298950"], ["darklineage.exe"], "horror", "blood red and grey", exenote=" The Windows executable is from Steam's launch data."),
C("Darkness Within: In Pursuit of Loath Nolder", ["298930"], ["darknesswithin.exe"], "horror", "blood red and grey", exenote=" The Windows executable is from Steam's launch data."),
T("Dawn of Andromeda", ["427570"], ["dawn of andromeda.exe"], "scifi", "cold blue and cyan", exenote=" The Windows executable is from Steam's launch data."),
P("Dead Season", ["2666550"], ["dead season.exe"], "coldsteel", "steel blue and grey."),
P("Doomblade", ["922050"], ["doomblade.exe"], "scifi", "cold blue and cyan.", ctl=["dualshock"]),
P("Empathy: Path of Whispers", ["291690"], ["empathy.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Headsnatchers", ["797410"], ["headsnatchers.exe"], "coldsteel", "steel blue and grey."),
T("Heroes of Valor", ["2504090"], ["heroesofvalor.exe"], "mystic", "violet and gold", exenote=" The Windows executable is from Steam's launch data."),
P("Inmates", ["634120"], ["inmates.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Inside My Radio", ["357720"], ["insidemyradio.exe"], "coldsteel", "steel blue and grey."),
CW("Into the Stars", ["360510"], ["nebraska.exe"], "scifi", "cold blue and cyan", exenote=" The Windows executable is from Steam's launch data."),
T("King of Retail", ["968250"], ["kingofretail.exe"], "calm", "soft pastel tones", exenote=" The Windows executable is from Steam's launch data."),
T("Land of the Vikings", ["1981570"], ["vikingoyunu.exe"], "parchment", "parchment brown and gold", exenote=" The Windows executable is from Steam's launch data."),
T("Lords of the Black Sun", ["246940"], ["lords of the black sun.exe"], "scifi", "cold blue and cyan", exenote=" The Windows executable is from Steam's launch data."),
C("Lunacy: Saint Rhodes", ["874450"], ["lunacy.exe"], "horror", "blood red and grey", exenote=" The Windows executable is from Steam's launch data."),
C("Mah\u014dkenshi", ["1717640"], ["mahokenshi.exe"], "mystic", "violet and gold", exenote=" The Windows executable is from Steam's launch data.", slug="mahokenshi"),
C("Midnight Protocol", ["1162700"], ["midnightprotocol.exe"], "scifi", "cold blue and cyan", exenote=" The Windows executable is from Steam's launch data."),
P("Nuclear Dawn", ["17710"], ["nucleardawn.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
T("Oriental Empires", ["357310"], ["oriental empires.exe"], "parchment", "parchment brown and gold", exenote=" The Windows executable is from Steam's launch data."),
T("Pax Nova", ["804850"], ["pax nova.exe"], "scifi", "cold blue and cyan", exenote=" The Windows executable is from Steam's launch data."),
P("Radical Relocation", ["816430"], ["radical relocation.exe"], "calm", "soft pastel tones."),
T("Railroad Corporation", ["797400"], ["railroadcorporationsteam.exe"], "industrial", "steel grey and orange", exenote=" The Windows executable is from Steam's launch data."),
T("Railroad Corporation 2", ["1677970"], ["railroadcorporation2.exe"], "industrial", "steel grey and orange", exenote=" The Windows executable is from Steam's launch data."),
P("Recursive Ruin", ["1561890"], ["recursive ruin.exe"], "calm", "soft pastel tones."),
T("Sacred Fire", ["900400"], ["sacred fire.exe"], "mystic", "violet and gold", exenote=" The Windows executable is from Steam's launch data."),
T("Star Dynasties", ["1194590"], ["stardynasties.exe"], "parchment", "parchment brown and gold", exenote=" The Windows executable is from Steam's launch data."),
T("StarDrive 2", ["252450"], ["sd2.exe"], "scifi", "cold blue and cyan", exenote=" The Windows executable is from Steam's launch data."),
T("Stars in Shadow", ["464880"], ["sis.exe", "sis64.exe"], "scifi", "cold blue and cyan", exenote=" The Windows executable is from Steam's launch data."),
T("Starship Corporation", ["292330"], ["starshipcorporation.exe"], "scifi", "cold blue and cyan", exenote=" The Windows executable is from Steam's launch data."),
C("Still There", ["1063490"], ["still there.exe"], "noir", "noir grey and amber", exenote=" The Windows executable is from Steam's launch data."),
P("Strange Horticulture", ["1574580"], ["strange horticulture.exe"], "calm", "soft pastel tones."),
C("Tech Support: Error Unknown", ["781480"], ["techsupport.exe"], "horror", "blood red and grey", exenote=" The Windows executable is from Steam's launch data."),
C("The Good Life", ["293340"], ["tgl.exe"], "nature", "leaf green and earth brown", exenote=" The Windows executable is from Steam's launch data."),
P("The Sojourn", ["794960"], ["thesojourn.exe"], "calm", "soft pastel tones."),
P("Transient", ["962630"], ["transient.exe"], "coldsteel", "steel blue and grey."),
P("Vector Thrust", ["242130"], ["vectorthrust.exe"], "coldsteel", "steel blue and grey."),
]
