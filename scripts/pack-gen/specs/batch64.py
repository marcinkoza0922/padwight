from helpers import *
# Games Wired Productions publishes, not covered by an earlier pack.
# Selection: a live Steam game whose Steam publisher is Wired Productions (wiki names it too), no pack by Steam id or name.
# P = passthrough, for games with controller support (PS = passthrough plus a gyro profile, for shooters); T = strategy with a pointer;
# C / CW = pointer (and WASD) pack; F = mouse-and-keyboard shooter with a gyro profile; KB = keyboard game (pad support before XInput or none).
# Controller support: Steam's flag (full or partial) when it has one, else the PCGamingWiki snapshot; "dualshock" / "switch_pro" are ticked only where the wiki says so.
# Steam ids are each game's base app; executables come from Steam's launch data (Discord's list where Steam has none) and are left out when the name is generic
# or another pack already uses it (that pack then matches by Steam id alone). Genre-based kind and theme were assigned by script and corrected by hand for the
# pointer and keyboard games.
# Left out everywhere: VR games, DLC, demos, tools and soundtracks, games with no release date or not out yet, games delisted from Steam, and games that already have a pack.
S = []
S += [
P("Arcade Paradise", ["1388870"], ["arcadeparadise.exe"], "coldsteel", "steel blue and grey."),
PS("Beneath", ["1593230"], ["beneath.exe"], "horror", "blood red and grey."),
P("BrokenLore: Don't Lie", ["3886200"], ["dontlie.exe"], "horror", "blood red and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Bulwark: Falconeer Chronicles", ["290100"], ["bulwark.exe"], "coldsteel", "steel blue and grey."),
P("Close to the Sun", ["968870"], ["ctts.exe"], "coldsteel", "steel blue and grey."),
P("Deliver Us the Moon", ["428660"], ["moonman.exe"], "calm", "soft pastel tones."),
P("Fractured Minds", ["688740"], ["fractured_minds.exe"], "coldsteel", "steel blue and grey."),
P("GRIP: Combat Racing", ["396900"], ["grip.exe"], "fighter", "arena red and gold."),
T("Hotel Architect", ["1602000"], ["hotelarchitect.exe"], "calm", "soft pastel tones", exenote=" The Windows executable is from Steam's launch data."),
P("Karma: The Dark World", ["1376200"], ["karma.exe"], "horror", "blood red and grey.", ctl=["dualshock"]),
P("Last Inua", ["331980"], ["lastinua_x86.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Lumote", ["791240"], ["rengine.exe"], "coldsteel", "steel blue and grey."),
P("Martha Is Dead", ["515960"], ["mid.exe"], "calm", "soft pastel tones."),
P("Super Dungeon Bros", ["306000"], ["superdungeonbros.exe"], "mystic", "violet and gold.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("The Falconeer", ["1135260"], ["thefalconeer.exe", "thefalconeervr.exe"], "coldsteel", "steel blue and grey."),
P("The Last Worker", ["1579280"], ["the last worker.exe"], "coldsteel", "steel blue and grey."),
P("The Town of Light", ["433100"], ["ttol.exe"], "coldsteel", "steel blue and grey."),
P("Those Who Remain", ["715380"], ["thosewhoremain.exe"], "coldsteel", "steel blue and grey."),
P("Tin Hearts", ["1831700"], ["tin hearts.exe"], "coldsteel", "steel blue and grey."),
CW("Tiny Troopers", ["216110"], ["tinytroopers.exe"], "military", "olive drab and khaki", exenote=" The Windows executable is from Steam's launch data."),
PS("Tiny Troopers: Global Ops", ["1374350"], ["tiny troopers global ops.exe"], "coldsteel", "steel blue and grey."),
PS("Tiny Troopers: Joint Ops", ["2225750"], ["ttjo.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Warhammer 40,000: Speed Freeks", ["2078450"], ["speedfreeks.exe"], "military", "olive drab and khaki.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
]
