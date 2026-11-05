from helpers import *
# Games City Connection publishes, not covered by an earlier pack.
# Selection: a live Steam game whose Steam publisher is City Connection (wiki names it too), no pack by Steam id or name.
# P = passthrough, for games with controller support (PS = passthrough plus a gyro profile, for shooters); T = strategy with a pointer;
# C / CW = pointer (and WASD) pack; F = mouse-and-keyboard shooter with a gyro profile; KB = keyboard game (pad support before XInput or none).
# Controller support: Steam's flag (full or partial) when it has one, else the PCGamingWiki snapshot; "dualshock" / "switch_pro" are ticked only where the wiki says so.
# Steam ids are each game's base app; executables come from Steam's launch data (Discord's list where Steam has none) and are left out when the name is generic
# or another pack already uses it (that pack then matches by Steam id alone). Genre-based kind and theme were assigned by script and corrected by hand for the
# pointer and keyboard games.
# Left out everywhere: VR games, DLC, demos, tools and soundtracks, games with no release date or not out yet, games delisted from Steam, and games that already have a pack.
S = []
S += [
PS("Abarenbo Tengu & Zombie Nation", ["1603920"], ["zn.exe"], "horror", "blood red and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
PS("Akai Katana Shin", ["2076220"], ["akai_katana_shin.exe"], "samurai", "ink black and crimson.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
KB("Cleopatra Fortune S-Tribute", ["1913690"], [], "pixel", "bright pixel-art colors"),
PS("Cotton 2: Magical Night Dreams", ["1708800"], ["cotton2.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
PS("Cotton Boomerang", ["1708810"], ["cottonboomerang.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Culdcept The First", ["3825600"], [], "coldsteel", "steel blue and grey."),
PS("Deathsmiles I\u30fbII", ["1884300"], ["deathsmilesi-ii.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard.", slug="deathsmiles-iii"),
PS("Dragon Blaze", ["1279420"], ["dragonblaze.exe"], "mystic", "violet and gold."),
KB("Elevator Action -Returns- S-Tribute", ["1913680"], [], "pixel", "bright pixel-art colors"),
PS("Guardian Force", ["1708820"], ["guardianforce.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Gunbarich", ["1342240"], ["gunbarich.exe"], "fighter", "arena red and gold."),
PS("Gunbird", ["1261970"], ["gunbird.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
PS("Gunbird 2", ["1279410"], ["gunbird2.exe"], "coldsteel", "steel blue and grey."),
P("Hebereke: Enjoy Edition", ["2869000"], ["fchebereke.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
KB("Layer Section & Galactic Attack S-Tribute", ["1805240"], [], "pixel", "bright pixel-art colors"),
PS("Mamorukun ReCurse!", ["3273980"], ["noroixp.exe"], "coldsteel", "steel blue and grey."),
PS("Metal Black S-Tribute", ["1913670"], [], "coldsteel", "steel blue and grey."),
PS("Muchi Muchi Pork! & PinkSweets Boosted", ["4229200"], ["muchi muchi pork! & pink sweets boosted.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
P("Ninja JaJaMaru: The Great Yokai Battle + Hell", ["1195070"], ["jaja.exe"], "coldsteel", "steel blue and grey."),
PS("Psyvariar Delta", ["998990"], ["psyvariardelta.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Puzzle Bobble 2X / Bust-A-Move 2 Arcade Edition & Puzzle Bobble 3 / Bust-A-Move 3 S-Tribute", ["1913620"], [], "calm", "soft pastel tones."),
PS("Samurai Aces", ["1261980"], ["saces.exe"], "samurai", "ink black and crimson.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
PS("Samurai Aces III: Sengoku Cannon", ["1279390"], ["scannon.exe"], "samurai", "ink black and crimson."),
PS("Sol Divide: Sword of Darkness", ["1279450"], ["soldivide.exe"], "mystic", "violet and gold."),
PS("Strikers 1945", ["1261960"], ["s1945.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
PS("Strikers 1945 II", ["1279380"], ["s1945ii.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
PS("Strikers 1945 III", ["1279400"], ["s1945iii.exe"], "coldsteel", "steel blue and grey."),
PS("Tengai", ["1279370"], ["sblade.exe"], "coldsteel", "steel blue and grey."),
PS("Under Defeat", ["2869510"], ["udx.exe"], "coldsteel", "steel blue and grey."),
P("WiZmans World Re;Try", ["2214620"], ["wizmans world retry.exe"], "mystic", "violet and gold."),
PS("Zero Gunner 2-", ["1342250"], ["zero gunner 2-.exe"], "coldsteel", "steel blue and grey."),
]
