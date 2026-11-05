from helpers import *
# Games Dear Villagers publishes, not covered by an earlier pack.
# Selection: a live Steam game whose Steam publisher is Dear Villagers (wiki names it too), no pack by Steam id or name.
# P = passthrough, for games with controller support (PS = passthrough plus a gyro profile, for shooters); T = strategy with a pointer;
# C / CW = pointer (and WASD) pack; F = mouse-and-keyboard shooter with a gyro profile; KB = keyboard game (pad support before XInput or none).
# Controller support: Steam's flag (full or partial) when it has one, else the PCGamingWiki snapshot; "dualshock" / "switch_pro" are ticked only where the wiki says so.
# Steam ids are each game's base app; executables come from Steam's launch data (Discord's list where Steam has none) and are left out when the name is generic
# or another pack already uses it (that pack then matches by Steam id alone). Genre-based kind and theme were assigned by script and corrected by hand for the
# pointer and keyboard games.
# Left out everywhere: VR games, DLC, demos, tools and soundtracks, games with no release date or not out yet, games delisted from Steam, and games that already have a pack.
S = []
S += [
P("1348 Ex Voto", ["1895900"], ["1348.exe"], "coldsteel", "steel blue and grey."),
P("Ashwalkers", ["1273690"], ["ashwalkers  a survival journey.exe"], "coldsteel", "steel blue and grey."),
P("Astria Ascending", ["1121780"], ["astria ascending.exe", "astria.exe"], "mystic", "violet and gold."),
P("Big Helmet Heroes", ["2397250"], ["bighelmetheroes.exe"], "coldsteel", "steel blue and grey."),
P("Caravan SandWitch", ["1582650"], ["caravansandwitch.exe"], "mystic", "violet and gold.", ctl=["dualshock"]),
P("Deer & Boy", ["1803140"], ["deerandboy.exe"], "calm", "soft pastel tones.", ctl=["dualshock"]),
P("Dive or Die: Children of Rain", ["3590290"], ["dive or die.exe"], "nature", "leaf green and earth brown.", ctl=["dualshock"]),
T("Fabledom", ["1651560"], ["fabledom.exe"], "mystic", "violet and gold"),
P("Foretales", ["1170080"], ["foretales.exe"], "calm", "soft pastel tones."),
P("Fort Solis", ["1931730"], ["fortsolis.exe", "sycamore.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
P("Frog Climbers", ["485120"], ["frogclimbers.exe"], "coldsteel", "steel blue and grey."),
P("Guild of Ascension", ["1169520"], ["guildofascension.exe"], "mystic", "violet and gold."),
P("Machinika: Atlas", ["2817620"], ["machinikaatlas.exe"], "calm", "soft pastel tones.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Nocturnal", ["1634080"], ["nocturnal.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
P("Recompile", ["986310"], ["recompile.exe"], "coldsteel", "steel blue and grey."),
P("Revita", ["1175460"], ["revita.exe"], "coldsteel", "steel blue and grey."),
P("ScourgeBringer", ["1037020"], ["scourgebringer.exe"], "coldsteel", "steel blue and grey."),
P("Souldiers", ["1419160"], ["souldiers.exe"], "calm", "soft pastel tones.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Sports: Renovations", ["2230960"], ["sports renovations.exe"], "coldsteel", "steel blue and grey."),
P("Star Overdrive", ["2055590"], ["staroverdrive.exe"], "scifi", "cold blue and cyan.", ctl=["dualshock"]),
P("Terra Memoria", ["1912750"], ["terra memoria.exe"], "scifi", "cold blue and cyan.", ctl=["dualshock"]),
P("The Dungeon of Naheulbeuk: The Amulet of Chaos", ["970830"], ["naheulbeuk.exe"], "mystic", "violet and gold."),
P("The Forgotten City", ["874260"], ["modernstoryteller01.exe"], "coldsteel", "steel blue and grey."),
P("Vampire: The Masquerade - Coteries of New York", ["1096410"], ["vtm coteries of new york.exe"], "horror", "blood red and grey."),
P("Vampire: The Masquerade - Reckoning of New York", ["2658720"], ["vtm reckoning of new york.exe"], "horror", "blood red and grey."),
P("Vampire: The Masquerade - Shadows of New York", ["1279630"], ["vtm shadows of new york.exe"], "horror", "blood red and grey."),
]
