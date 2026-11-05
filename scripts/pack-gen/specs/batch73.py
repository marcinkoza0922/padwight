from helpers import *
# SNK Corporation's Steam games (Neo Geo classics, Metal Slug, King of Fighters, Samurai Shodown, Fatal Fury, collections).
# Selection: every SNK game on Steam with a base app, found by title search and the wiki; the ACA NEOGEO series is Windows Store only and has no Steam id.
# P = passthrough, for games with controller support (PS = passthrough plus a gyro profile, for shooters); T = strategy with a pointer;
# C / CW = pointer (and WASD) pack; F = mouse-and-keyboard shooter with a gyro profile; KB = keyboard game (pad support before XInput or none).
# Controller support: Steam's flag (full or partial) when it has one, else the PCGamingWiki snapshot; "dualshock" / "switch_pro" are ticked only where the wiki says so.
# Steam ids are each game's base app (editions the wiki lists too); executables come from Steam's launch data and are left out when the name is generic
# or another pack already uses it (that pack then matches by Steam id alone). Kind and theme were assigned by script from the wiki's genres and corrected by hand for the first groups.
# Left out everywhere: VR games, DLC, demos, tools and soundtracks, games with no release date or not out yet, delisted games, adult games, the publishers listed in the user's exclusions, and games that already have a pack.
S = []
S += [
P("SNK 40th Anniversary Collection", ["865940"], ["snk40thcollection.exe"], "pixel", "night blue and gold."),
P("SNK vs. Capcom: SVC Chaos", ["2442380"], ["svc chaos.exe"], "fighter", "deep purple with red and yellow.", ctl=["dualshock"]),
P("SNK Heroines: Tag Team Frenzy", ["794580"], ["snkheroines.exe"], "fighter", "deep purple with red and yellow."),
P("SNK vs. Capcom: The Match of the Millennium", ["1575670"], [], "fighter", "deep purple with red and yellow."),
P("NeoGeo Pocket Color Selection Vol. 1 Steam Edition", ["1586350"], [], "pixel", "night blue and gold."),
P("NeoGeo Pocket Color Selection Vol. 2", ["1983850"], [], "pixel", "night blue and gold."),
P("Samurai Shodown V Special", ["1076550"], ["ssvs.exe"], "samurai", "ink black, red and gold."),
P("Samurai Shodown (2020)", ["1342260"], ["samuraishodown.exe"], "samurai", "ink black, red and gold."),
P("Samurai Shodown NeoGeo Collection", ["999660"], ["samurai shodown collection.exe"], "samurai", "ink black, red and gold."),
P("The King of Fighters XV", ["1498570"], ["kofxv_steam.exe"], "fighter", "deep purple with red and yellow."),
P("The King of Fighters '98 Ultimate Match Final Edition", ["222420"], ["kingoffighters98um.exe", "kingoffighters98um_x64.exe"], "fighter", "deep purple with red and yellow.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("The King of Fighters 2002 Unlimited Match", ["222440"], ["kingoffighters2002um.exe", "kingoffighters2002um_x64.exe"], "fighter", "deep purple with red and yellow.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("The King of Fighters XIII Global Match", ["3050220"], [], "fighter", "deep purple with red and yellow.", ctl=["dualshock"]),
P("The King of Fighters XIII Steam Edition", ["222940"], ["kofxiii.exe"], "fighter", "deep purple with red and yellow."),
P("The King of Fighters XIV Steam Edition", ["571260"], ["kofxiv.exe"], "fighter", "deep purple with red and yellow."),
P("The King of Fighters '97 Global Match", ["702120"], ["kof97.exe"], "fighter", "deep purple with red and yellow."),
P("Metal Slug Tactics", ["1590760"], ["metal slug tactics.exe"], "military", "olive drab and khaki.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard.", ctl=["dualshock"]),
P("Metal Slug: Awakening", ["2963870"], [], "military", "olive drab and khaki.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Metal Slug 3", ["250180"], ["mslug3.exe"], "military", "olive drab and khaki."),
P("Metal Slug X", ["312610"], ["mslugx.exe"], "military", "olive drab and khaki."),
P("Metal Slug", ["366250"], ["mslug1.exe"], "military", "olive drab and khaki."),
P("Metal Slug 2", ["366260"], ["mslug2.exe"], "military", "olive drab and khaki."),
P("Metal Slug XX", ["975680"], ["metal slug xx.exe"], "military", "olive drab and khaki."),
P("Metal Slug Defense", ["356310"], ["msdf-steam.exe"], "military", "olive drab and khaki.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Metal Slug Attack Reloaded", ["2322440"], ["msar_steam.exe"], "military", "olive drab and khaki.", ctl=["dualshock"]),
P("Fatal Fury: City of the Wolves", ["2492040"], ["fatal fury city of the wolves.exe"], "fighter", "deep purple with red and yellow.", ctl=["dualshock"]),
P("Real Bout Fatal Fury 2: The Newcomers", ["3359900"], ["rb2.exe"], "fighter", "deep purple with red and yellow."),
P("Baseball Stars 2", ["366230"], ["bstars2.exe"], "pixel", "night blue and gold."),
P("Garou: Mark of the Wolves", ["366240"], ["garou.exe"], "fighter", "deep purple with red and yellow."),
P("The Last Blade", ["465840"], ["lastblade.exe"], "samurai", "ink black, red and gold."),
P("The Last Blade 2", ["702110"], ["lastblade2app.exe"], "samurai", "ink black, red and gold."),
P("Shock Troopers", ["366270"], ["shocktro.exe"], "military", "olive drab and khaki."),
P("Shock Troopers: 2nd Squad", ["465870"], ["shocktr2.exe"], "military", "olive drab and khaki.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Twinkle Star Sprites", ["366280"], ["twinspri.exe"], "pixel", "night blue and gold."),
P("Windjammers 2", ["1114290"], ["windjammers2.exe", "windjammers2_opengl.exe"], "pixel", "night blue and gold."),
P("World Heroes Perfect", ["3822220"], ["world heroes perfect.exe"], "fighter", "deep purple with red and yellow."),
]
