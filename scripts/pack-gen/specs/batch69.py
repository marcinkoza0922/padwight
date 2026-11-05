from helpers import *
# Games Apogee Entertainment publishes, not covered by an earlier pack.
# Selection: a live Steam game whose Steam publisher is Apogee Entertainment (wiki names it too), no pack by Steam id or name.
# P = passthrough, for games with controller support (PS = passthrough plus a gyro profile, for shooters); T = strategy with a pointer;
# C / CW = pointer (and WASD) pack; F = mouse-and-keyboard shooter with a gyro profile; KB = keyboard game (pad support before XInput or none).
# Controller support: Steam's flag (full or partial) when it has one, else the PCGamingWiki snapshot; "dualshock" / "switch_pro" are ticked only where the wiki says so.
# Steam ids are each game's base app; executables come from Steam's launch data (Discord's list where Steam has none) and are left out when the name is generic
# or another pack already uses it (that pack then matches by Steam id alone). Genre-based kind and theme were assigned by script and corrected by hand for the
# pointer and keyboard games.
# Left out everywhere: VR games, DLC, demos, tools and soundtracks, games with no release date or not out yet, games delisted from Steam, and games that already have a pack.
S = []
S += [
P("Below the Stone", ["1170230"], ["below the stone.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
PS("Blake Stone: Aliens of Gold", ["358190"], [], "horror", "blood red and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
PS("Blake Stone: Planet Strike", ["358310"], [], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Cosmo's Cosmic Adventure", ["358250"], [], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Crystal Caves", ["358260"], [], "pixel", "bright pixel-art colors.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Crystal Caves HD", ["1330890"], ["crystal caves hd.exe"], "pixel", "bright pixel-art colors."),
P("Hocus Pocus", ["358290"], [], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
PS("Major Stryker", ["358300"], [], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Math Rescue", ["358200"], [], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Monster Bash", ["358210"], [], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Monster Bash HD", ["459840"], ["monster bash hd.exe"], "coldsteel", "steel blue and grey."),
P("Mystic Towers", ["358220"], [], "calm", "soft pastel tones.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Paganitzu", ["358230"], [], "calm", "soft pastel tones.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Quest Master", ["2094070"], ["quest master.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
P("Realms of Chaos", ["358320"], [], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
PS("Rise of the Triad", ["217140"], [], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
PS("Rise of the Triad: Dark War", ["358410"], [], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
PS("Rise of the Triad: Ludicrous Edition", ["1421490"], ["rottex.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock", "switch_pro"]),
PS("Secret Agent", ["358350"], [], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
PS("Secret Agent HD", ["1607670"], ["secret agent hd.exe"], "coldsteel", "steel blue and grey."),
PS("Stargunner", ["358390"], [], "scifi", "cold blue and cyan.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Total Chaos", ["2208350"], ["total chaos.exe"], "horror", "blood red and grey.", ctl=["dualshock"]),
PS("Turbo Overkill", ["1328350"], ["turbo overkill.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Wacky Wheels", ["358380"], [], "fighter", "arena red and gold.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
PS("Wizordum", ["1715590"], ["wizordum.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Word Rescue", ["358340"], [], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
]
