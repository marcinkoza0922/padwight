from helpers import *
# Games Humble Games (listed as Balor Games on Steam) publishes, not covered by an earlier pack.
# Selection: a live Steam game whose Steam publisher is that label (wiki names it too), no pack by Steam id or name.
# P = passthrough, for games with controller support (PS = passthrough plus a gyro profile, for shooters); T = strategy with a pointer;
# C / CW = pointer (and WASD) pack; F = mouse-and-keyboard shooter with a gyro profile; KB = keyboard game (pad support before XInput or none).
# Controller support: Steam's flag (full or partial) when it has one, else the PCGamingWiki snapshot; "dualshock" / "switch_pro" are ticked only where the wiki says so.
# Steam ids are each game's base app; executables come from Steam's launch data (Discord's list where Steam has none) and are left out when the name is generic
# or another pack already uses it (that pack then matches by Steam id alone). Genre-based kind and theme were assigned by script and corrected by hand for the
# pointer and keyboard games.
# Left out everywhere: VR games, DLC, demos, tools and soundtracks, games with no release date or not out yet, games delisted from Steam, and games that already have a pack.
S = []
S += [
P("Archvale", ["1296360"], ["archvale.exe"], "mystic", "violet and gold."),
P("B\u014d: Path of the Teal Lotus", ["1614440"], ["bo.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"], slug="bo-path-of-the-teal-lotus"),
P("Carto", ["1172450"], ["carto.exe"], "calm", "soft pastel tones."),
C("Chinatown Detective Agency", ["1172190"], ["chinatown detective agency.exe"], "noir", "noir grey and amber", exenote=" The Windows executable is from Steam's launch data."),
P("Coral Island", ["1158160"], ["projectcoral.exe"], "nature", "leaf green and earth brown."),
P("Dodgeball Academia", ["1422420"], ["dodgeballacademia.exe"], "mystic", "violet and gold."),
C("Floor 13: Deep State", ["1263990"], ["floor 13 deep state.exe"], "noir", "noir grey and amber", exenote=" The Windows executable is from Steam's launch data."),
P("Flynn: Son of Crimson", ["737520"], ["flynn son of crimson.exe"], "mystic", "violet and gold."),
P("Ghost Song", ["347800"], ["ghostsong.exe"], "coldsteel", "steel blue and grey."),
P("Ikenfell", ["854940"], ["ikenfellwin.exe"], "mystic", "violet and gold."),
P("Infinite Guitars", ["1309080"], ["infinite guitars.exe"], "mystic", "violet and gold."),
PS("Into the Pit", ["1422410"], ["into the pit.exe"], "coldsteel", "steel blue and grey."),
P("Lost Skies", ["1931180"], ["lostskies.exe", "wildskies.exe"], "nature", "leaf green and earth brown."),
P("Midnight Fight Express", ["1390410"], ["midnightfightexpress.exe"], "coldsteel", "steel blue and grey."),
P("Monaco 2", ["1063030"], ["monaco2.exe"], "coldsteel", "steel blue and grey."),
P("Moonscars", ["1374970"], ["moonscars.exe"], "fighter", "arena red and gold."),
P("Next Space Rebels", ["1422430"], ["nextspacerebels.exe"], "military", "olive drab and khaki."),
P("On Your Tail", ["2132560"], ["on your tail.exe"], "coldsteel", "steel blue and grey."),
P("One Step from Eden", ["960690"], ["osfe.exe"], "coldsteel", "steel blue and grey."),
CW("Popup Dungeon", ["349730"], ["popup dungeon.exe"], "mystic", "violet and gold", exenote=" The Windows executable is from Steam's launch data."),
PS("Prodeus", ["964800"], ["prodeus.exe"], "coldsteel", "steel blue and grey."),
P("Project Wingman", ["895870"], ["projectwingman.exe"], "industrial", "steel grey and orange."),
P("Ring of Pain", ["998740"], ["ring of pain.exe"], "calm", "soft pastel tones."),
P("Rolling Hills: Make Sushi, Make Friends", ["1235100"], ["rolling hills.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
P("Signalis", ["1262350"], ["signalis.exe"], "horror", "blood red and grey.", ctl=["dualshock"]),
P("Staxel", ["405710"], ["staxel.modmanager.exe", "staxelce.exe"], "coldsteel", "steel blue and grey."),
P("Stray Gods: The Roleplaying Musical", ["1920780"], ["straygods.exe"], "coldsteel", "steel blue and grey."),
T("The Iron Oath", ["699330"], ["the iron oath.exe"], "mystic", "violet and gold", exenote=" The Windows executable is from Steam's launch data."),
P("The Wild at Heart", ["1093290"], ["the wild at heart.exe"], "calm", "soft pastel tones."),
P("Unsighted", ["1062110"], ["unsighted beta.exe", "unsighted.exe"], "coldsteel", "steel blue and grey."),
P("While the Iron's Hot", ["1906830"], ["while the iron's hot.exe"], "coldsteel", "steel blue and grey."),
P("Wildfire", ["431940"], ["wildfire.exe"], "calm", "soft pastel tones."),
]
