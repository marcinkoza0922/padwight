from helpers import *
# Games PQube publishes, not covered by an earlier pack.
# Selection: a live Steam game whose Steam publisher is PQube (wiki names it too), no pack by Steam id or name.
# P = passthrough, for games with controller support (PS = passthrough plus a gyro profile, for shooters); T = strategy with a pointer;
# C / CW = pointer (and WASD) pack; F = mouse-and-keyboard shooter with a gyro profile; KB = keyboard game (pad support before XInput or none).
# Controller support: Steam's flag (full or partial) when it has one, else the PCGamingWiki snapshot; "dualshock" / "switch_pro" are ticked only where the wiki says so.
# Steam ids are each game's base app; executables come from Steam's launch data (Discord's list where Steam has none) and are left out when the name is generic
# or another pack already uses it (that pack then matches by Steam id alone). Genre-based kind and theme were assigned by script and corrected by hand for the
# pointer and keyboard games.
# Left out everywhere: VR games, DLC, demos, tools and soundtracks, games with no release date or not out yet, games delisted from Steam, and games that already have a pack.
S = []
S += [
P("Aggelos", ["717310"], ["aggelos.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Arcade Spirits", ["910630"], ["arcadespirits.exe"], "coldsteel", "steel blue and grey."),
P("Beyond the Ice Palace II", ["2384730"], ["beyond the ice palace ii.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
P("Ciel Fledge", ["738270"], ["cielfledge.exe"], "mystic", "violet and gold."),
P("Class of Heroes: Anniversary Edition", ["2195780"], ["totomono1.exe"], "mystic", "violet and gold.", ctl=["dualshock"]),
P("Discounty", ["2274620"], ["discounty.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
C("Don't Disturb", ["501840"], ["don'tdisturb.exe"], "horror", "blood red and grey", exenote=" The Windows executable is from Steam's launch data."),
P("Evergate", ["845030"], ["evergate.exe"], "coldsteel", "steel blue and grey."),
P("Fallen Knight", ["1378370"], ["fallen knight.exe"], "coldsteel", "steel blue and grey."),
PS("Gal*Gun 2", ["851890"], ["galgun2.exe"], "coldsteel", "steel blue and grey."),
PS("Gal*Gun Returns", ["1425730"], [], "coldsteel", "steel blue and grey."),
PS("Gal*Gun: Double Peace", ["511740"], ["gg2game.exe"], "coldsteel", "steel blue and grey."),
P("Gun Gun Pixies", ["1023970"], [], "coldsteel", "steel blue and grey."),
P("Inertial Drift", ["1184480"], ["inertialdrift.exe"], "fighter", "arena red and gold."),
P("Is It Wrong to Try to Pick Up Girls in a Dungeon? Infinite Combate", ["1187940"], ["danmachi.exe"], "mystic", "violet and gold.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Iwakura Aria", ["3197710"], ["iwakura_aria.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Kitaria Fables", ["1356280"], ["kitaria fables.exe"], "mystic", "violet and gold."),
P("KONOSUBA - God's Blessing on this Wonderful World! Love For These Clothes Of Desire!", ["2349140"], ["konosuba_lfd.exe"], "coldsteel", "steel blue and grey."),
P("Kotodama: The 7 Mysteries of Fujisawa", ["997060"], ["kotodama the 7 mysteries of fujisawa.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
PS("Kusan: City of Wolves", ["1714510"], ["kusan.exe"], "coldsteel", "steel blue and grey."),
P("Metal Max Xeno Reborn", ["1637230"], ["metal max xeno reborn.exe"], "mystic", "violet and gold."),
P("Muddledash", ["769840"], ["muddledash.exe"], "fighter", "arena red and gold."),
P("Nexomon: Extinction", ["1196630"], ["nexomon extinction.exe"], "mystic", "violet and gold."),
P("Nippon Marathon", ["746940"], ["nipponmarathon.exe"], "coldsteel", "steel blue and grey."),
P("Our World Is Ended", ["940650"], ["ourworldisended.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
PS("Panzer Knights", ["1416230"], ["panzer knights.exe"], "mystic", "violet and gold.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("PlataGO! Super Platform Game Maker", ["732400"], ["platago.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Potion Permit", ["1337760"], ["potion permit.exe"], "mystic", "violet and gold."),
P("Punch Line", ["766680"], [], "coldsteel", "steel blue and grey."),
PS("Quantum Replica", ["470090"], ["replicagame.exe"], "coldsteel", "steel blue and grey."),
P("Razed", ["726710"], ["razed.exe"], "coldsteel", "steel blue and grey."),
P("Republic of Pirates", ["2103470"], ["republic of pirates.exe"], "industrial", "steel grey and orange.", ctl=["dualshock"]),
P("Since Memories: Off The Starry Sky", ["2838360"], ["sincememories.exe"], "scifi", "cold blue and cyan."),
P("Sky Oceans: Wings for Hire", ["1691750"], ["sky oceans.exe"], "mystic", "violet and gold.", ctl=["dualshock", "switch_pro"]),
P("Sniper Dan", ["2841650"], ["sniper dan.exe"], "military", "olive drab and khaki.", ctl=["dualshock"]),
P("Songbird Symphony", ["909820"], ["songbird symphony.exe"], "coldsteel", "steel blue and grey."),
P("Street Heat", ["744040"], ["streetheat.exe"], "fighter", "arena red and gold."),
P("Super Bullet Break", ["1767170"], ["superbulletbreak.exe"], "coldsteel", "steel blue and grey."),
P("Supermarket Shriek", ["1086460"], ["supermarket shriek.exe"], "coldsteel", "steel blue and grey."),
P("Tears of Avia", ["461510"], ["toa.exe"], "mystic", "violet and gold."),
P("The Bridge Curse 2: The Extrication", ["2331330"], ["tbc2-win64-shipping.exe"], "horror", "blood red and grey."),
P("The Plane Effect", ["1448760"], ["the_plane_effect.exe"], "coldsteel", "steel blue and grey."),
P("Tormented Souls", ["1367590"], ["tormentedsouls.exe"], "horror", "blood red and grey."),
P("Tormented Souls 2", ["2464280"], ["tormentedsouls2.exe"], "horror", "blood red and grey.", ctl=["dualshock"]),
P("Valthirian Arc: Hero School Story", ["785850"], ["vahss.exe"], "mystic", "violet and gold."),
P("White Day 2: The Flower That Tells Lies", ["2078040"], ["wd.exe"], "horror", "blood red and grey.", ctl=["dualshock"]),
P("White Day: A Labyrinth Named School (2017)", ["466130"], ["whiteday.exe"], "horror", "blood red and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
]
