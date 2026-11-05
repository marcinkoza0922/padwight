from helpers import *
# Games Spike Chunsoft publishes, not covered by an earlier pack.
# Selection: a live Steam game whose Steam publisher is Spike Chunsoft (wiki names it too), no pack by Steam id or name.
# P = passthrough, for games with controller support (PS = passthrough plus a gyro profile, for shooters); T = strategy with a pointer;
# C / CW = pointer (and WASD) pack; F = mouse-and-keyboard shooter with a gyro profile; KB = keyboard game (pad support before XInput or none).
# Controller support: Steam's flag (full or partial) when it has one, else the PCGamingWiki snapshot; "dualshock" / "switch_pro" are ticked only where the wiki says so.
# Steam ids are each game's base app; executables come from Steam's launch data (Discord's list where Steam has none) and are left out when the name is generic
# or another pack already uses it (that pack then matches by Steam id alone). Genre-based kind and theme were assigned by script and corrected by hand for the
# pointer and keyboard games.
# Left out everywhere: VR games, DLC, demos, tools and soundtracks, games with no release date or not out yet, games delisted from Steam, and games that already have a pack.
S = []
S += [
P("428: Shibuya Scramble", ["648580"], ["428 shibuya scramble.exe", "kachina_debug.exe"], "coldsteel", "steel blue and grey."),
P("AI: The Somnium Files", ["948740"], [], "calm", "soft pastel tones.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Anonymous;Code", ["2291020"], ["ac_guidebook.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Bakeru", ["2969380"], ["bakeru.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
P("Chaos;Child", ["970570"], [], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Chaos;Head NoAH", ["1961950"], [], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Conception II: Children of the Seven Stars", ["458730"], ["conception2.exe"], "scifi", "cold blue and cyan.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Conception PLUS: Maidens of the Twelve Stars", ["906510"], ["conceptionplus.exe"], "scifi", "cold blue and cyan."),
PS("Danganronpa Another Episode: Ultra Despair Girls", ["555950"], [], "coldsteel", "steel blue and grey."),
P("Danganronpa S: Ultimate Summer Camp", ["1691970"], ["danganronpa s ultimate summer camp.exe"], "mystic", "violet and gold.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Danganronpa V3: Killing Harmony", ["567640"], ["dangan3win.exe", "dangan3windebug.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
P("Ever 17: The Out of Infinity (Remaster)", ["2392500"], ["gamee17_steam.exe"], "fighter", "arena red and gold."),
P("Fire Pro Wrestling World", ["564230"], ["fireprowrestlingw.exe"], "fighter", "arena red and gold.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
C("Kamaitachi no Yoru x3", ["2612660"], [], "horror", "blood red and grey"),
P("Katana Kami: A Way of the Samurai Story", ["1062340"], [], "samurai", "ink black and crimson.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Made in Abyss: Binary Star Falling into Darkness", ["1324340"], ["madeinabyss.exe"], "scifi", "cold blue and cyan.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Master Detective Archives: Rain Code Plus", ["2903950"], ["raincodeplus.exe", "the book of death.exe"], "noir", "noir grey and amber.", ctl=["dualshock"]),
P("Memories Off", ["675610"], [], "scifi", "cold blue and cyan.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Memories Off 2nd", ["679520"], [], "scifi", "cold blue and cyan.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard.", ctl=["dualshock"]),
P("Memories Off Number 5 Togireta Film", ["2184570"], [], "scifi", "cold blue and cyan.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard.", ctl=["dualshock"]),
P("Memories Off Sorekara", ["960730"], [], "scifi", "cold blue and cyan.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard.", ctl=["dualshock"]),
P("Memories Off Yubikiri No Kioku", ["2184590"], [], "scifi", "cold blue and cyan.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard.", ctl=["dualshock"]),
P("Memories Off6 T-wave", ["2184580"], [], "scifi", "cold blue and cyan.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard.", ctl=["dualshock"]),
P("Memories Off: Innocent Fille", ["738510"], [], "scifi", "cold blue and cyan.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Memories Off: Innocent Fille for Dearest", ["1200110"], [], "scifi", "cold blue and cyan.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Mystery Chronicle: One Way Heroics", ["409000"], ["mc_onewayheroics.exe"], "coldsteel", "steel blue and grey."),
P("Natsu-Mon: 20th Century Summer Kid", ["2839280"], ["natsumon.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
P("Never7: The End of Infinity (Remaster)", ["2184620"], ["gamen7_steam.exe"], "fighter", "arena red and gold."),
P("NirvanA Initiative", ["1449200"], [], "calm", "soft pastel tones."),
P("No Sleep For Kaname Date", ["2752180"], [], "calm", "soft pastel tones.", ctl=["dualshock"]),
P("Omoide Ni Kawaru Kimi Memories Off", ["743910"], [], "scifi", "cold blue and cyan.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard.", ctl=["dualshock"]),
P("PixelJunk Monsters 2", ["749800"], ["pixeljunkmonsters2.exe"], "pixel", "bright pixel-art colors."),
P("Re:ZERO -Starting Life in Another World- The Prophecy of the Throne", ["1277510"], ["rezero-tpt.exe"], "scifi", "cold blue and cyan."),
P("Research and Destroy", ["1279310"], ["tbs.exe"], "coldsteel", "steel blue and grey."),
P("Robotics;Notes DaSH", ["1111390"], [], "scifi", "cold blue and cyan.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Robotics;Notes Elite", ["1111380"], [], "scifi", "cold blue and cyan.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Shiren the Wanderer: The Mystery Dungeon of Serpentcoil Island", ["2178480"], ["shirenthewanderer6.exe"], "mystic", "violet and gold.", ctl=["dualshock"]),
P("Shiren the Wanderer: The Tower of Fortune and the Dice of Fate", ["1178790"], [], "mystic", "violet and gold.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard.", ctl=["dualshock"]),
P("Shuten Order", ["3216500"], ["shuten.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard.", ctl=["dualshock"]),
P("Slitterhead", ["2631250"], ["yks.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
P("Steins;Gate 0", ["825630"], [], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Steins;Gate Elite", ["819030"], [], "coldsteel", "steel blue and grey."),
P("Steins;Gate Re:Boot", ["4012810"], ["sg_octet.exe", "sgre_steam.exe", "sgre_steam_d.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Steins;Gate: Linear Bounded Phenogram", ["930910"], [], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Steins;Gate: My Darling's Embrace", ["970560"], [], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Way of the Samurai 3", ["427510"], ["wayofthesamurai3.exe"], "samurai", "ink black and crimson.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Way of the Samurai 4", ["312780"], ["wayofthesamurai4.exe"], "samurai", "ink black and crimson.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("YU-NO: A Girl Who Chants Love at the Bound of this World (2017)", ["912450"], [], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Zanki Zero: Last Beginning", ["819020"], ["zankizero.exe", "zankizerosetting.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Zero Escape: The Nonary Games", ["477740"], [], "calm", "soft pastel tones.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Zero Escape: Zero Time Dilemma", ["311240"], ["zero escape.exe"], "calm", "soft pastel tones."),
P("\u4f8d\u9053\u5916\u4f1d KATANAKAMI", ["1062340"], [], "samurai", "ink black and crimson.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard.", slug="katanakami"),
]
