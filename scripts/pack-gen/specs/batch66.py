from helpers import *
# Games Thunderful publishes, not covered by an earlier pack.
# Selection: a live Steam game whose Steam publisher is Thunderful (wiki names it too), no pack by Steam id or name.
# P = passthrough, for games with controller support (PS = passthrough plus a gyro profile, for shooters); T = strategy with a pointer;
# C / CW = pointer (and WASD) pack; F = mouse-and-keyboard shooter with a gyro profile; KB = keyboard game (pad support before XInput or none).
# Controller support: Steam's flag (full or partial) when it has one, else the PCGamingWiki snapshot; "dualshock" / "switch_pro" are ticked only where the wiki says so.
# Steam ids are each game's base app; executables come from Steam's launch data (Discord's list where Steam has none) and are left out when the name is generic
# or another pack already uses it (that pack then matches by Steam id alone). Genre-based kind and theme were assigned by script and corrected by hand for the
# pointer and keyboard games.
# Left out everywhere: VR games, DLC, demos, tools and soundtracks, games with no release date or not out yet, games delisted from Steam, and games that already have a pack.
S = []
S += [
P("ASKA", ["1898300"], ["aska.exe"], "nature", "leaf green and earth brown.", ctl=["dualshock"]),
P("Cursed to Golf", ["1726120"], ["cursed to golf.exe"], "fighter", "arena red and gold."),
P("Firegirl: Hack 'n Splash Rescue", ["1608550"], ["fireg.exe"], "coldsteel", "steel blue and grey."),
P("Godbreakers", ["2687400"], ["godbreakers.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
PS("Laika: Aged through Blood", ["1796220"], ["laika aged through blood.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
P("Lego Bricktales", ["1898290"], ["lego_bricktales.exe"], "fighter", "arena red and gold."),
P("Lost in Random: The Eternal Die", ["2564520"], ["theeternaldie.exe"], "mystic", "violet and gold.", ctl=["dualshock"]),
P("Planet of Lana", ["1608230"], ["planet of lana.exe"], "calm", "soft pastel tones."),
P("Planet of Lana II", ["2997230"], ["planet of lana 2.exe"], "calm", "soft pastel tones.", ctl=["dualshock"]),
P("Reignbreaker", ["2129810"], ["project kokidon.exe", "reignbreaker.exe"], "mystic", "violet and gold.", ctl=["dualshock"]),
P("Replaced", ["1663850"], ["replaced.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
C("Say No! More", ["1191900"], ["snm.exe"], "calm", "soft pastel tones", exenote=" The Windows executable is from Steam's launch data."),
P("Source of Madness", ["1315610"], ["source of madness.exe"], "coldsteel", "steel blue and grey."),
P("SteamWorld Build", ["2134770"], ["steamworld build.exe"], "coldsteel", "steel blue and grey."),
P("SteamWorld Heist II", ["2396240"], ["steamworld heist 2.exe"], "mystic", "violet and gold.", ctl=["dualshock"]),
P("SteamWorld Quest: Hand of Gilgamech", ["804010"], ["quest.exe"], "scifi", "cold blue and cyan."),
P("Swordship", ["1804270"], ["swordship.exe"], "mystic", "violet and gold."),
PS("The Gunk", ["1087760"], ["dust.exe"], "calm", "soft pastel tones."),
P("Togges", ["1550270"], ["togges.exe"], "calm", "soft pastel tones."),
P("Viewfinder", ["1382070"], ["viewfinder.exe", "viewfinder_demo.exe"], "calm", "soft pastel tones.", ctl=["dualshock"]),
P("Wavetale", ["1823930"], ["wavetale.exe"], "coldsteel", "steel blue and grey."),
P("White Shadows", ["1158890"], ["whiteshadows.exe"], "calm", "soft pastel tones."),
]
