from helpers import *
# Games MicroProse publishes, not covered by an earlier pack.
# Selection: a live Steam game whose Steam publisher is MicroProse (wiki names it too), no pack by Steam id or name.
# P = passthrough, for games with controller support (PS = passthrough plus a gyro profile, for shooters); T = strategy with a pointer;
# C / CW = pointer (and WASD) pack; F = mouse-and-keyboard shooter with a gyro profile; KB = keyboard game (pad support before XInput or none).
# Controller support: Steam's flag (full or partial) when it has one, else the PCGamingWiki snapshot; "dualshock" / "switch_pro" are ticked only where the wiki says so.
# Steam ids are each game's base app; executables come from Steam's launch data (Discord's list where Steam has none) and are left out when the name is generic
# or another pack already uses it (that pack then matches by Steam id alone). Genre-based kind and theme were assigned by script and corrected by hand for the
# pointer and keyboard games.
# Left out everywhere: VR games, DLC, demos, tools and soundtracks, games with no release date or not out yet, games delisted from Steam, and games that already have a pack.
S = []
S += [
KB("Apache Longbow", ["3317800"], [], "military", "olive drab and khaki", n="The game has some controller support, but it predates XInput or is limited, so this pack plays it with the keyboard keys."),
KB("B-17 Flying Fortress World War II Bombers in Action", ["1634450"], [], "military", "olive drab and khaki", n="The game has some controller support, but it predates XInput or is limited, so this pack plays it with the keyboard keys."),
KB("B-17 Flying Fortress: The Mighty 8th", ["328900"], [], "military", "olive drab and khaki"),
P("B-17 Flying Fortress: The Mighty 8th Redux", ["2000800"], ["gameredux.exe"], "industrial", "steel grey and orange."),
T("Battlefall: State of Conflict", ["1031730"], ["3d fps.exe", "battlefall_state_of_conflict.exe", "battlefall_state_of_conflict_developer.exe"], "military", "olive drab and khaki", exenote=" The Windows executable is from Steam's launch data."),
CW("Better Than Dead", ["2599690"], ["btd.exe"], "noir", "noir grey and amber", exenote=" The Windows executable is from Steam's launch data."),
T("Car Park Capital", ["3031880"], ["carparkcapital.exe"], "calm", "soft pastel tones", exenote=" The Windows executable is from Steam's launch data."),
CW("Carrier Command", ["1802690"], [], "military", "olive drab and khaki"),
P("Carrier Command 2", ["1489630"], ["carrier_command.exe", "carrier_command_vr.exe"], "coldsteel", "steel blue and grey."),
P("Cleared Hot", ["1710820"], ["clearedhot.exe"], "industrial", "steel grey and orange.", ctl=["dualshock"]),
T("Command HQ", ["329630"], [], "military", "olive drab and khaki"),
PS("Dagger Directive", ["2437670"], ["dagger directive.exe"], "coldsteel", "steel blue and grey."),
PS("Darkenstein 3D", ["2195910"], ["darkenstein3d.exe"], "coldsteel", "steel blue and grey."),
P("Death by Scrolling", ["3773590"], ["death by scrolling.exe"], "mystic", "violet and gold."),
P("Drop: System Breach", ["1902300"], ["drop.exe"], "calm", "soft pastel tones.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
F("Dying Breed", ["2088550"], ["dyingbreed.exe"], "horror", "blood red and grey", exenote=" The Windows executable is from Steam's launch data."),
KB("European Air War", ["514130"], ["eaw.exe"], "military", "olive drab and khaki", exenote=" The Windows executable is from Steam's launch data."),
T("Exploration", ["3020300"], [], "parchment", "parchment brown and gold"),
KB("F-117A Nighthawk Stealth Fighter 2.0", ["328920"], [], "military", "olive drab and khaki", n="The game has some controller support, but it predates XInput or is limited, so this pack plays it with the keyboard keys."),
KB("F-22: Air Dominance Fighter", ["3146140"], ["adf.exe"], "military", "olive drab and khaki", exenote=" The Windows executable is from Steam's launch data."),
KB("Falcon", ["374080"], [], "military", "olive drab and khaki"),
KB("Falcon 3.0", ["429520"], [], "military", "olive drab and khaki"),
KB("Falcon 4.0", ["429530"], ["falcon4.exe"], "military", "olive drab and khaki", exenote=" The Windows executable is from Steam's launch data."),
KB("Falcon A.T.", ["286790"], [], "military", "olive drab and khaki"),
P("Forgotten but Unbroken", ["2122090"], ["forgotten but unbroken.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Grand Prix 3", ["3603730"], [], "fighter", "arena red and gold.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Grand Prix 4", ["3603740"], [], "fighter", "arena red and gold.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
KB("Gunship", ["2766090"], [], "military", "olive drab and khaki"),
KB("Gunship 2000", ["2766090"], [], "military", "olive drab and khaki"),
T("Harpoon Classic '97", ["3086120"], ["winharp32.exe"], "military", "olive drab and khaki", exenote=" The Windows executable is from Steam's launch data."),
CW("HighFleet", ["1434950"], ["highfleet.exe"], "military", "olive drab and khaki", exenote=" The Windows executable is from Steam's launch data."),
KB("Hind", ["2577400"], [], "military", "olive drab and khaki"),
P("IF-16", ["3020280"], [], "industrial", "steel grey and orange.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
KB("IM1A2 Abrams", ["2577380"], ["abrams.exe"], "military", "olive drab and khaki", exenote=" The Windows executable is from Steam's launch data."),
T("Mech Engineer", ["1428520"], ["mech_engineer.exe"], "scifi", "cold blue and cyan", exenote=" The Windows executable is from Steam's launch data."),
C("Neyyah", ["1289720"], ["neyyah.exe"], "mystic", "violet and gold", exenote=" The Windows executable is from Steam's launch data."),
T("Open Wheel Manager", ["1013960"], ["openwheelmanager.exe"], "fighter", "arena red and gold", exenote=" The Windows executable is from Steam's launch data."),
T("Open Wheel Manager 2", ["1757580"], ["open wheel manager 2.exe"], "fighter", "arena red and gold", exenote=" The Windows executable is from Steam's launch data."),
T("Regiments", ["1109680"], ["regiments.exe"], "military", "olive drab and khaki", exenote=" The Windows executable is from Steam's launch data."),
T("Second Front", ["1148490"], ["second front.exe"], "military", "olive drab and khaki", exenote=" The Windows executable is from Steam's launch data."),
KB("Silent Depth 2", ["2591010"], ["silent depth 2.exe"], "military", "olive drab and khaki", exenote=" The Windows executable is from Steam's launch data."),
P("Spearhead", ["3020350"], [], "industrial", "steel grey and orange.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
T("Strategos", ["3064810"], ["strategos.exe"], "parchment", "parchment brown and gold", exenote=" The Windows executable is from Steam's launch data."),
C("Sword of the Samurai", ["327950"], [], "military", "olive drab and khaki"),
T("Task Force 1942: Surface Naval Action in the South Pacific", ["329680"], [], "military", "olive drab and khaki"),
T("TFC2: Collapse of The Bronze Age", ["4158610"], ["tfc2.exe"], "parchment", "parchment brown and gold", exenote=" The Windows executable is from Steam's launch data."),
T("TFC: The Fertile Crescent", ["1674820"], ["the fertile crescent.exe"], "parchment", "parchment brown and gold", exenote=" The Windows executable is from Steam's launch data."),
KB("The History Channel: Battle of Britain - World War II 1940", ["3020340"], ["wb3.exe"], "military", "olive drab and khaki", exenote=" The Windows executable is from Steam's launch data."),
P("Tiny Combat Arena", ["1347550"], ["arena.exe"], "industrial", "steel grey and orange.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Tiny Football", ["1887010"], ["tinyfootball.exe"], "fighter", "arena red and gold."),
T("Urban Strife", ["710230"], ["usg.exe"], "military", "olive drab and khaki", exenote=" The Windows executable is from Steam's launch data."),
T("Waronoi", ["1444550"], ["waronoi.exe"], "military", "olive drab and khaki", exenote=" The Windows executable is from Steam's launch data."),
]
