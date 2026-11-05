from helpers import *
# Games 505 Games publishes, not covered by an earlier pack.
# Selection: a live Steam game whose Steam publisher is 505 Games (wiki names it too), no pack by Steam id or name.
# P = passthrough, for games with controller support (PS = passthrough plus a gyro profile, for shooters); T = strategy with a pointer;
# C / CW = pointer (and WASD) pack; F = mouse-and-keyboard shooter with a gyro profile; KB = keyboard game (pad support before XInput or none).
# Controller support: Steam's flag (full or partial) when it has one, else the PCGamingWiki snapshot; "dualshock" / "switch_pro" are ticked only where the wiki says so.
# Steam ids are each game's base app; executables come from Steam's launch data (Discord's list where Steam has none) and are left out when the name is generic
# or another pack already uses it (that pack then matches by Steam id alone). Genre-based kind and theme were assigned by script and corrected by hand for the
# pointer and keyboard games.
# Left out everywhere: VR games, DLC, demos, tools and soundtracks, games with no release date or not out yet, games delisted from Steam, and games that already have a pack.
S = []
S += [
P("Abz\u00fb", ["384190"], ["abzugame.exe"], "coldsteel", "steel blue and grey.", slug="abzu"),
P("ADR1FT", ["300060"], ["adr1ft.exe"], "coldsteel", "steel blue and grey."),
P("Ashes Cricket 2013", ["265020"], [], "fighter", "arena red and gold."),
P("Assetto Corsa Competizione", ["805550"], [], "fighter", "arena red and gold.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Assetto Corsa EVO", ["3058630"], ["assettocorsaevo.exe"], "fighter", "arena red and gold.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Assetto Corsa Rally", ["3917090"], ["acr.exe"], "fighter", "arena red and gold."),
T("Battle Islands", ["305260"], ["battleislands.exe"], "military", "olive drab and khaki", exenote=" The Windows executable is from Steam's launch data."),
P("Battle Islands: Commanders", ["445720"], ["commanders.exe"], "coldsteel", "steel blue and grey."),
P("Blades of Fire", ["2091020"], ["bladesoffire.exe"], "mystic", "violet and gold."),
P("Bloodstained: Ritual of the Night", ["692850"], ["bloodstainedrotn.exe"], "mystic", "violet and gold."),
P("Brothers: A Tale of Two Sons", ["225080"], [], "calm", "soft pastel tones."),
P("Brothers: A Tale of Two Sons Remake", ["2153350"], ["brothers - a tale of two sons - remake.exe"], "coldsteel", "steel blue and grey."),
PS("Crime Boss: Rockay City", ["2933080"], ["crimeboss.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard.", ctl=["dualshock"]),
P("Death Stranding", ["1190460"], [], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
P("Death Stranding Director's Cut", ["1850570"], [], "fighter", "arena red and gold.", ctl=["dualshock"]),
P("Drift21", ["520950"], ["drift21.exe"], "fighter", "arena red and gold."),
P("Eiyuden Chronicle: Hundred Heroes", ["1658280"], ["eiyudenchronicle.exe"], "mystic", "violet and gold.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Eiyuden Chronicle: Rising", ["1658290"], ["eiyuden chronicle rising.exe"], "mystic", "violet and gold."),
T("Ember", ["339580"], ["ember.exe"], "mystic", "violet and gold", exenote=" The Windows executable is from Steam's launch data."),
P("Ghostrunner", ["1139900"], ["ghostrunner.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard.", ctl=["dualshock"]),
P("Ghostrunner 2", ["2144740"], ["ghostrunner2.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard.", ctl=["dualshock"]),
P("Grow: Song of the Evertree", ["1380420"], ["grow.exe"], "coldsteel", "steel blue and grey."),
F("Hawken Reborn", ["705040"], ["hawken2.exe"], "scifi", "cold blue and cyan", exenote=" The Windows executable is from Steam's launch data."),
P("Horace", ["629090"], ["horace.exe"], "coldsteel", "steel blue and grey."),
P("How to Survive", ["250400"], [], "coldsteel", "steel blue and grey."),
PS("How to Survive 2", ["360170"], ["howtosurvive2.exe"], "coldsteel", "steel blue and grey."),
P("How to Survive: Third Person Standalone", ["360150"], [], "coldsteel", "steel blue and grey."),
P("Indivisible", ["421170"], ["indivisible.exe"], "mystic", "violet and gold."),
P("Journey to the Savage Planet", ["973810"], ["towers.exe"], "coldsteel", "steel blue and grey."),
P("Laser League", ["570460"], ["laserleague.exe"], "coldsteel", "steel blue and grey."),
P("Last Day of June", ["635320"], ["lastdayofjune.exe"], "coldsteel", "steel blue and grey."),
P("Marlow Briggs and the Mask of Death", ["249680"], ["kick_win.exe"], "coldsteel", "steel blue and grey."),
PS("Memories of Mars", ["644290"], [], "scifi", "cold blue and cyan."),
P("Miasma Chronicles", ["1649010"], ["miasma.exe"], "mystic", "violet and gold."),
P("Nivalis Nights", ["1488490"], ["nivalis nights.exe"], "pixel", "bright pixel-art colors."),
T("Objects in Space", ["824070"], ["ois.exe"], "scifi", "cold blue and cyan", exenote=" The Windows executable is from Steam's launch data."),
P("Open Country", ["1192010"], ["outdooradventures.exe"], "coldsteel", "steel blue and grey."),
P("Portal Knights", ["374040"], ["portal_knights_x64.exe"], "scifi", "cold blue and cyan.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard.", ctl=["dualshock"]),
T("Pro Rugby Manager 2015", ["273030"], [], "nature", "leaf green and earth brown"),
P("Puzzle Quest 3", ["1380410"], ["puzzlequest3.exe"], "mystic", "violet and gold.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
P("Puzzle Quest: Immortal Edition", ["3236710"], ["puzzlequestimmortal.exe"], "mystic", "violet and gold."),
CW("Quarantine", ["489370"], ["quarantine.exe"], "scifi", "cold blue and cyan", exenote=" The Windows executable is from Steam's launch data."),
P("Re:Legend", ["823950"], ["milestone14.3.exe", "relegend.exe"], "coldsteel", "steel blue and grey.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."),
CW("Red Solstice 2: Survivors", ["768520"], ["survivors.exe"], "scifi", "cold blue and cyan", exenote=" The Windows executable is from Steam's launch data."),
F("Rekoil", ["243320"], ["rekoil.exe", "rekoilshop.exe"], "military", "olive drab and khaki", exenote=" The Windows executable is from Steam's launch data."),
P("Rescue Party: Live!", ["1662570"], ["rescuepartylive.exe"], "coldsteel", "steel blue and grey."),
P("Serial Cleaners", ["1337920"], ["serialcleaners.exe"], "coldsteel", "steel blue and grey."),
P("Stray Blade", ["1621990"], ["stray blade.exe"], "mystic", "violet and gold."),
P("Takedown: Red Sabre", ["236510"], ["takedowngame-win32-shipping.exe", "takedowngame-win64-shipping.exe"], "coldsteel", "steel blue and grey."),
P("The Guest", ["402040"], ["thegueststeam_test.exe"], "calm", "soft pastel tones."),
CW("Total Tank Simulator", ["674500"], ["totaltanksim.exe"], "military", "olive drab and khaki", exenote=" The Windows executable is from Steam's launch data."),
P("Underworld Ascendant", ["692840"], ["ua.exe"], "mystic", "violet and gold."),
P("Unholy", ["817020"], ["unholy.exe"], "horror", "blood red and grey."),
P("Virginia", ["374030"], ["virginia.exe"], "coldsteel", "steel blue and grey."),
P("Wuchang: Fallen Feathers", ["2277560"], ["project_plague.exe"], "mystic", "violet and gold.", ctl=["dualshock"]),
]
