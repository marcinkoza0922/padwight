from helpers import *
# Games Bohemia Interactive and Hooded Horse publish, not covered by an earlier pack.
# Selection: a live Steam game whose Steam publisher is a Bohemia Interactive or Hooded Horse label.
# P = passthrough, for games with controller support (PS = passthrough plus a gyro profile, for shooters); T = strategy with a pointer;
# C / CW = pointer (and WASD) pack; F = mouse-and-keyboard shooter with a gyro profile; KB = keyboard game (pad support before XInput or none).
# Controller support: Steam's flag (full or partial) when it has one, else the PCGamingWiki snapshot; "dualshock" / "switch_pro" are ticked only where the wiki says so.
# Steam ids are each game's base app; executables come from Steam's launch data (Discord's list where Steam has none) and are left out when the name is generic
# or another pack already uses it (that pack then matches by Steam id alone). Genre-based kind and theme were assigned by script and corrected by hand for the
# pointer and keyboard games.
# Left out everywhere: VR games, DLC, demos, tools and soundtracks, games with no release date or not out yet, games delisted from Steam, and games that already have a pack.
S = []
S += [
P("9 Kings", ["2784470"], ["9kings.exe"], "calm", "soft pastel tones.", ctl=["dualshock"]),
F("Alpha Prime", ["2590"], ["alphaprime.exe"], "scifi", "cold blue and cyan", exenote=" The Windows executable is from Steam's launch data."),
C("Alternativa", ["33990"], ["alternativa.exe"], "calm", "soft pastel tones", exenote=" The Windows executable is from Steam's launch data."),
PS("Argo", ["530700"], ["argobattleye.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
PS("Arma Reforger", ["1874880"], ["armareforger_be.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
P("Arma Tactics", ["224860"], ["armatactics.exe"], "coldsteel", "steel blue and grey."),
PS("Arma: Armed Assault", ["2780"], ["arma.exe"], "coldsteel", "steel blue and grey.", n="The wiki lists controller support but is not sure the game reads XInput: if the pad does nothing in the game, set the controller type in its options or run it through Steam Input."),
PS("Arma: Cold War Assault Remastered", ["65790"], [], "coldsteel", "steel blue and grey.", ctl=["dualshock", "switch_pro"]),
PS("Arma: Cold War Assault", ["65790"], [], "coldsteel", "steel blue and grey."),
T("Beyond These Stars", ["2295060"], ["beyond these stars.exe"], "scifi", "cold blue and cyan", exenote=" The Windows executable is from Steam's launch data."),
T("Blacksmith Master", ["2292800"], ["blacksmith master.exe"], "parchment", "parchment brown and gold", exenote=" The Windows executable is from Steam's launch data."),
P("Carrier Command: Gaea Mission", ["65740"], ["carrier.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
T("Cataclismo", ["1422440"], ["castle.exe", "cataclismo.exe"], "parchment", "parchment brown and gold", exenote=" The Windows executable is from Steam's launch data."),
T("Clanfolk", ["1700870"], ["clanfolk.exe"], "nature", "leaf green and earth brown", exenote=" The Windows executable is from Steam's launch data."),
T("Corsair Cove", ["1368140"], ["corsaircove.exe"], "parchment", "parchment brown and gold", exenote=" The Windows executable is from Steam's launch data."),
P("Empires of the Undergrowth", ["463530"], ["eotu.exe"], "coldsteel", "steel blue and grey."),
P("Endless Legend 2", ["3407390"], ["endless legend 2.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
P("Everwind", ["2253100"], ["everwind.exe"], "mystic", "violet and gold."),
C("Fairy Tale About Father Frost, Ivan and Nastya", ["289320"], ["mrazik.exe"], "calm", "soft pastel tones", exenote=" The Windows executable is from Steam's launch data."),
C("Fish Fillets 2", ["33950"], ["fillets2.exe"], "calm", "soft pastel tones", exenote=" The Windows executable is from Steam's launch data."),
P("Fogpiercer", ["3219010"], ["fogpiercer-full.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
P("He is Coming", ["2824490"], ["he is coming.exe"], "coldsteel", "steel blue and grey."),
P("Heart of the Machine", ["2001070"], ["heartofthemachine.exe"], "coldsteel", "steel blue and grey."),
T("Heroes of Might and Magic: Olden Era", ["3105440"], ["heroesoldenera.exe"], "mystic", "violet and gold", exenote=" The Windows executable is from Steam's launch data."),
C("Memento Mori 2", ["237970"], ["memento2.exe"], "scifi", "cold blue and cyan", exenote=" The Windows executable is from Steam's launch data."),
C("Memento Mori", ["200490"], [], "scifi", "cold blue and cyan"),
T("Menace (2026)", ["2432860"], ["menace.exe"], "scifi", "cold blue and cyan", exenote=" The Windows executable is from Steam's launch data."),
T("Nebulous: Fleet Command", ["887570"], ["nebulous.exe"], "scifi", "cold blue and cyan", exenote=" The Windows executable is from Steam's launch data."),
T("Nova Roma", ["2426530"], ["novaroma.exe"], "parchment", "parchment brown and gold", exenote=" The Windows executable is from Steam's launch data."),
T("Old World", ["597180"], ["oldworld.exe"], "parchment", "parchment brown and gold", exenote=" The Windows executable is from Steam's launch data."),
C("Pat & Mat", ["65710"], ["patamat.exe"], "calm", "soft pastel tones", exenote=" The Windows executable is from Steam's launch data."),
C("Pound of Ground", ["33980"], ["zl007.exe"], "calm", "soft pastel tones", exenote=" The Windows executable is from Steam's launch data."),
P("Shadow Gambit: The Cursed Crew", ["1545560"], ["shadowgambit_tcc.exe", "skartoffel_unity.exe"], "coldsteel", "steel blue and grey."),
CW("Silica", ["1494420"], ["silica.exe"], "scifi", "cold blue and cyan", exenote=" The Windows executable is from Steam's launch data."),
P("Someday You'll Return: Director's Cut", ["2229350"], ["theforest.exe"], "horror", "blood red and grey.", ctl=["dualshock"]),
P("Super Fantasy Kingdom", ["2289750"], ["super fantasy kingdom.exe"], "mystic", "violet and gold."),
P("Take On Helicopters", ["65730"], ["datacachepreprocessor.exe", "takeonh.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Take On Mars", ["244030"], ["tkom.exe", "tkom_dev.exe", "tkom_loader.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
T("UFO: Afterlight", ["237950"], ["ufo.exe"], "scifi", "cold blue and cyan", exenote=" The Windows executable is from Steam's launch data."),
T("Whiskerwood", ["2489330"], ["whiskerwood.exe"], "nature", "leaf green and earth brown", exenote=" The Windows executable is from Steam's launch data."),
T("Xenonauts 2", ["538030"], ["xenonauts2.exe"], "scifi", "cold blue and cyan", exenote=" The Windows executable is from Steam's launch data."),
P("Ylands", ["298610"], ["kosmo.exe", "ylands.exe"], "coldsteel", "steel blue and grey."),
]
