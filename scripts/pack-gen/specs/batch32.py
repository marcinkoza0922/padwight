from helpers import *
# Games Sony Interactive Entertainment publishes through PlayStation Publishing, not covered by an earlier pack.
# Selection: a live Steam game whose Steam publisher is PlayStation Publishing, PlayStation PC, Firesprite, Housemarque, Insomniac, Haven Studios and the like.
# Concord (shut down) and Sony Pictures VR games are left out.
# P = passthrough, for games with controller support (PS = passthrough plus a gyro profile, for shooters); T = strategy with a pointer;
# C / CW = pointer (and WASD) pack; F = mouse-and-keyboard shooter with a gyro profile; KB = keyboard game (pad support before XInput or none).
# Controller support: Steam's flag (full or partial) when it has one, else the PCGamingWiki snapshot; "dualshock" / "switch_pro" are ticked only where the wiki says so.
# Steam ids are each game's base app; executables come from Steam's launch data (Discord's list where Steam has none) and are left out when the name is generic
# or another pack already uses it (that pack then matches by Steam id alone). Genre-based kind and theme were assigned by script and corrected by hand for the
# pointer and keyboard games.
# Left out everywhere: VR games, DLC, demos, tools and soundtracks, games with no release date or not out yet, games delisted from Steam, and games that already have a pack.
S = []
S += [
P("Days Gone", ["1259420"], ["daysgone.exe"], "horror", "blood red and grey.", ctl=["dualshock"]),
P("Death Stranding 2: On the Beach", ["3280350"], ["ds2.exe", "nip.wingamesteam.releasesym.dx12.exe", "nip.wingamesteam.releasesymrtdbg.dx12.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
P("Everybody's Gone to the Rapture", ["417880"], ["rapture.exe", "rapture_release.exe"], "coldsteel", "steel blue and grey."),
P("Ghost of Tsushima Director's Cut", ["2215430"], ["ghostoftsushima.exe", "ne_pc_steam.exe", "ne_pc_steam_debug.exe"], "samurai", "ink black and crimson.", ctl=["dualshock"]),
P("God of War", ["1593500"], ["gow.exe"], "mystic", "violet and gold.", ctl=["dualshock"]),
P("God of War Ragnar\u00f6k", ["2322010"], ["gowr.exe"], "mystic", "violet and gold.", ctl=["dualshock"], slug="god-of-war-ragnarok"),
P("Guns Up!", ["446150"], ["guns up.exe"], "coldsteel", "steel blue and grey."),
CW("Hamlet", ["673580"], ["hamlet3d.exe"], "noir", "noir grey and amber", exenote=" The Windows executable is from Steam's launch data."),
PS("Helldivers", ["394510"], ["helldivers.exe"], "scifi", "cold blue and cyan.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard.", ctl=["dualshock"]),
PS("Helldivers 2", ["553850"], ["stingray_win64_dev.exe", "stingray_win64_profile.exe", "stingray_win64_release.exe"], "scifi", "cold blue and cyan.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard.", ctl=["dualshock"]),
P("Horizon Forbidden West", ["2420110"], ["horizonforbiddenwest.exe", "nip.wingamesteam.submission.dx12.exe"], "scifi", "cold blue and cyan.", ctl=["dualshock"]),
P("Horizon Zero Dawn", ["1151640"], ["bootloader.exe", "horizonzerodawn.exe"], "scifi", "cold blue and cyan.", ctl=["dualshock"]),
P("Horizon Zero Dawn Remastered", ["2561580"], ["bootloader.win64.releasesymrtdbg.exe", "horizonzerodawnremastered.exe", "nip.wingamesteam.submission.exe"], "scifi", "cold blue and cyan.", ctl=["dualshock"]),
P("Lego Horizon Adventures", ["2428810"], ["glow-win64-shipping.exe", "glow-win64-test.exe", "glow.exe"], "scifi", "cold blue and cyan.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard.", ctl=["dualshock"]),
P("Lost Soul Aside", ["3378960"], ["lostsoulaside.exe"], "mystic", "violet and gold.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard.", ctl=["dualshock"]),
P("Marvel T\u014dkon: Fighting Souls", ["3787240"], ["redsteam.exe"], "fighter", "arena red and gold.", n="The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard.", ctl=["dualshock"], slug="marvel-tokon-fighting-souls"),
P("Marvel's Spider-Man 2", ["2651280"], ["i20_steam_finaldm.exe", "i20_steam_release.exe", "i30_steam_finaldm.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
PS("Midnight Murder Club", ["2698870"], ["midnightmurderclub.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
PS("Nex Machina", ["404540"], ["nex_machina.exe"], "scifi", "cold blue and cyan."),
PS("Ratchet & Clank: Rift Apart", ["1895880"], ["i29_steam_finaldm.exe", "i29_steam_release.exe", "riftapart.exe"], "scifi", "cold blue and cyan.", ctl=["dualshock"]),
PS("Returnal", ["1649240"], [], "scifi", "cold blue and cyan.", ctl=["dualshock"]),
P("Sackboy: A Big Adventure", ["1599660"], [], "nature", "leaf green and earth brown.", ctl=["dualshock"]),
P("Slow Down, Bull", ["333580"], ["slowdownbull.exe"], "coldsteel", "steel blue and grey."),
P("Stellar Blade", ["3489700"], ["sb.exe"], "scifi", "cold blue and cyan.", ctl=["dualshock"]),
PS("The Last of Us Part I", ["1888930"], ["t1x-final-pgo-lto.exe", "t1x-release.exe", "t1x-rtm.exe"], "nature", "leaf green and earth brown.", ctl=["dualshock"]),
PS("The Last of Us Part II Remastered", ["2531310"], ["t2r-steam-final-pgo-lto.exe", "t2r-steam-release.exe", "tlou-ii-l.exe"], "nature", "leaf green and earth brown.", ctl=["dualshock"]),
P("The Persistence", ["1140100"], ["camp.player.exe", "persistence.exe"], "coldsteel", "steel blue and grey."),
P("Uncharted: Legacy of Thieves Collection", ["1659420"], ["u4.exe"], "coldsteel", "steel blue and grey.", ctl=["dualshock"]),
P("Until Dawn", ["2172010"], ["bates.exe"], "horror", "blood red and grey.", ctl=["dualshock"]),
]
