from helpers import *
# Games Valve and Epic Games (with Psyonix and Mediatonic) publish, not covered by an earlier pack.
# Selection: a live Steam game whose Steam publisher is Valve, Epic Games, Psyonix or Mediatonic. Half-Life, Counter-Strike, Opposing Force, Team Fortress Classic
# and Deathmatch Classic already have packs.
# P = passthrough, for games with controller support (PS = passthrough plus a gyro profile, for shooters); T = strategy with a pointer;
# C / CW = pointer (and WASD) pack; F = mouse-and-keyboard shooter with a gyro profile; KB = keyboard game (pad support before XInput or none).
# Controller support: Steam's flag (full or partial) when it has one, else the PCGamingWiki snapshot; "dualshock" / "switch_pro" are ticked only where the wiki says so.
# Steam ids are each game's base app; executables come from Steam's launch data (Discord's list where Steam has none) and are left out when the name is generic
# or another pack already uses it (that pack then matches by Steam id alone). Genre-based kind and theme were assigned by script and corrected by hand for the
# pointer and keyboard games.
# Left out everywhere: VR games, DLC, demos, tools and soundtracks, games with no release date or not out yet, games delisted from Steam, and games that already have a pack.
S = []
S += [
P("Aperture Desk Job", ["1902490"], ["deskjob.exe"], "scifi", "cold blue and cyan."),
F("Codename: Gordon", ["92"], ["cg.exe"], "scifi", "cold blue and cyan", exenote=" The Windows executable is from Steam's launch data."),
F("Day of Defeat", ["30"], [], "coldsteel", "steel blue and grey"),
T("Dota 2", ["570"], ["dota2.exe", "dota2cfg.exe"], "scifi", "cold blue and cyan", exenote=" The Windows executable is from Steam's launch data."),
P("Foul Play", ["244810"], ["foul_play.exe"], "fighter", "arena red and gold."),
F("Half-Life 2: Deathmatch", ["320"], ["hl2mp.exe", "hl2mp_win64.exe"], "scifi", "cold blue and cyan", n="The game has some controller support, but it predates XInput or is limited, so this pack plays it with the keyboard keys.", exenote=" The Windows executable is from Steam's launch data."),
F("Half-Life: Blue Shift", ["130"], [], "scifi", "cold blue and cyan"),
F("Half-Life: Source", ["280"], [], "scifi", "cold blue and cyan"),
C("Hatoful Boyfriend: Holiday Star", ["377080"], ["hb2.exe"], "calm", "soft pastel tones", exenote=" The Windows executable is from Steam's launch data."),
T("Moondust", ["887260"], [], "scifi", "cold blue and cyan"),
P("Pampas & Selene: The Maze of Demons", ["1966220"], ["maze.exe"], "coldsteel", "steel blue and grey."),
P("Portal 2", ["620"], ["portal2.exe"], "scifi", "cold blue and cyan."),
F("Ricochet", ["60"], [], "scifi", "cold blue and cyan", n="The game has some controller support, but it predates XInput or is limited, so this pack plays it with the keyboard keys."),
P("Rocket League", ["252950"], ["rocketleague.exe", "rocketleague_eac.exe"], "fighter", "arena red and gold.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard.", ctl=["dualshock"]),
PS("Shadow Complex Remastered", ["385560"], ["shadowcomplex-win32.exe"], "coldsteel", "steel blue and grey."),
F("Unreal", ["13250"], [], "scifi", "cold blue and cyan"),
F("Unreal II: The Awakening", ["13200"], ["unreal2.exe"], "scifi", "cold blue and cyan", exenote=" The Windows executable is from Steam's launch data."),
F("Unreal Tournament", ["13240"], ["unrealtournament.exe"], "scifi", "cold blue and cyan", exenote=" The Windows executable is from Steam's launch data."),
F("Unreal Tournament 2004", ["13230"], ["ut2004.exe"], "scifi", "cold blue and cyan", exenote=" The Windows executable is from Steam's launch data."),
PS("Unreal Tournament 3", ["13210"], ["eosbootstrapper.exe", "ut3.exe"], "scifi", "cold blue and cyan."),
]
