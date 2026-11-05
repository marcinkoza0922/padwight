from helpers import *
# Games Outright Games publishes, not covered by an earlier pack.
# Selection: a live Steam game whose Steam publisher is Outright Games (wiki names it too), no pack by Steam id or name.
# P = passthrough, for games with controller support (PS = passthrough plus a gyro profile, for shooters); T = strategy with a pointer;
# C / CW = pointer (and WASD) pack; F = mouse-and-keyboard shooter with a gyro profile; KB = keyboard game (pad support before XInput or none).
# Controller support: Steam's flag (full or partial) when it has one, else the PCGamingWiki snapshot; "dualshock" / "switch_pro" are ticked only where the wiki says so.
# Steam ids are each game's base app; executables come from Steam's launch data (Discord's list where Steam has none) and are left out when the name is generic
# or another pack already uses it (that pack then matches by Steam id alone). Genre-based kind and theme were assigned by script and corrected by hand for the
# pointer and keyboard games.
# Left out everywhere: VR games, DLC, demos, tools and soundtracks, games with no release date or not out yet, games delisted from Steam, and games that already have a pack.
S = []
S += [
P("Adventure Time: Pirates of the Enchiridion", ["728240"], ["adventure time pirates of the enchiridion.exe"], "coldsteel", "steel blue and grey."),
P("Baby Shark: Sing & Swim Party", ["2075240"], ["baby shark.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Barbie Horse Trails", ["2786660"], ["barbie.exe"], "coldsteel", "steel blue and grey."),
P("Barbie: Project Friendship", ["2478330"], ["barbie project friendship.exe"], "coldsteel", "steel blue and grey."),
P("Ben 10", ["703180"], [], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Ben 10: Power Trip", ["1063040"], [], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Blaze and the Monster Machines: Axle City Racers", ["1434330"], ["blz.exe"], "fighter", "arena red and gold."),
P("Bluey: The Videogame", ["2078350"], ["project biscuits.exe"], "coldsteel", "steel blue and grey."),
C("Bratz Rhythm & Style", ["3212580"], ["bratz.exe"], "calm", "soft pastel tones", exenote=" The Windows executable is from Steam's launch data."),
P("Bratz: Flaunt Your Fashion", ["2088340"], ["bratz flaunt your fashion.exe"], "coldsteel", "steel blue and grey."),
P("Chicken Run: Eggstraction", ["3419690"], ["cr2.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Crayola Scoot", ["772530"], ["crayola scoot.exe"], "coldsteel", "steel blue and grey."),
P("DC League of Super-Pets: The Adventures of Krypto and Ace", ["1697760"], ["dc league of super pets.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("DC's Justice League: Cosmic Chaos", ["1944850"], ["dcjlcc.exe"], "coldsteel", "steel blue and grey."),
P("Dora: Rainforest Rescue", ["3273550"], ["dora rainforest rescue.exe"], "nature", "leaf green and earth brown."),
P("DreamWorks Dragons: Dawn of New Riders", ["896960"], ["dragonsdawnofnewriders.exe"], "mystic", "violet and gold."),
P("DreamWorks Gabby's Dollhouse: Ready to Party", ["3222830"], ["gabbysdollhouse_readytoparty.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Fast & Furious: Spy Racers Rise of SH1FT3R", ["1434320"], ["ff.exe"], "fighter", "arena red and gold.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Gigantosaurus: Dino Kart", ["1770500"], ["gig.exe"], "fighter", "arena red and gold."),
P("Gigantosaurus: The Game", ["1177020"], ["gigantosaurus.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Hotel Transylvania 3: Monsters Overboard", ["757600"], ["hoteltransylvania3.exe"], "coldsteel", "steel blue and grey."),
P("Hotel Transylvania: Scary Tale Adventures", ["1433500"], ["hotel transylvania 4.exe"], "coldsteel", "steel blue and grey."),
P("Ice Age: Scrat's Nutty Adventure", ["751060"], ["iceage.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("JoJo Siwa: Worldwide Party", ["1910300"], ["jojosiwaworldparty.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Jumanji: The Video Game", ["896950"], ["echo.exe"], "coldsteel", "steel blue and grey."),
P("Jumanji: Wild Adventures", ["1823200"], ["jumanji 2.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Matchbox Driving Adventures", ["2322330"], ["matchbox.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
P("My Friend Peppa Pig", ["1635590"], ["my friend peppa pig.exe"], "coldsteel", "steel blue and grey."),
P("My Little Pony: A Maretime Bay Adventure", ["1600780"], ["mlp.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("My Little Pony: A Zephyr Heights Mystery", ["2235440"], ["mylittleponyzephyrheights.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("NBA Bounce", ["3115130"], ["nba bounce.exe"], "fighter", "arena red and gold.", ctl=["dualshock"]),
P("Nick Jr. Party Adventure", ["2339320"], ["nick jr party adventure.exe"], "calm", "soft pastel tones."),
P("PAW Patrol Dino World", ["3525790"], ["pdw.exe"], "coldsteel", "steel blue and grey."),
KB("PAW Patrol Rescue Wheels: Championship", ["3291010"], ["ppr.exe"], "calm", "soft pastel tones", exenote=" The Windows executable is from Steam's launch data."),
P("Paw Patrol The Movie: Adventure City Calls", ["1434310"], ["pptm.exe"], "coldsteel", "steel blue and grey."),
P("PAW Patrol World", ["1952520"], ["ppa.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Paw Patrol: Grand Prix", ["1770490"], ["paw.exe"], "fighter", "arena red and gold."),
P("Paw Patrol: Mighty Pups Save Adventure Bay", ["1374990"], ["pp2.exe"], "coldsteel", "steel blue and grey."),
P("Paw Patrol: On A Roll!", ["758730"], ["pawpatrol.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Peppa Pig: World Adventures", ["1998980"], ["peppa pig 2.exe"], "coldsteel", "steel blue and grey."),
P("PJ Masks: Heroes of the Night", ["1635600"], ["pj masks.exe"], "fighter", "arena red and gold."),
P("Race with Ryan", ["1054990"], ["racewithryan.exe"], "fighter", "arena red and gold."),
P("Rainbow High: Runway Rush", ["2007520"], ["rainbow high runway rush.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Sesame Street: Friends & Fun", ["3930330"], ["sesame_street.exe", "sesame_street_friends_fun.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
P("Spirit: Lucky's Big Adventure", ["1270720"], ["spirit.exe"], "coldsteel", "steel blue and grey."),
P("SpongeBob SquarePants: The Patrick Star Game", ["2322380"], ["patrick.exe"], "scifi", "cold blue and cyan.", ctl=["dualshock"]),
P("Star Trek Prodigy: Supernova", ["1655220"], ["startrekprodigy.exe"], "scifi", "cold blue and cyan."),
P("Teenage Mutant Ninja Turtles: Mutants Unleashed", ["2460130"], ["mutants_unleashed.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("The Addams Family: Mansion Mayhem", ["1543830"], ["the addams family.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("The Cat in the Hat: Rainy Day Mayhem", ["3465710"], ["cat in the hat.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard.", ctl=["dualshock"]),
P("The Elf on the Shelf: Christmas Heroes", ["3210120"], ["elf on the shelf.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("The Grinch 2: Saving Christmas", ["3470930"], ["the grinch 2 saving christmas.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
P("The Grinch: Christmas Adventures", ["1978210"], ["thegrinch.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Transformers: EarthSpark - Expedition", ["1955800"], ["transformers.exe"], "scifi", "cold blue and cyan.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Trollhunters: Defenders of Arcadia", ["1344070"], ["trollhunters_x64.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
]
