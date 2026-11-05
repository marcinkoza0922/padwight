from helpers import *
# Games 3D Realms publishes, not covered by an earlier pack.
# Selection: a live Steam game whose Steam publisher is 3D Realms (wiki names it too), no pack by Steam id or name.
# P = passthrough, for games with controller support (PS = passthrough plus a gyro profile, for shooters); T = strategy with a pointer;
# C / CW = pointer (and WASD) pack; F = mouse-and-keyboard shooter with a gyro profile; KB = keyboard game (pad support before XInput or none).
# Controller support: Steam's flag (full or partial) when it has one, else the PCGamingWiki snapshot; "dualshock" / "switch_pro" are ticked only where the wiki says so.
# Steam ids are each game's base app; executables come from Steam's launch data (Discord's list where Steam has none) and are left out when the name is generic
# or another pack already uses it (that pack then matches by Steam id alone). Genre-based kind and theme were assigned by script and corrected by hand for the
# pointer and keyboard games.
# Left out everywhere: VR games, DLC, demos, tools and soundtracks, games with no release date or not out yet, games delisted from Steam, and games that already have a pack.
S = []
S += [
P("Alien Carnage", ["358280"], [], "horror", "blood red and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Arctic Adventure", ["358170"], [], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
KB("Balls of Steel", ["358430"], [], "pixel", "bright pixel-art colors"),
PS("Bio Menace", ["358180"], [], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
PS("Bombshell", ["353190"], ["bombshell.exe"], "mystic", "violet and gold."),
PS("Cultic", ["1684930"], ["cultic.exe"], "coldsteel", "steel blue and grey."),
KB("Dark Ages", ["358440"], [], "pixel", "bright pixel-art colors"),
P("Death Rally", ["358270"], ["dr.exe"], "fighter", "arena red and gold.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
KB("Duke Nukem", ["240160"], [], "pixel", "bright pixel-art colors", n="The game has some controller support, but it predates XInput or is limited, so this pack plays it with the keyboard keys."),
KB("Duke Nukem II", ["240180"], [], "pixel", "bright pixel-art colors", n="The game has some controller support, but it predates XInput or is limited, so this pack plays it with the keyboard keys."),
PS("Duke Nukem: Manhattan Project", ["240200"], ["prism3d.exe"], "pixel", "bright pixel-art colors.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Graveball", ["563170"], ["graveball.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
PS("Graven", ["1371690"], ["praest-win64-shipping.exe"], "mystic", "violet and gold."),
PS("Ion Fury", ["562860"], ["fury.exe"], "coldsteel", "steel blue and grey."),
P("Monuments of Mars", ["358240"], [], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
PS("Painkiller (2025)", ["2300120"], ["painkiller.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
PS("Phantom Fury", ["1733240"], [], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Pharaoh's Tomb", ["358330"], [], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
PS("Rad Rodgers: Radical Edition", ["805660"], [], "coldsteel", "steel blue and grey."),
PS("Rad Rodgers: World One", ["353580"], [], "coldsteel", "steel blue and grey."),
PS("Radikal Fighters", ["2073930"], ["rfgame.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
PS("Ripout", ["1558830"], ["spacemyths.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("The Kindeman Remedy", ["2055410"], ["tkr.exe"], "coldsteel", "steel blue and grey."),
PS("Twisted Tower", ["1575990"], ["twisted tower.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
PS("Wrath: Aeon of Ruin", ["1000410"], ["wrath-steam.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Xenophage: Alien Bloodsport", ["358420"], [], "horror", "blood red and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
]
