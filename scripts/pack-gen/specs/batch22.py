from helpers import *
# Games Take-Two published through its labels (2K, Rockstar, and Gearbox now that Take-Two owns it), not covered by an earlier pack.
# Selection: the wiki's publisher or developer field names a Take-Two label, the game has a live Steam page of type game, and Steam's publisher
# field names 2K, 2K Games, 2K Sports, Rockstar Games or Gearbox Publishing.
# P = passthrough, for games with controller support that came out after XInput (Dec 2005); PS = passthrough plus a gyro profile, for shooters.
# T = strategy with a pointer; C / CW = pointer (and WASD) pack; F = mouse-and-keyboard shooter with a gyro profile; KB = keyboard game.
# ctl= ticks "dualshock" only where the wiki's PlayStation column says true; the DualSense flag is not in Steam's app info, so it is left off.
# Left out: VR games (Borderlands 2 VR, Carnival Games VR, NBA 2KVR Experience, L.A. Noire: The VR Case Files, Desert Bus VR, Penn & Teller VR),
# games Take-Two sold off (Private Division's After Us, Disintegration, Penny's Big Breakaway, The Outer Worlds, No Rest for the Wicked, Ancestors,
# Tales of the Shire), cancelled or delisted games (Star Wars: Hunters, Solstice Arena, Wild Metal Country), the unreleased Judas, a bundle that is not
# a Steam game of its own (GTA: The Trilogy - The Definitive Edition), and games where Steam credits another publisher (Age of Wonders 1 and 2, Aliens:
# Colonial Marines, Duke Nukem 1, 2, 3D and Manhattan Project, Half-Life: Blue Shift, Serious Sam, Sid Meier's Alpha Centauri, MLB Front Office Manager,
# Top Spin 2, Torchlight III, Relic Hunters Legend, Eyes in the Dark, We Happy Few, Port Royale 2, Tzar, Siege of Avalon, Metal Fatigue, The Corporate
# Machine, The Great Art Race, Emergency 2, Fragmental, RADtv, Rainbow Six: Rogue Spear,) or names no publisher (NBA 2K9).
# Games that share an executable name with another pack (the Borderlands and Tiny Tina launchers, the Tales from the Borderlands exe) match by Steam id alone.
# Steam ids are each game's base app; demos, DLC and editions are not rules.
S = []
PART = "The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."
XI = "The wiki lists controller support but is not sure the game reads XInput: if the pad does nothing in the game, set the controller type in its options or run it through Steam Input."
SOON = "Not out yet: Steam gives no release date and the wiki lists controller support, so this pack assumes the usual controller support of the studio's games and is ready before the game is."
FOLDER = " The executable rule names Steam's install folder in front of the file name (installdir from Steam's app info), because other games share the file name; it is not checked on an install."
GUESS = "Typical defaults for the genre, not checked against the game."
def T(name, ids, exes, theme, mood, n="", **kw):
    return G(name, ids, exes, "strat", theme, f"The left stick pans, the right stick points; RT and A click, LT right-clicks. Overlays in {mood}.", n=(n + " " if n else "") + GUESS, **kw)
def C(name, ids, exes, theme, mood, n="", **kw):
    return G(name, ids, exes, "click", theme, f"The right stick points; A and RT click, LT right-clicks. Overlays in {mood}.", n=n or "A mouse-driven game; no keys are mapped beyond the mouse.", **kw)
def CW(name, ids, exes, theme, mood, n="", **kw):
    return G(name, ids, exes, "click", theme, f"The left stick walks on WASD and the right stick points or looks; RT and A click, LT right-clicks. Overlays in {mood}.", left="wasd", n=(n + " " if n else "") + GUESS, **kw)
CODN = PART.replace("The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard.", "")
DS = ["dualshock", "dualsense"]

GOLF = "Steam's executable for this game is golf.exe, the same name PGA Tour 2K21 uses, so this pack matches by Steam id alone."
SPORT = "A sports game: its menus and the pitch, court or ring all use the pad."
S += [
# Shooters
PS("BioShock", ["7670"], ["bioshock.exe"], "noir", "Rapture art-deco teal and gold.", n=PART),
PS("BioShock Remastered", ["409710"], ["bioshockhd.exe"], "noir", "Rapture art-deco teal and gold."),
PS("BioShock 2", ["8850"], ["bioshock2.exe"], "noir", "Rapture art-deco teal and gold."),
PS("BioShock 2 Remastered", ["409720"], ["bioshock2hd.exe"], "noir", "Rapture art-deco teal and gold."),
PS("BioShock Infinite", ["8870"], ["bioshockinfinite.exe"], "azure", "Columbia sky blue and gold."),
PS("Borderlands", ["8980"], ["borderlands.exe"], "industrial", "Pandora dust and orange.", n=PART),
PS("Borderlands 2", ["49520"], ["borderlands2.exe"], "industrial", "Pandora dust and orange."),
PS("Borderlands: The Pre-Sequel", ["261640"], ["borderlandspresequel.exe"], "scifi", "moon-base blue and orange."),
PS("Borderlands 3", ["397540"], ["borderlands3.exe"], "industrial", "Pandora dust and orange."),
PS("Borderlands 4", ["1285190"], ["borderlands4.exe"], "industrial", "Kairos dust and orange.", n=PART, ctl=["dualshock"]),
PS("Tiny Tina's Wonderlands", ["1286680"], ["wonderlands.exe"], "mystic", "tabletop-fantasy violet and gold.", n=PART),
PS("Tiny Tina's Assault on Dragon Keep: A Wonderlands One-Shot Adventure", ["1712840"], ["tinytina.exe"], "mystic", "tabletop-fantasy violet and gold.", slug="tiny-tinas-assault-on-dragon-keep"),
PS("Battleborn", ["394230"], ["battleborn.exe"], "scifi", "Solus blue and gold."),
PS("Bulletstorm: Full Clip Edition", ["501590"], ["stormgame-win64-shipping.exe"], "scifi", "Stygia neon pink and teal."),
PS("Duke Nukem Forever", ["57900"], ["dukeforever.exe"], "industrial", "Las Vegas gold and red."),
PS("Duke Nukem 3D: 20th Anniversary World Tour", ["434050"], ["duke3d.exe"], "industrial", "Los Angeles gold and red.", n=PART, exenote=" The executable is from Discord's list."),
PS("Evolve Stage 2", ["273350"], ["evolve.exe"], "nature", "Shear jungle green and orange."),
PS("Spec Ops: The Line", ["50300"], ["specopstheline.exe"], "industrial", "Dubai sand and dust."),
PS("The Bureau: XCOM Declassified", ["65930"], ["thebureau.exe"], "military", "1960s olive and amber."),
PS("The Darkness II", ["67370"], ["darknessii.exe"], "horror", "comic-ink black and red.", n=PART),
PS("Max Payne 3", ["204100"], ["maxpayne3.exe"], "noir", "Sao Paulo noir and amber.", n=PART),
PS("Red Dead Redemption", ["2668510"], ["rdr.exe", "playrdr.exe"], "parchment", "Old West dust and tan.", n="The wiki says controller support is limited."),
PS("Red Dead Redemption 2", ["1174180"], ["rdr2.exe", "playrdr2.exe"], "parchment", "Old West dust and tan.", n=PART),
F("Prey", ["3970"], ["prey.exe"], "scifi", "Sphere-ship green and orange."),
# Rockstar and 2K open-world and action
P("Grand Theft Auto III", ["12100"], ["gta3.exe"], "industrial", "Liberty City grey and green.", n=PART + " " + XI),
P("Grand Theft Auto: Vice City", ["12110"], ["gta-vc.exe", "gta--vc.exe"], "casino", "Vice City neon pink and teal.", n=PART + " " + XI),
P("Grand Theft Auto: San Andreas", ["12120"], ["gta-sa.exe", "gta_sa.exe"], "industrial", "San Andreas orange and green.", n=XI),
P("Grand Theft Auto III - The Definitive Edition", ["1546970"], ["playgta3.exe", "libertycity.exe"], "industrial", "Liberty City grey and green."),
P("Grand Theft Auto: Vice City - The Definitive Edition", ["1546990"], ["playgtavicecity.exe", "vicecity.exe"], "casino", "Vice City neon pink and teal."),
P("Grand Theft Auto: San Andreas - The Definitive Edition", ["1547000"], ["playgtasanandreas.exe", "sanandreas.exe"], "industrial", "San Andreas orange and green."),
P("Grand Theft Auto IV", ["12210"], ["gtaiv.exe", "playgtaiv.exe"], "industrial", "Liberty City grey and blue.", n=PART),
P("Grand Theft Auto: Episodes from Liberty City", ["12220"], ["eflc.exe", "launcheflc.exe"], "industrial", "Liberty City grey and blue.", n="Steam gives no controller details while the wiki lists support."),
P("Grand Theft Auto V", ["271590"], ["gta5.exe"], "industrial", "Los Santos sunset orange and blue."),
P("Grand Theft Auto V Enhanced", ["3240220"], ["gta5_enhanced.exe", "gta5_enhanced_be.exe"], "industrial", "Los Santos sunset orange and blue.", n=PART, ctl=["dualshock"]),
KB("Grand Theft Auto", ["12170"], ["grand theft auto.exe"], "industrial", "top-down city grey and red.", b={"South": ("k:ENTER", "Enter / leave car", None), "East": ("k:LEFTCTRL", "Fire", None), "West": ("k:SPACE", "Handbrake", None)}),
KB("Grand Theft Auto 2", ["12180"], ["gta2.exe"], "industrial", "top-down city grey and yellow.", b={"South": ("k:ENTER", "Enter / leave car", None), "East": ("k:LEFTCTRL", "Fire", None), "West": ("k:SPACE", "Handbrake", None)}),
P("Bully: Scholarship Edition", ["12200"], ["bully.exe"], "paper", "school-yard blue and red.", n=PART),
P("Manhunt", ["12130"], ["manhunt.exe"], "horror", "snuff-film grey and red.", n=XI),
P("Midnight Club II", ["12160"], ["mc2.exe"], "industrial", "street-racing orange and grey.", n=XI),
P("Mafia", ["40990"], ["mafia/game.exe"], "noir", "Lost Heaven noir and amber.", n=XI, exenote=FOLDER),
P("Mafia: Definitive Edition", ["1030840"], ["mafiadefinitiveedition.exe"], "noir", "Lost Heaven noir and amber."),
P("Mafia II", ["50130"], ["mafia2.exe"], "noir", "Empire Bay noir and amber.", n=PART),
P("Mafia II: Definitive Edition", ["1030830"], ["mafia ii definitive edition.exe"], "noir", "Empire Bay noir and amber."),
P("Mafia III", ["360430"], ["mafia3definitiveedition.exe"], "industrial", "New Bordeaux swamp green and amber."),
P("Mafia: The Old Country", ["1941540"], ["mafiatheoldcountry.exe"], "parchment", "Sicilian sand and olive.", ctl=["dualshock"]),
P("Marvel's Midnight Suns", ["368260"], ["midnightsuns.exe", "midnightsuns-win64-shipping.exe"], "fighter", "comic purple and gold."),
P("New Tales from the Borderlands", ["1454970"], ["newtales.exe"], "industrial", "Pandora dust and orange."),
P("Tales from the Borderlands", ["330830"], [], "industrial", "Pandora dust and orange."),
P("The Quarry", ["1577120"], ["thequarry.exe", "thequarry-win64-shipping.exe"], "horror", "summer-camp green and red."),
P("Lego 2K Drive", ["1451810"], ["lego2kdrive.exe", "lego2kdrive-win64-shipping.exe"], "fighter", "brick red and yellow."),
P("Blanc", ["1982340"], ["blanc.exe"], "calm", "snow white and blue."),
P("Godfall", ["928960"], ["aperion.exe", "aperion-win64-shipping.exe"], "mystic", "Aperion gold and violet."),
P("Tribes of Midgard", ["858820"], ["tom.exe", "tom-win64-shipping.exe"], "moonlit", "Norse winter blue and silver."),
P("Risk of Rain 2", ["632360"], ["risk of rain 2.exe"], "scifi", "alien-moon teal and orange.", ctl=["dualshock"]),
P("Risk of Rain Returns", ["1337520"], ["risk of rain returns.exe"], "pixel", "alien-moon teal and orange."),
P("OlliOlli World", ["1190170"], ["olliolli world.exe"], "fighter", "Radlandia pink and teal.", ctl=["dualshock"]),
P("Rollerdrome", ["1294420"], ["rollerdrome.exe"], "scifi", "arena orange and black.", ctl=["dualshock"]),
P("The Misadventures of P.B. Winterbottom", ["40930"], ["winterbottom.exe"], "paper", "silent-film grey and sepia.", n=PART + " The wiki lists no controller support while Steam lists partial support."),
P("Carnival Games", ["1249740"], ["carnival games launcher.exe"], "fighter", "fairground red and yellow.", n=PART),
P("Project Ethos", ["2139090"], [], "scifi", "arena blue and gold.", n=SOON, ctl=["dualshock"]),
# Sports
P("NBA 2K10", ["40920"], ["nba2k10.exe"], "fighter", "hardwood orange and blue.", n=XI),
P("NBA 2K11", ["65950"], ["nba2k11.exe"], "fighter", "hardwood orange and blue.", n=XI),
P("NBA 2K12", ["201020"], ["nba2k12.exe"], "fighter", "hardwood orange and blue.", n=PART),
P("NBA 2K13", ["219600"], ["nba2k13.exe"], "fighter", "hardwood orange and blue.", n=XI),
P("NBA 2K14", ["255480"], ["nba2k14.exe"], "fighter", "hardwood orange and blue.", n=XI),
P("NBA 2K15", ["282350"], ["nba2k15.exe"], "fighter", "hardwood orange and blue."),
P("NBA 2K16", ["370240"], ["nba2k16.exe"], "fighter", "hardwood orange and blue."),
P("NBA 2K17", ["385760"], ["nba2k17.exe"], "fighter", "hardwood orange and blue."),
P("NBA 2K18", ["577800"], ["nba2k18.exe"], "fighter", "hardwood orange and blue."),
P("NBA 2K19", ["841370"], ["nba2k19.exe"], "fighter", "hardwood orange and blue."),
P("NBA 2K20", ["1089350"], ["nba2k20.exe"], "fighter", "hardwood orange and blue."),
P("NBA 2K21", ["1225330"], ["nba2k21.exe"], "fighter", "hardwood orange and blue."),
P("NBA 2K22", ["1644960"], ["nba2k22.exe"], "fighter", "hardwood orange and blue."),
P("NBA 2K23", ["1919590"], ["nba2k23.exe"], "fighter", "hardwood orange and blue."),
P("NBA 2K24", ["2338770"], ["nba2k24.exe"], "fighter", "hardwood orange and blue.", n=PART),
P("NBA 2K25", ["2878980"], ["nba2k25.exe"], "fighter", "hardwood orange and blue.", n=PART),
P("NBA 2K26", ["3472040"], ["nba2k26.exe"], "fighter", "hardwood orange and blue.", n=PART, ctl=["dualshock"]),
P("NBA 2K Playgrounds 2", ["726590"], ["nbaplaygrounds2.exe"], "fighter", "street-court orange and purple."),
P("WWE 2K15", ["240460"], ["wwe2k15launcher.exe"], "fighter", "ring red and gold."),
P("WWE 2K16", ["385730"], ["wwe2k16.exe"], "fighter", "ring red and gold."),
P("WWE 2K17", ["510510"], ["wwe2k17_x64.exe"], "fighter", "ring red and gold."),
P("WWE 2K18", ["664430"], ["wwe2k18_x64.exe"], "fighter", "ring red and gold."),
P("WWE 2K19", ["817130"], ["wwe2k19_x64.exe"], "fighter", "ring red and gold."),
P("WWE 2K20", ["1015500"], ["wwe2k20_x64.exe"], "fighter", "ring red and gold."),
P("WWE 2K22", ["1255630"], ["wwe2k22_x64.exe"], "fighter", "ring red and gold.", n=PART),
P("WWE 2K23", ["1942660"], ["wwe2k23_x64.exe"], "fighter", "ring red and gold.", n=PART),
P("WWE 2K24", ["2315690"], ["wwe2k24_x64.exe"], "fighter", "ring red and gold.", n=PART, ctl=["dualshock"]),
P("WWE 2K25", ["2878960"], ["wwe2k25_x64.exe"], "fighter", "ring red and gold.", n=PART, ctl=["dualshock"]),
P("WWE 2K26", ["3717070"], ["wwe2k26_x64.exe"], "fighter", "ring red and gold.", n=PART, ctl=["dualshock"]),
P("WWE 2K Battlegrounds", ["1142100"], ["wweplaygrounds-win64-shipping.exe", "wwebattlegrounds.exe"], "fighter", "arcade ring red and yellow."),
P("PGA Tour 2K21", ["1016120"], ["pga tour 2k21/golf.exe"], "nature", "fairway green and white.", exenote=FOLDER),
P("PGA Tour 2K23", ["1588010"], ["pgatour2k23.exe"], "nature", "fairway green and white."),
P("PGA Tour 2K25", ["2385530"], ["pgatour2k25.exe"], "nature", "fairway green and white.", ctl=["dualshock"]),
P("The Golf Club 2019 featuring PGA Tour", ["695290"], [], "nature", "fairway green and white.", n=GOLF),
P("Major League Baseball 2K10", ["50120"], ["mlb2k10.exe"], "fighter", "ballpark navy and red.", n=PART),
P("Major League Baseball 2K12", ["207060"], ["mlb2k12.exe"], "fighter", "ballpark navy and red.", n=XI),
P("TopSpin 2K25", ["1785650"], ["topspin2k25.exe"], "calm", "hard-court blue and green.", n=PART),
# Old PC games
CW("Army Men", ["549160"], ["armymen.exe"], "military", "plastic green and tan", n="The wiki lists limited controller support; this pack plays it with the pointer and WASD."),
CW("Army Men II", ["549170"], ["armymen2.exe"], "military", "plastic green and tan"),
CW("Army Men: Toys in Space", ["549180"], ["armymentis.exe"], "scifi", "plastic space green and orange"),
T("Army Men: RTS", ["694500"], ["amrts.exe"], "military", "plastic green and tan"),
C("Axel & Pixel", ["8970"], ["ap.exe"], "paper", "graphic-novel grey and red"),
T("CivCity: Rome", ["3980"], ["civcity rome.exe"], "parchment", "Roman marble and gold", b={"North": ("k:SPACE", "Pause", "pause")}, n="Space pausing is a guess."),
T("Freedom Force", ["8880"], ["fforce.exe"], "fighter", "comic red and blue", b={"North": ("k:SPACE", "Pause", "pause")}, n="Space pausing is a guess."),
T("Freedom Force vs. The 3rd Reich", ["8890"], ["ffvt3r.exe"], "fighter", "comic red and blue", b={"North": ("k:SPACE", "Pause", "pause")}, n="Space pausing is a guess."),
CW("Hidden & Dangerous: Action Pack", ["703310"], ["hde.exe"], "military", "WWII khaki and red", slug="hidden-and-dangerous-action-pack", n="This is Steam's one app for the original, the Deluxe edition and the Fight for Freedom add-on."),
CW("Hidden & Dangerous 2", ["703320"], ["hd2.exe", "hd2_sabresquadron.exe"], "military", "WWII khaki and red"),
T("Railroad Tycoon II", ["7620"], ["rt2_plat.exe"], "parchment", "steam-age brown and gold"),
T("Shattered Union", ["3960"], ["shatteredunion.exe"], "military", "fallout grey and red"),
C("Sid Meier's Ace Patrol", ["244070"], ["acepatrol.exe"], "azure", "biplane sky blue and cream"),
C("Sid Meier's Ace Patrol: Pacific Skies", ["244090"], ["acepatrol2.exe"], "azure", "Pacific blue and cream"),
T("Sid Meier's Civilization IV: Colonization", ["16810"], ["colonization.exe"], "parchment", "colonial parchment and blue", b={"North": ("k:SPACE", "End turn", "hourglass")}, n="End turn on Space is a guess."),
T("Sid Meier's Starships", ["282210"], ["starships.exe", "starships64.exe"], "scifi", "fleet blue and silver", b={"North": ("k:SPACE", "End turn", "hourglass")}, n="End turn on Space is a guess."),
CW("X-COM: Enforcer", ["7770"], [], "scifi", "alien-base green and grey", n="The wiki lists controller support but Steam does not."),
CW("X-COM: Interceptor", ["7730"], ["interceptor.exe"], "scifi", "space blue and silver", n="The wiki lists controller support but Steam does not."),
T("X-COM: Apocalypse", ["7660"], [], "scifi", "Mega-Primus grey and orange", n="Steam starts it through DOSBox, so this pack has no executable rule and matches by Steam id alone."),
C("X-COM: Terror from the Deep", ["7650"], [], "scifi", "deep-sea teal and grey", n="Steam starts it through a launcher, so this pack matches by Steam id alone."),
]
