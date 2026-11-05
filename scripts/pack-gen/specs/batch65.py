from helpers import *
# Games Assemble Entertainment publishes, not covered by an earlier pack.
# Selection: a live Steam game whose Steam publisher is Assemble Entertainment (wiki names it too), no pack by Steam id or name.
# P = passthrough, for games with controller support (PS = passthrough plus a gyro profile, for shooters); T = strategy with a pointer;
# C / CW = pointer (and WASD) pack; F = mouse-and-keyboard shooter with a gyro profile; KB = keyboard game (pad support before XInput or none).
# Controller support: Steam's flag (full or partial) when it has one, else the PCGamingWiki snapshot; "dualshock" / "switch_pro" are ticked only where the wiki says so.
# Steam ids are each game's base app; executables come from Steam's launch data (Discord's list where Steam has none) and are left out when the name is generic
# or another pack already uses it (that pack then matches by Steam id alone). Genre-based kind and theme were assigned by script and corrected by hand for the
# pointer and keyboard games.
# Left out everywhere: VR games, DLC, demos, tools and soundtracks, games with no release date or not out yet, games delisted from Steam, and games that already have a pack.
S = []
S += [
P("Asgard's Fall: Viking Survivors", ["2780710"], ["asgardsfall.exe"], "coldsteel", "steel blue and grey."),
P("Between Horizons", ["1921980"], ["between horizons.exe"], "scifi", "cold blue and cyan.", ctl=["dualshock"]),
P("Decktamer", ["2870340"], ["decktamer.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
C("Encodya", ["1137450"], ["encodya.exe"], "scifi", "cold blue and cyan", exenote=" The Windows executable is from Steam's launch data."),
T("Endzone 2", ["2144640"], ["endzone2.exe"], "industrial", "steel grey and orange", exenote=" The Windows executable is from Steam's launch data."),
P("Epic Car Factory", ["656510"], ["epiccarfactory.exe", "mod_uploader.exe"], "coldsteel", "steel blue and grey."),
P("Fall of Porcupine", ["1710540"], ["fallofporcupine.exe"], "coldsteel", "steel blue and grey."),
T("Fast Food Tycoon 2", ["599000"], ["fastfood2.exe"], "calm", "soft pastel tones", exenote=" The Windows executable is from Steam's launch data."),
P("Flesh Made Fear", ["3316350"], ["fleshmadefear.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Growth", ["2300300"], ["growth.exe"], "calm", "soft pastel tones.", ctl=["dualshock"]),
C("Hauma", ["1443470"], ["hauma.exe"], "noir", "noir grey and amber", exenote=" The Windows executable is from Steam's launch data."),
T("Highrisers", ["733820"], ["highrisers.exe"], "industrial", "steel grey and orange", exenote=" The Windows executable is from Steam's launch data."),
P("In Between", ["388420"], ["in between.exe"], "coldsteel", "steel blue and grey."),
P("Itorah", ["1132910"], ["itorah.exe"], "coldsteel", "steel blue and grey."),
P("Lacuna", ["1364100"], ["lacuna.exe"], "calm", "soft pastel tones."),
C("Leisure Suit Larry 5: Passionate Patti Does a Little Undercover Work", ["765860"], [], "calm", "soft pastel tones"),
C("Leisure Suit Larry 6: Shape Up or Slip Out!", ["765910"], [], "calm", "soft pastel tones"),
C("Leisure Suit Larry Goes Looking for Love (in Several Wrong Places)", ["765840"], [], "calm", "soft pastel tones"),
C("Leisure Suit Larry III: Passionate Patti in Pursuit of the Pulsating Pectorals", ["765850"], [], "calm", "soft pastel tones"),
C("Leisure Suit Larry in the Land of the Lounge Lizards", ["763970"], [], "calm", "soft pastel tones"),
C("Leisure Suit Larry: Love for Sail!", ["765890"], [], "calm", "soft pastel tones"),
CW("Leisure Suit Larry: Magna Cum Laude", ["765900"], [], "calm", "soft pastel tones"),
P("Leisure Suit Larry: Wet Dreams Don't Dry", ["765870"], [], "calm", "soft pastel tones."),
P("Leisure Suit Larry: Wet Dreams Dry Twice", ["1373430"], ["leisure suit larry - wet dreams dry twice.exe"], "calm", "soft pastel tones."),
P("Nova Nukers!", ["445000"], ["nova_nukers.exe"], "coldsteel", "steel blue and grey."),
PS("Orbital Bullet", ["1167680"], ["orbitalbullet.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
T("Pizza Connection 3", ["588160"], ["pizza connection 3.exe"], "calm", "soft pastel tones", exenote=" The Windows executable is from Steam's launch data."),
C("Pizza Connection 3 - Pizza Creator", ["851330"], [], "calm", "soft pastel tones"),
T("Pizza Tycoon", ["598990"], [], "calm", "soft pastel tones"),
KB("Pole Position 2012", ["210150"], ["poleposition.exe"], "fighter", "arena red and gold", exenote=" The Windows executable is from Steam's launch data."),
C("Roadwarden", ["1155970"], ["roadwarden-32.exe", "roadwarden.exe"], "mystic", "violet and gold", exenote=" The Windows executable is from Steam's launch data."),
T("S.W.I.N.E. HD Remaster", ["944010"], ["swinehd.exe"], "military", "olive drab and khaki", exenote=" The Windows executable is from Steam's launch data."),
P("Shakes on a Plane", ["964620"], ["shakesdemo.exe"], "coldsteel", "steel blue and grey."),
T("Sphere: Flying Cities", ["1273220"], ["sphere - flying cities.exe"], "nature", "leaf green and earth brown", exenote=" The Windows executable is from Steam's launch data."),
PS("Squad 51 vs. the Flying Saucers", ["1378800"], ["squad51.exe", "squad51demoforevents.exe", "squad51private.exe"], "coldsteel", "steel blue and grey."),
P("Sticky Business", ["2303350"], ["stickybusiness.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Super Catboy", ["1376910"], ["super_catboy.exe"], "coldsteel", "steel blue and grey."),
T("The Great Art Race", ["33580"], ["tgar.exe"], "calm", "soft pastel tones", exenote=" The Windows executable is from Steam's launch data."),
C("The Skin Stapler", ["4310610"], ["theskinstapler.exe"], "horror", "blood red and grey", exenote=" The Windows executable is from Steam's launch data."),
P("Three Minutes to Eight", ["2303320"], ["three minutes to eight.exe"], "coldsteel", "steel blue and grey."),
P("Ultimate ADOM: Caverns of Chaos", ["1266820"], ["ultimate adom - caverns of chaos.exe"], "pixel", "bright pixel-art colors.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
C("Urban Jungle", ["2744010"], ["urbanjungle.exe"], "calm", "soft pastel tones", exenote=" The Windows executable is from Steam's launch data."),
P("Verne: The Shape of Fantasy", ["1424600"], ["verne.exe"], "mystic", "violet and gold.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("XEL", ["1674640"], ["xel.exe"], "coldsteel", "steel blue and grey."),
]
