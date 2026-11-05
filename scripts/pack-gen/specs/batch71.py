from helpers import *
# Games Night Dive Studios (Nightdive Studios on Steam, including its Humongous Entertainment re-releases) publishes, not covered by an earlier pack.
# Selection: a live Steam game whose Steam publisher is that label (wiki names it too), no pack by Steam id or name.
# P = passthrough, for games with controller support (PS = passthrough plus a gyro profile, for shooters); T = strategy with a pointer;
# C / CW = pointer (and WASD) pack; F = mouse-and-keyboard shooter with a gyro profile; KB = keyboard game (pad support before XInput or none).
# Controller support: Steam's flag (full or partial) when it has one, else the PCGamingWiki snapshot; "dualshock" / "switch_pro" are ticked only where the wiki says so.
# Steam ids are each game's base app; executables come from Steam's launch data (Discord's list where Steam has none) and are left out when the name is generic
# or another pack already uses it (that pack then matches by Steam id alone). Genre-based kind and theme were assigned by script and corrected by hand for the
# pointer and keyboard games.
# Left out everywhere: VR games, DLC, demos, tools and soundtracks, games with no release date or not out yet, games delisted from Steam, and games that already have a pack.
S = []
S += [
KB("1942: The Pacific Air War", ["347230"], [], "military", "olive drab and khaki"),
C("Big Thinkers 1st Grade", ["376770"], [], "calm", "soft pastel tones"),
C("Big Thinkers Kindergarten", ["376760"], [], "calm", "soft pastel tones"),
C("D: The Game", ["510590"], [], "horror", "blood red and grey"),
C("Fatty Bear's Birthday Surprise", ["317030"], [], "calm", "soft pastel tones"),
C("Freddi Fish 2: The Case of the Haunted Schoolhouse", ["294530"], [], "calm", "soft pastel tones"),
C("Freddi Fish 3: The Case of the Stolen Conch Shell", ["294540"], [], "calm", "soft pastel tones"),
C("Freddi Fish 4: The Case of the Hogfish Rustlers of Briny Gulch", ["294550"], [], "calm", "soft pastel tones"),
C("Freddi Fish 5: The Case of the Creature of Coral Cove", ["294570"], [], "calm", "soft pastel tones"),
C("Freddi Fish and Luther's Maze Madness", ["284020"], [], "calm", "soft pastel tones"),
C("Freddi Fish and Luther's Water Worries", ["294580"], [], "calm", "soft pastel tones"),
C("Freddi Fish and the Case of the Missing Kelp Seeds", ["283940"], [], "paper", "paper cream and ink"),
C("I Have No Mouth, and I Must Scream", ["245390"], ["ihnmaims.exe"], "horror", "blood red and grey", exenote=" The Windows executable is from Steam's launch data."),
C("Inherit the Earth: Quest for the Orb", ["257260"], ["itewin.exe"], "calm", "soft pastel tones", exenote=" The Windows executable is from Steam's launch data."),
KB("Knights of the Sky", ["347270"], [], "military", "olive drab and khaki"),
C("Let's Explore the Airport (Junior Field Trips)", ["363050"], [], "calm", "soft pastel tones"),
C("Let's Explore the Farm (Junior Field Trips)", ["363060"], [], "nature", "leaf green and earth brown"),
C("Let's Explore the Jungle (Junior Field Trips)", ["363070"], [], "nature", "leaf green and earth brown"),
F("MadSpace: To Hell and Beyond", ["351810"], [], "scifi", "cold blue and cyan", n="The game has some controller support, but it predates XInput or is limited, so this pack plays it with the keyboard keys."),
C("Noctropolis", ["377070"], ["noctropolis.exe"], "noir", "noir grey and amber", n="The game has some controller support, but it predates XInput or is limited, so this pack plays it with the keyboard keys.", exenote=" The Windows executable is from Steam's launch data."),
C("Pajama Sam 2: Thunder and Lightning Aren't So Frightening", ["292780"], [], "calm", "soft pastel tones"),
C("Pajama Sam 3: You Are What You Eat from Your Head to Your Feet", ["292800"], [], "calm", "soft pastel tones"),
C("Pajama Sam 4: Life Is Rough When You Lose Your Stuff!", ["292820"], ["pajamalrs.exe"], "calm", "soft pastel tones", exenote=" The Windows executable is from Steam's launch data."),
C("Pajama Sam's Lost & Found", ["292860"], [], "calm", "soft pastel tones"),
C("Pajama Sam's Sock Works", ["292840"], [], "calm", "soft pastel tones"),
C("Pajama Sam: Games to Play on Any Day", ["317020"], [], "calm", "soft pastel tones"),
C("Pajama Sam: No Need to Hide When It's Dark Outside", ["283960"], [], "calm", "soft pastel tones"),
C("Putt-Putt and Fatty Bear's Activity Pack", ["294720"], [], "calm", "soft pastel tones"),
C("Putt-Putt and Pep's Balloon-o-Rama", ["284000"], [], "calm", "soft pastel tones"),
C("Putt-Putt and Pep's Dog on a Stick", ["294710"], [], "calm", "soft pastel tones"),
C("Putt-Putt Enters the Race", ["294680"], [], "calm", "soft pastel tones"),
C("Putt-Putt Goes to the Moon", ["294650"], [], "calm", "soft pastel tones"),
C("Putt-Putt Joins the Circus", ["294690"], [], "calm", "soft pastel tones"),
C("Putt-Putt Joins the Parade", ["283920"], [], "calm", "soft pastel tones"),
C("Putt-Putt Saves the Zoo", ["294660"], [], "calm", "soft pastel tones"),
C("Putt-Putt Travels through Time", ["294670"], [], "calm", "soft pastel tones"),
C("Putt-Putt: Pep's Birthday Surprise", ["294700"], ["puttputtpbs.exe"], "calm", "soft pastel tones", exenote=" The Windows executable is from Steam's launch data."),
PS("Spirits of Xanadu", ["312230"], ["spirits of xanadu.exe"], "coldsteel", "steel blue and grey."),
C("Spy Fox 2: Some Assembly Required", ["292240"], [], "calm", "soft pastel tones"),
C("Spy Fox 3: Operation Ozone", ["292260"], [], "calm", "soft pastel tones"),
C("Spy Fox in: Cheese Chase", ["292280"], [], "calm", "soft pastel tones"),
C("Spy Fox in: Hold the Mustard", ["292300"], [], "calm", "soft pastel tones"),
C("Spy Fox: Dry Cereal", ["283980"], [], "calm", "soft pastel tones"),
PS("Strife: Veteran Edition", ["317040"], ["strife-ve.exe"], "mystic", "violet and gold.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
C("Tender Loving Care", ["593900"], ["tlc.exe"], "noir", "noir grey and amber", exenote=" The Windows executable is from Steam's launch data."),
C("The Labyrinth of Time", ["266980"], ["labw.exe"], "calm", "soft pastel tones", exenote=" The Windows executable is from Steam's launch data."),
C("Timelapse", ["433580"], [], "calm", "soft pastel tones"),
C("Titanic: Adventure Out of Time", ["785480"], ["ti.exe"], "calm", "soft pastel tones", exenote=" The Windows executable is from Steam's launch data."),
C("Tony Tough and the Night of Roasted Moths", ["356210"], [], "calm", "soft pastel tones"),
PS("Turok (2015)", ["405820"], ["sobek_shipping_steam_x64.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock", "switch_pro"]),
P("Womb Room", ["426010"], ["wombroom.exe"], "horror", "blood red and grey."),
F("Z.A.R.", ["351820"], [], "scifi", "cold blue and cyan"),
]
