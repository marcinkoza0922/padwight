from helpers import *
# Games Konami developed or published (the wiki's publisher or developer field), not covered by an earlier pack.
# P = passthrough, for games the wiki or Steam say have controller support and that came out after XInput (Dec 2005).
# PS = passthrough plus a gyro profile, for shooters (see GYRO_PT_NOTE in engine.py).
# G = generated pointer pack, for games with no controller support.
# Left out: games with no Steam id (DOS-era Konami, the 1997-2007 Windows ports, the Japanese arcade-and-online titles such as Beatmania IIDX
# Infinitas, Bombergirl, Pop'n Music Lively and DanceDanceRevolution Grand Prix, the GOG-only Metal Gear (MSX2), Metal Gear Solid: Integral and
# Substance, the Frogger and Yu-Gi-Oh! Power of Chaos ports, P.T. for PC), games whose Steam page is gone (Battle: Los Angeles, Saw, Super
# Bomberman R Online, Professional Baseball Spirits 2024-2025, Pro Evolution Soccer 2013 to 2017, Metal Gear Solid: Master Collection Vol. 2's
# separate game ids), Steam stubs that cannot be bought (Pro Evolution Soccer 2014, Red Bull X-Fighters, the PES trials and Lite editions, DLC and
# bonus-content apps), the DOS Killing Cloud, and games where the wiki lists Konami only as the publisher of someone else's old Windows port.
# Steam ids are each game's base app; demos and DLC are not rules.
S = []
PART = "The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."
XI = "The wiki lists controller support but is not sure the game reads XInput: if the pad does nothing in the game, set the controller type in its options or run it through Steam Input."
SOON = "Not out yet: the wiki and Steam give a date still ahead, so this pack is ready before the game is."
TBA = "Not out yet: Steam gives no release date, so this pack is ready before the game is."
MC = "The collection's Steam page lists full controller support while the wiki calls it limited: some menus may still want the keyboard."
S += [
# --- Metal Gear
PS("Metal Gear Solid V: The Phantom Pain", ["287700"], ["mgsvtpp.exe", "mgsvmgo.exe"], "military", "Mother Base olive and amber."),
PS("Metal Gear Solid V: Ground Zeroes", ["311340"], ["mgsgroundzeroes.exe"], "military", "Camp Omega olive and amber."),
PS("Metal Gear Solid Δ: Snake Eater", ["2417610"], ["mgsdelta.exe"], "military", "jungle olive and amber."),
PS("Metal Gear Survive", ["543900"], ["mgv.exe"], "military", "Dite olive and amber."),
P("Metal Gear Rising: Revengeance", ["235460"], ["metal gear rising revengeance.exe"], "industrial", "Raiden red and steel."),
P("Metal Gear Solid - Master Collection Version", ["2131630"], ["mgs1/metal gear solid.exe"], "military", "Shadow Moses olive and amber.", n=MC),
P("Metal Gear Solid 2: Sons of Liberty - Master Collection Version", ["2131640"], [], "military", "Big Shell olive and amber.", n=MC),
P("Metal Gear Solid 3: Snake Eater - Master Collection Version", ["2131650"], ["mgs3/metal gear solid3.exe"], "military", "jungle olive and amber.", n=MC),
P("Metal Gear & Metal Gear 2: Solid Snake - Master Collection Version", ["2131680"], [], "pixel", "8-bit olive and amber.", n=MC),
P("Metal Gear Solid: Peace Walker - Master Collection Version", ["2492660"], [], "military", "Costa Rica olive and amber.", n=MC),
P("Metal Gear Solid 4: Guns of the Patriots - Master Collection Version", ["2492670"], [], "military", "battlefield olive and amber.", n=MC),
P("Metal Gear Solid: Master Collection Vol. 2", ["3859630"], ["mgs_mc2/mgs_mc2.exe"], "military", "olive and amber.", n=MC + " This pack covers the collection's launcher, which also starts Ghost Babel."),
P("Zone of the Enders: The 2nd Runner - M∀RS", ["650510"], [], "scifi", "Orbital Frame blue and orange.", n=PART),
# --- Silent Hill
PS("Silent Hill 2", ["2124490"], ["shproto.exe", "shproto-win64-shipping.exe"], "horror", "Toluca fog grey and rust."),
P("Silent Hill f", ["2947440"], ["shf.exe", "shf-win64-shipping.exe"], "horror", "Ebisugaoka fog white and red."),
P("Silent Hill: Townfall", ["1636440"], ["townfall.exe"], "horror", "Scottish fog grey and rust."),
PS("Silent Hill: Homecoming", ["19000"], ["silenthill.exe"], "horror", "Shepherd's Glen fog grey and rust.", n=XI),
# --- Castlevania and Contra
P("Castlevania: Lords of Shadow", ["234080"], ["castlevanialosue.exe"], "moonlit", "Gothic crimson and gold."),
P("Castlevania: Lords of Shadow - Mirror of Fate HD", ["282530"], ["cmof.exe"], "moonlit", "Gothic crimson and gold."),
P("Castlevania: Lords of Shadow 2", ["239250"], ["clos2.exe"], "moonlit", "Gothic crimson and gold."),
P("Castlevania: Belmont's Curse", ["4231820"], ["castlevania belmont's curse.exe"], "moonlit", "Belmont crimson and gold."),
P("Castlevania Anniversary Collection", ["1018010"], ["castlevania anniversary collection/game.exe"], "pixel", "8-bit crimson and gold."),
P("Castlevania Advance Collection", ["1552550"], ["castlevania advance collection/game.exe"], "pixel", "handheld crimson and gold."),
P("Castlevania Dominus Collection", ["2369900"], ["castlevania dominus collection/game.exe"], "pixel", "Dracula's castle crimson and gold."),
P("Contra Anniversary Collection", ["1018020"], ["contra anniversary collection/game.exe"], "pixel", "jungle green and red."),
P("Contra: Operation Galuga", ["2235020"], ["contraog.exe"], "pixel", "Galuga jungle green and red."),
PS("Contra: Rogue Corps", ["1020540"], ["contra_rc.exe"], "military", "Neo Contra green and red."),
P("Arcade Classics Anniversary Collection", ["1018000"], ["aa_ac_arcadeclassics.exe"], "pixel", "Konami arcade blue and yellow.", n=PART),
P("Gradius Origins", ["2897590"], ["gradius origin collection.exe"], "pixel", "Vic Viper blue and red.", n=PART),
P("Teenage Mutant Ninja Turtles: The Cowabunga Collection", ["1659600"], ["tmnt_cowabunga.exe"], "pixel", "sewer green and orange."),
P("Rocket Knight Adventures: Re-Sparked!", ["2728190"], ["rocketknight.exe"], "pixel", "Sparkster blue and gold."),
P("Rocket Knight", ["19030"], [], "scifi", "Sparkster blue and gold.", n=PART),
P("Ninja Five-O", ["2728200"], ["ninjafiveo.exe"], "pixel", "kunoichi violet and red."),
P("GetsuFumaDen: Undying Moon", ["1323470"], ["getsufumaden.exe"], "samurai", "Fuma crimson and gold."),
P("Skelattack", ["605800"], ["skelattack.exe"], "moonlit", "skeleton violet and orange."),
P("Super Crazy Rhythm Castle", ["1812560"], ["super crazy rhythm castle.exe"], "casino", "castle purple and gold."),
P("Darwin's Paradox!", ["2989180"], ["darwin's paradox.exe"], "nature", "ocean teal and orange."),
P("Cygni: All Guns Blazing", ["1248080"], ["cygni.exe", "cygni-win64-shipping.exe"], "scifi", "space-opera blue and magenta.", n=PART),
P("Deliver At All Costs", ["1880610"], ["deliver at all costs.exe"], "industrial", "1950s sky blue and orange."),
P("Edens Zero", ["3125750"], ["edenszero.exe"], "scifi", "Edens space blue and gold."),
P("Marvel MaXimum Collection", ["3931060"], ["marvel.exe"], "fighter", "comic-book blue and red."),
P("Bail or Jail", ["1715980"], ["obakeidoro!.exe"], "paper", "ghost-prison violet and orange."),
# --- Bomberman
P("Super Bomberman R", ["702700"], ["superbombermanr.exe", "super bomberman r/superbombermanr.exe"], "pixel", "Bomberman blue and orange.", n=PART),
P("Super Bomberman R 2", ["1913740"], ["superbombermanr2.exe"], "pixel", "Bomberman blue and orange.", n=PART),
P("Super Bomberman Collection", ["3756010"], ["super bomberman collection.exe"], "pixel", "Bomberman blue and orange."),
# --- Yu-Gi-Oh!
P("Yu-Gi-Oh! Master Duel", ["1449850"], ["masterduel.exe"], "casino", "duel-arena purple and gold.", n=XI),
P("Yu-Gi-Oh! Legacy of the Duelist", ["480650"], ["yu-gi-oh! legacy of the duelist/yugioh.exe"], "casino", "duel-arena purple and gold."),
P("Yu-Gi-Oh! Legacy of the Duelist: Link Evolution", ["1150640"], ["yu-gi-oh! legacy of the duelist link evolution/yugioh.exe"], "casino", "Link Vrains purple and cyan."),
P("Yu-Gi-Oh! Early Days Collection", ["3013550"], ["yugioh_early_days_collection.exe"], "pixel", "handheld purple and gold."),
P("Yu-Gi-Oh! Tag Force GX", ["4416300"], [], "casino", "duel-academy red and gold.", n=PART + " " + SOON),
# --- Suikoden, football, tennis and the rest
P("Suikoden I & II HD Remaster Gate Rune and Dunan Unification Wars", ["1932640"], ["suikoden i and ii hd remaster/suikoden i and ii hd remaster.exe"], "parchment", "Suikoden parchment and blue."),
P("Suikoden Star Leap", ["3668850"], [], "parchment", "Suikoden parchment and blue.", n=PART + " " + TBA),
P("eFootball", ["1665460"], ["efootball.exe"], "azure", "pitch green and blue."),
P("eFootball PES 2020", ["996470"], ["pes2020.exe"], "azure", "pitch green and blue.", n=PART),
P("eFootball PES 2021 Season Update", ["1259970"], ["pes2021.exe"], "azure", "pitch green and blue.", n=PART),
P("Pro Evolution Soccer 2018", ["592580"], ["pes2018.exe"], "azure", "pitch green and blue."),
P("Pro Evolution Soccer 2019", ["770240"], ["pes2019.exe"], "azure", "pitch green and blue.", n=PART),
P("Ebaseball: Pro Spirit", ["4128400"], ["prospi.exe"], "azure", "diamond green and blue."),
P("The Prince of Tennis Doki Doki Survival Eternal Passion", ["3907380"], ["dokisaba.exe"], "paper", "school-court green and pink."),
P("The Prince of Tennis Sweet School Festival 0-40 and More", ["3907350"], ["gakupuri.exe"], "paper", "school-court green and pink."),
P("Kick Buds", ["4039320"], [], "azure", "pitch green and blue.", n=TBA),
P("Rhapsody in Scarlet", ["3353430"], [], "moonlit", "scarlet and gold.", n=TBA),
# --- No controller support: the pad plays them with the pointer
G("Casino Inc.", ["361320"], ["cincmenu.exe"], "click", "casino", "The right stick points; RT and A click, LT right-clicks. Overlays in casino green and gold.", n="Steam and the wiki list no controller support."),
G("Puzzle Chronicles", ["19020"], ["puzzlechronicles.exe"], "click", "paper", "The right stick points; RT and A click, LT right-clicks.", n="Steam lists partial controller support, the wiki none."),
G("Crimesight", ["1491710"], ["crimesight.exe"], "click", "industrial", "The right stick points; RT and A click, LT right-clicks. Overlays in case-file grey and red.", n="Steam and the wiki list no controller support."),
G("Theme Park Mystery", ["1518370"], [], "click", "paper", "The right stick points; RT and A click, LT right-clicks.", n="A DOS adventure run through DOSBox; Steam lists no controller support."),
G("Bloodwych", ["1249410"], [], "click", "parchment", "The right stick points; RT and A click, LT right-clicks.", n="A DOS dungeon crawler run through DOSBox; the wiki lists controller support but Steam lists none, and it predates XInput."),
]
