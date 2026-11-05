from helpers import *
# Games Degica, Bigben Interactive, Merge Games publishes, not covered by an earlier pack.
# Selection: a live Steam game whose Steam publisher is Degica, Bigben Interactive, Merge Games (wiki names it too), no pack by Steam id or name.
# P = passthrough, for games with controller support (PS = passthrough plus a gyro profile, for shooters); T = strategy with a pointer;
# C / CW = pointer (and WASD) pack; F = mouse-and-keyboard shooter with a gyro profile; KB = keyboard game (pad support before XInput or none).
# Controller support: Steam's flag (full or partial) when it has one, else the PCGamingWiki snapshot; "dualshock" / "switch_pro" are ticked only where the wiki says so.
# Steam ids are each game's base app; executables come from Steam's launch data (Discord's list where Steam has none) and are left out when the name is generic
# or another pack already uses it (that pack then matches by Steam id alone). Genre-based kind and theme were assigned by script and corrected by hand for the
# pointer and keyboard games.
# Left out everywhere: VR games, DLC, demos, tools and soundtracks, games with no release date or not out yet, games delisted from Steam, and games that already have a pack.
S = []
S += [
C("Adventurezator: When Pigs Fly", ["300280"], [], "mystic", "violet and gold"),
T("Blood Bowl: Death Zone", ["602490"], ["bbdz.exe"], "fighter", "arena red and gold", exenote=" The Windows executable is from Steam's launch data."),
P("Zombie Kill of the Week - Reborn", ["342300"], ["reborn.exe"], "horror", "blood red and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
]
