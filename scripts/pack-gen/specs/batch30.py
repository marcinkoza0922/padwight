from helpers import *
# Games Warner Bros. Games publishes (WB Games, Rocksteady, NetherRealm, Monolith, TT Games), not covered by an earlier pack.
# Selection: a live Steam game whose Steam publisher is a Warner Bros. label.
# P = passthrough, for games with controller support (PS = passthrough plus a gyro profile, for shooters); T = strategy with a pointer;
# C / CW = pointer (and WASD) pack; F = mouse-and-keyboard shooter with a gyro profile; KB = keyboard game (pad support before XInput or none).
# Controller support: Steam's flag (full or partial) when it has one, else the PCGamingWiki snapshot; "dualshock" / "switch_pro" are ticked only where the wiki says so.
# Steam ids are each game's base app; executables come from Steam's launch data (Discord's list where Steam has none) and are left out when the name is generic
# or another pack already uses it (that pack then matches by Steam id alone). Genre-based kind and theme were assigned by script and corrected by hand for the
# pointer and keyboard games.
# Left out everywhere: VR games, DLC, demos, tools and soundtracks, games with no release date or not out yet, games delisted from Steam, and games that already have a pack.
S = []
S += [
PS("Back 4 Blood", ["924970"], ["back4blood.exe", "gobi-win64-shipping.exe", "gobi-win64-test.exe"], "horror", "blood red and grey.", ctl=["dualshock"]),
P("Batman: Arkham City", ["57400"], ["batmanac.exe"], "noir", "noir grey and amber.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard.", ctl=["dualshock"]),
P("Batman: Arkham Knight", ["208650"], ["batmanak.exe"], "noir", "noir grey and amber.", ctl=["dualshock"]),
P("Batman: Arkham Origins", ["209000"], ["batmanoriginsonline.exe", "batmanorigins.exe"], "noir", "noir grey and amber."),
P("Batman: Arkham Origins Blackgate Deluxe Edition", ["267490"], [], "noir", "noir grey and amber."),
PS("Blood: Refreshed Supply", ["3884520"], ["anuket_shipping_playfab_steam_x64.exe", "bloodrefreshedsupply.exe", "dosbox-staging.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock", "switch_pro"]),
PS("F.E.A.R. 3", ["21100"], ["f.e.a.r. 3.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
PS("F.E.A.R. Perseus Mandate", ["21120"], [], "coldsteel", "steel blue and grey."),
P("Gauntlet (2014)", ["258970"], ["gauntlet.exe"], "coldsteel", "steel blue and grey."),
P("Gotham Knights", ["1496790"], ["gothamknights.exe", "gothamknightsnooverlay.exe"], "mystic", "violet and gold."),
P("Guardians of Middle-earth", ["111900"], ["gome.exe"], "mystic", "violet and gold."),
P("Harry Potter: Quidditch Champions", ["2878600"], ["quidditchchampions.exe"], "mystic", "violet and gold.", ctl=["dualshock"]),
P("Hogwarts Legacy", ["990080"], ["hogwartslegacy.exe", "phoenix-win64-test.exe"], "mystic", "violet and gold.", ctl=["dualshock"]),
P("Hot Wheels World's Best Driver", ["236530"], [], "fighter", "arena red and gold."),
P("Injustice 2", ["627270"], ["injustice2.exe"], "fighter", "arena red and gold.", ctl=["dualshock"]),
P("Injustice: Gods Among Us", ["242700"], ["injustice.exe"], "fighter", "arena red and gold."),
P("Lego Batman 2: DC Super Heroes", ["213330"], ["legobatman2.exe"], "fighter", "arena red and gold."),
P("Lego Batman 3: Beyond Gotham", ["313690"], ["legobatman3.exe", "legobatman3_dx11.exe"], "fighter", "arena red and gold."),
P("Lego Batman: Legacy of the Dark Knight", ["2215200"], ["legobatmanlotdk.exe"], "fighter", "arena red and gold.", ctl=["dualshock"]),
P("Lego Batman: The Videogame", ["21000"], ["legobatman.exe"], "fighter", "arena red and gold.", ctl=["dualshock"]),
P("Lego City Undercover", ["578330"], ["legolcur_dx11.exe"], "fighter", "arena red and gold."),
P("Lego DC Super-Villains", ["829110"], ["lego dc super-villains_dx11.exe"], "fighter", "arena red and gold."),
P("Lego Harry Potter Collection", ["2950340"], ["lhpc.exe"], "mystic", "violet and gold."),
P("Lego Harry Potter: Years 1-4", ["21130"], ["legoharrypotter.exe"], "mystic", "violet and gold."),
P("Lego Harry Potter: Years 5-7", ["204120"], ["harry2.exe"], "mystic", "violet and gold."),
P("Lego Jurassic World", ["352400"], ["legojurassicworld.exe"], "fighter", "arena red and gold."),
P("Lego Marvel Super Heroes", ["249130"], ["legomarvel.exe"], "fighter", "arena red and gold."),
P("Lego Marvel Super Heroes 2", ["647830"], ["legomarvel2_dx11.exe"], "fighter", "arena red and gold.", ctl=["switch_pro"]),
P("Lego Marvel's Avengers", ["405310"], ["legomarvelavengers.exe"], "fighter", "arena red and gold."),
P("Lego Star Wars: The Force Awakens", ["438640"], ["legoswtfa.exe"], "scifi", "cold blue and cyan."),
P("Lego Star Wars: The Skywalker Saga", ["920210"], ["legostarwarsskywalkersaga_dx11.exe"], "scifi", "cold blue and cyan.", ctl=["dualshock", "switch_pro"]),
P("Lego The Hobbit", ["285160"], ["legohobbit.exe"], "mystic", "violet and gold."),
P("Lego The Incredibles", ["818320"], ["lego the incredibles.exe", "lego the incredibles_dx11.exe"], "fighter", "arena red and gold."),
P("Lego The Lord of the Rings", ["214510"], ["legolotr.exe"], "mystic", "violet and gold."),
P("Lego Worlds", ["332310"], ["lego_worlds.exe", "lego_worlds_dx11.exe"], "fighter", "arena red and gold."),
P("Mad Max", ["234140"], ["madmax.exe"], "industrial", "steel grey and orange."),
P("Middle-earth: Shadow of Mordor", ["241930"], ["shadowofmordor.exe"], "mystic", "violet and gold."),
P("Middle-earth: Shadow of War", ["356190"], ["shadowofwar.exe"], "mystic", "violet and gold."),
P("Mortal Kombat 1", ["1971870"], ["mk12.exe"], "fighter", "arena red and gold."),
P("Mortal Kombat 11", ["976310"], ["mk11game-retail.exe", "mk11game-retailcheatson.exe", "mk11.exe"], "fighter", "arena red and gold."),
P("Mortal Kombat Arcade Kollection", ["205350"], ["mkhdgame.exe"], "fighter", "arena red and gold."),
P("Mortal Kombat Komplete Edition", ["237110"], ["mkke.exe"], "fighter", "arena red and gold."),
P("Mortal Kombat X", ["307780"], ["mk10.exe"], "fighter", "arena red and gold."),
P("MultiVersus", ["1818750"], ["multiversus.exe"], "fighter", "arena red and gold.", ctl=["dualshock"]),
P("Scooby Doo! & Looney Tunes Cartoon Universe: Adventure", ["294280"], ["cartoonuniverse.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
C("Scribblenauts Unmasked: A DC Comics Adventure", ["249870"], [], "calm", "soft pastel tones"),
PS("Suicide Squad: Kill the Justice League", ["315210"], [], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
P("Terminator: Salvation", ["21080"], ["terminatorsalvation.exe"], "military", "olive drab and khaki.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("The Lego Movie 2 Videogame", ["881320"], ["lego the lego movie 2_dx11.exe"], "fighter", "arena red and gold."),
P("The Lego Movie Videogame", ["267530"], ["legoemmet.exe"], "fighter", "arena red and gold."),
P("The Lego Ninjago Movie Video Game", ["640590"], ["legoninjago.exe", "legoninjago_dx11.exe"], "fighter", "arena red and gold."),
P("The Lord of the Rings: War in the North", ["32800"], ["witn.exe"], "mystic", "violet and gold."),
PS("Wanted: Weapons of Fate", ["21070"], ["wanted.exe"], "coldsteel", "steel blue and grey.", n="The wiki lists controller support but is not sure the game reads XInput: if the pad does nothing in the game, set the controller type in its options or run it through Steam Input."),
P("Watchmen: The End is Nigh", ["21010"], [], "fighter", "arena red and gold."),
P("Watchmen: The End is Nigh Part 2", ["21030"], [], "fighter", "arena red and gold.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
C("Westworld Awakening", ["1133320"], ["labescape-win64-shipping.exe"], "noir", "noir grey and amber", exenote=" The Windows executable is from Steam's launch data."),
]
