from helpers import *
# Games GameMill Entertainment publishes, not covered by an earlier pack.
# Selection: a live Steam game whose Steam publisher is GameMill Entertainment (wiki names it too), no pack by Steam id or name.
# P = passthrough, for games with controller support (PS = passthrough plus a gyro profile, for shooters); T = strategy with a pointer;
# C / CW = pointer (and WASD) pack; F = mouse-and-keyboard shooter with a gyro profile; KB = keyboard game (pad support before XInput or none).
# Controller support: Steam's flag (full or partial) when it has one, else the PCGamingWiki snapshot; "dualshock" / "switch_pro" are ticked only where the wiki says so.
# Steam ids are each game's base app; executables come from Steam's launch data (Discord's list where Steam has none) and are left out when the name is generic
# or another pack already uses it (that pack then matches by Steam id alone). Genre-based kind and theme were assigned by script and corrected by hand for the
# pointer and keyboard games.
# Left out everywhere: VR games, DLC, demos, tools and soundtracks, games with no release date or not out yet, games delisted from Steam, and games that already have a pack.
S = []
S += [
P("Avatar: The Last Airbender - Quest for Balance", ["1620030"], ["ala-win64-shippingdrm.exe"], "coldsteel", "steel blue and grey."),
P("Big Buck Hunter Arcade", ["455700"], ["buck.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
C("Castle: Never Judge a Book by its Cover", ["301740"], ["castle_neverjudgeabookbyitscover.exe"], "paper", "paper cream and ink", exenote=" The Windows executable is from Steam's launch data."),
P("Cobra Kai 2: Dojos Rising", ["1681870"], ["cobra kai 2 - dojos rising.exe"], "coldsteel", "steel blue and grey."),
P("Cobra Kai: The Karate Kid Saga Continues", ["1279920"], ["cobra kai the karate kid saga continues.exe"], "fighter", "arena red and gold.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Deer Hunter: Reloaded", ["361370"], ["deerhunter.exe"], "nature", "leaf green and earth brown."),
P("DreamWorks All-Star Kart Racing", ["2224740"], ["dwr-win64-shippingdrm.exe"], "scifi", "cold blue and cyan."),
P("DreamWorks Trolls Remix Rescue", ["2199930"], ["tr.exe"], "coldsteel", "steel blue and grey."),
PS("G.I. Joe: Operation Blackout", ["1432220"], ["gijoe.exe"], "coldsteel", "steel blue and grey."),
P("Goosebumps: Terror in Little Creek", ["3080510"], [], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
P("Goosebumps: The Game", ["361360"], [], "coldsteel", "steel blue and grey."),
P("Hot Wheels Let's Race: Ultimate Speed", ["3040910"], ["hwr-win64-shippingdrm.exe"], "fighter", "arena red and gold."),
P("Hot Wheels Monster Trucks: Stunt Mayhem", ["2468400"], ["htw.exe"], "fighter", "arena red and gold."),
P("Little League World Series Baseball 2022", ["1681890"], ["baseball.exe"], "coldsteel", "steel blue and grey."),
P("Looney Tunes: Wacky World of Sports", ["2330430"], ["lts-win64-shippingdrm.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
P("Miraculous - Paris Under Siege", ["2676410"], ["valentine.exe"], "coldsteel", "steel blue and grey."),
P("Miraculous: Rise of the Sphinx", ["1740660"], ["miraculousriseofthesphinx.exe"], "fighter", "arena red and gold.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Monster Jam Battlegrounds", ["336250"], [], "coldsteel", "steel blue and grey."),
P("NASCAR Arcade Rush", ["2192060"], ["nascar.exe"], "fighter", "arena red and gold."),
PS("Nerf Legends", ["1279910"], ["projectdart.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
KB("NHRA Championship Drag Racing: Speed for All", ["1681880"], ["nhra-win64-debuggame.exe", "nhra-win64-shipping.exe", "nhra-win64-test.exe"], "fighter", "arena red and gold", exenote=" The Windows executable is from Steam's launch data."),
P("Nickelodeon All-Star Brawl", ["1414850"], ["nickelodeon all-star brawl.exe"], "scifi", "cold blue and cyan."),
P("Nickelodeon All-Star Brawl 2", ["2017080"], ["nickelodeon all-star brawl 2.exe"], "scifi", "cold blue and cyan.", ctl=["dualshock"]),
P("Nickelodeon Kart Racers 2: Grand Prix", ["1383960"], ["gamesteamdrm.exe"], "fighter", "arena red and gold.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Nickelodeon Kart Racers 3: Slime Speedway", ["1620040"], ["nkr3.exe"], "fighter", "arena red and gold.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Nicktoons & The Dice of Destiny", ["3051490"], ["dice.exe"], "mystic", "violet and gold."),
P("Pickleball Smash", ["2343740"], ["pickleball.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Rudolph the Red-Nosed Reindeer", ["3527810"], ["rudolph the red nosed reindeer.exe"], "coldsteel", "steel blue and grey."),
P("Skull Island: Rise of Kong", ["2097890"], ["kong.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Snoopy & The Great Mystery Club", ["3328180"], ["peanuts.exe"], "coldsteel", "steel blue and grey."),
P("Street Outlaws 2: Winner Takes All", ["1451950"], ["streetoutlaws-win64-debuggame.exe", "streetoutlaws-win64-shipping.exe"], "fighter", "arena red and gold."),
P("Street Outlaws: The List", ["987330"], ["street outlaws the list_us.exe"], "fighter", "arena red and gold."),
P("The Karate Kid: Street Rumble", ["2549780"], ["the karate kid street rumble.exe"], "fighter", "arena red and gold.", ctl=["dualshock"]),
P("The Walking Dead: Destinies", ["2192790"], ["the walking dead destinies.exe"], "horror", "blood red and grey."),
PS("Zombieland: Double Tap - Road Trip", ["995430"], ["zombieland.exe"], "horror", "blood red and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
]
