from helpers import *
# Games Annapurna Interactive publishes, not covered by an earlier pack.
# Selection: a live Steam game whose Steam publisher is Annapurna Interactive (wiki names it too), no pack by Steam id or name.
# P = passthrough, for games with controller support (PS = passthrough plus a gyro profile, for shooters); T = strategy with a pointer;
# C / CW = pointer (and WASD) pack; F = mouse-and-keyboard shooter with a gyro profile; KB = keyboard game (pad support before XInput or none).
# Controller support: Steam's flag (full or partial) when it has one, else the PCGamingWiki snapshot; "dualshock" / "switch_pro" are ticked only where the wiki says so.
# Steam ids are each game's base app; executables come from Steam's launch data (Discord's list where Steam has none) and are left out when the name is generic
# or another pack already uses it (that pack then matches by Steam id alone). Genre-based kind and theme were assigned by script and corrected by hand for the
# pointer and keyboard games.
# Left out everywhere: VR games, DLC, demos, tools and soundtracks, games with no release date or not out yet, games delisted from Steam, and games that already have a pack.
S = []
S += [
P("A Memoir Blue", ["1497450"], ["a_memoir_blue.exe"], "coldsteel", "steel blue and grey."),
P("Ashen", ["649950"], ["ashen.exe"], "mystic", "violet and gold."),
P("Bounty Star", ["1497430"], ["bountystar.exe"], "scifi", "cold blue and cyan."),
P("Cocoon", ["1497440"], ["universe.exe"], "calm", "soft pastel tones.", ctl=["dualshock"]),
P("Donut County", ["702670"], ["donutcounty.exe"], "calm", "soft pastel tones.", ctl=["dualshock"]),
P("Florence", ["1102130"], ["florence.exe"], "coldsteel", "steel blue and grey."),
P("Flower", ["966330"], ["flower.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
P("Forever Ago", ["1215940"], ["foreverago.exe"], "coldsteel", "steel blue and grey."),
P("I Am Dead", ["951750"], ["iad.exe"], "calm", "soft pastel tones."),
C("If Found...", ["1041920"], ["iffound.exe"], "paper", "paper cream and ink", exenote=" The Windows executable is from Steam's launch data."),
P("Journey", ["638230"], ["journey.exe"], "coldsteel", "steel blue and grey."),
P("Last Stop", ["1122690"], ["last stop.exe"], "coldsteel", "steel blue and grey."),
P("Lego Voyagers", ["1538550"], ["lego voyagers.exe"], "fighter", "arena red and gold.", ctl=["dualshock"]),
P("Lorelei and the Laser Eyes", ["2008920"], ["lorelei and the laser eyes.exe"], "calm", "soft pastel tones.", ctl=["dualshock"]),
P("Lushfoil Photography Sim", ["1749860"], ["lushfoilsim.exe", "lushfoilsimvr.exe"], "coldsteel", "steel blue and grey."),
P("Maquette", ["762840"], [], "calm", "soft pastel tones."),
P("Mixtape", ["2582320"], ["mixtape.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
P("Morsels", ["1719040"], ["morsels.exe"], "coldsteel", "steel blue and grey."),
PS("Neon White", ["1533420"], ["neon white.exe"], "scifi", "cold blue and cyan."),
P("Open Roads", ["1497360"], ["openroads.exe"], "coldsteel", "steel blue and grey."),
P("Outer Wilds", ["753640"], ["outerwilds.exe"], "calm", "soft pastel tones."),
P("People of Note", ["1626170"], ["peopleofnote.exe"], "mystic", "violet and gold."),
P("Sayonara Wild Hearts", ["1122720"], ["sayonara wild hearts.exe"], "coldsteel", "steel blue and grey."),
PS("Skin Deep", ["301280"], ["skindeep.exe"], "coldsteel", "steel blue and grey."),
P("Solar Ash", ["1867530"], ["solar.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
P("Storyteller", ["1624540"], ["storyteller.exe"], "calm", "soft pastel tones."),
P("Telling Lies", ["762830"], ["tellinglies.exe"], "coldsteel", "steel blue and grey."),
P("The Artful Escape", ["1122680"], ["artfulescape.exe"], "coldsteel", "steel blue and grey."),
P("The Pathless", ["1492680"], ["pathless.exe"], "coldsteel", "steel blue and grey."),
P("The Unfinished Swan", ["1206430"], ["tus.exe"], "calm", "soft pastel tones."),
P("Thirsty Suitors", ["1617220"], ["thirsty suitors.exe"], "coldsteel", "steel blue and grey."),
P("To a T", ["1875320"], ["t.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Twelve Minutes", ["1097200"], ["12m.exe"], "coldsteel", "steel blue and grey."),
P("Wanderstop", ["1299460"], ["clearing.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
P("Wattam", ["702680"], ["wattam.exe"], "coldsteel", "steel blue and grey."),
P("What Remains of Edith Finch", ["501300"], ["finchgame.exe"], "coldsteel", "steel blue and grey."),
P("Wheel World", ["1497460"], ["wheel world.exe"], "fighter", "arena red and gold.", ctl=["dualshock"]),
]
