from helpers import *
# Games Curve Digital publishes, not covered by an earlier pack.
# Selection: a live Steam game whose Steam publisher is Curve Digital (wiki names it too), no pack by Steam id or name.
# P = passthrough, for games with controller support (PS = passthrough plus a gyro profile, for shooters); T = strategy with a pointer;
# C / CW = pointer (and WASD) pack; F = mouse-and-keyboard shooter with a gyro profile; KB = keyboard game (pad support before XInput or none).
# Controller support: Steam's flag (full or partial) when it has one, else the PCGamingWiki snapshot; "dualshock" / "switch_pro" are ticked only where the wiki says so.
# Steam ids are each game's base app; executables come from Steam's launch data (Discord's list where Steam has none) and are left out when the name is generic
# or another pack already uses it (that pack then matches by Steam id alone). Genre-based kind and theme were assigned by script and corrected by hand for the
# pointer and keyboard games.
# Left out everywhere: VR games, DLC, demos, tools and soundtracks, games with no release date or not out yet, games delisted from Steam, and games that already have a pack.
S = []
S += [
P("10 Second Ninja X", ["435790"], ["10snx.exe"], "coldsteel", "steel blue and grey."),
P("American Fugitive", ["934780"], ["americanfugitive.exe"], "fighter", "arena red and gold."),
P("Badlands Crew", ["2078230"], ["badlandscrew.exe"], "industrial", "steel grey and orange.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Bomber Crew", ["537800"], ["bombercrew.exe"], "industrial", "steel grey and orange.", ctl=["dualshock"]),
P("Dungeons of Hinterberg", ["1983260"], ["dungeons of hinterberg.exe"], "mystic", "violet and gold.", ctl=["dualshock"]),
PS("Embr", ["1062830"], ["fireproject.exe"], "coldsteel", "steel blue and grey."),
P("For the King II", ["1676840"], ["for the king ii.exe"], "mystic", "violet and gold."),
PS("From Space", ["1502190"], ["fromspace.exe"], "scifi", "cold blue and cyan."),
KB("Golf Gang", ["1151050"], ["golf gang.exe"], "nature", "leaf green and earth brown", exenote=" The Windows executable is from Steam's launch data."),
P("Hotshot Racing", ["609920"], ["hotshotracing.exe"], "fighter", "arena red and gold."),
P("Hue", ["383270"], ["hue.exe"], "calm", "soft pastel tones."),
P("Human: Fall Flat", ["477160"], ["human.exe"], "coldsteel", "steel blue and grey."),
P("I Am Fish", ["1472560"], ["iamfish.exe"], "coldsteel", "steel blue and grey."),
P("Iron Fisticle", ["306700"], ["ironfisticle.exe"], "coldsteel", "steel blue and grey."),
P("Jump Stars", ["486990"], ["jumpstars.exe"], "scifi", "cold blue and cyan."),
P("Just Die Already", ["979070"], ["jdgame.exe"], "coldsteel", "steel blue and grey."),
P("Manual Samuel", ["504130"], ["manual samuel.exe"], "coldsteel", "steel blue and grey."),
P("Narcos: Rise of the Cartels", ["914110"], ["narcos.exe"], "coldsteel", "steel blue and grey."),
P("Patch Quest", ["1347970"], ["patch quest.exe"], "coldsteel", "steel blue and grey."),
P("Peaky Blinders: Mastermind", ["1013310"], ["peakyblinders.exe"], "calm", "soft pastel tones."),
P("Porcunipine", ["352620"], ["porcunipine.exe"], "coldsteel", "steel blue and grey."),
P("Pumped BMX +", ["392070"], ["pumped_bmx_+.exe"], "coldsteel", "steel blue and grey."),
P("Pumped BMX Pro", ["966720"], ["pumpedbmxpro.exe"], "fighter", "arena red and gold.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Qora", ["304460"], ["qora.exe"], "coldsteel", "steel blue and grey.", n="The wiki lists controller support but is not sure the game reads XInput: if the pad does nothing in the game, set the controller type in its options or run it through Steam Input."),
P("Serial Cleaner", ["522210"], ["cleaner.exe"], "coldsteel", "steel blue and grey."),
P("Smoke and Sacrifice", ["695100"], ["shade.exe"], "mystic", "violet and gold."),
P("Space Crew", ["1176710"], ["spacecrew.exe"], "scifi", "cold blue and cyan.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
C("Stranded", ["295250"], ["stranded.exe"], "nature", "leaf green and earth brown", exenote=" The Windows executable is from Steam's launch data."),
P("Table Manners", ["1019450"], ["tablemanners.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("The Indie Mixtape", ["352640"], [], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Wax Heads", ["2769240"], ["wax_heads.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
T("When Ski Lifts Go Wrong", ["638000"], ["when ski lifts go wrong.exe"], "calm", "soft pastel tones", exenote=" The Windows executable is from Steam's launch data."),
P("You Suck at Parking", ["837880"], ["ysap.exe"], "fighter", "arena red and gold."),
]
