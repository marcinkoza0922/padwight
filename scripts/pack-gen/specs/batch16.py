from helpers import *
# Games Capcom developed or published (the wiki's publisher or developer field), not covered by an earlier pack.
# P = passthrough, for games whose wiki entry says the game has controller support and that came out after XInput (Dec 2005).
# PS = passthrough plus a gyro profile, for shooters (see GYRO_PT_NOTE in engine.py).
# G / KB = generated keyboard and pointer packs, for games with no native pad support, with only hackable XInput, or that predate XInput.
# Left out: games with no Steam id and no executable in the wiki (DOS-era Capcom, the Japanese Sourcenext releases, Mega Man X to X8, Rockman
# 1~6, Street Fighter Alpha, Onimusha: Warlords 2003, Chaos Legion, Capcom's arcade and coin-op compilations of 2003), shut-down or
# delisted online games (Monster Hunter Frontier and Online, Dragon's Dogma Online, Re:Verse, Rockman X Dive, MotoGP 08, Onimusha 3,
# Neopets Puzzle Adventure, MLB Front Office Manager), the Resident Evil 7 teaser demo, and games where the wiki lists Capcom only as the
# Japanese or regional publisher of someone else's game (Grand Theft Auto, Diablo II, StarCraft, Warcraft III, Counter-Strike: Condition
# Zero, Aliens versus Predator 2, Haegemonia, Prince of Qin, Test Drive 5, Robin Hood, Red Shark, Disney Classic Games).
# Steam ids are each game's base app; demos and DLC are not rules.
S = []
PART = "The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."
XI = "The wiki lists controller support but is not sure the game reads XInput: if the pad does nothing in the game, set the controller type in its options or run it through Steam Input."
SOON = "Not out yet: the wiki and Steam give a 2027 date, so this pack is ready before the game is."
S += [
# --- Resident Evil (the 2005-and-earlier releases and the HD remasters already have packs)
PS("Resident Evil 2 (2019)", ["883710"], ["re2.exe"], "typewriter", "Raccoon City typewriter ink."),
PS("Resident Evil 3 (2020)", ["952060"], ["re3.exe"], "typewriter", "Raccoon City typewriter ink."),
PS("Resident Evil 7: Biohazard", ["418370"], ["re7.exe"], "typewriter", "Dulvey typewriter ink."),
PS("Resident Evil Village", ["1196590"], ["re8.exe"], "typewriter", "village typewriter ink."),
PS("Resident Evil Requiem", ["3764200"], ["re9.exe"], "typewriter", "typewriter ink."),
P("Resident Evil Veronica", ["4824610"], [], "typewriter", "typewriter ink.", n=SOON),
PS("Resident Evil: Operation Raccoon City", ["209100"], ["raccooncity.exe"], "typewriter", "Raccoon City typewriter ink."),
PS("Resident Evil Resistance", ["952070"], ["reresistance.exe"], "typewriter", "typewriter ink."),
PS("Umbrella Corps", ["390340"], ["uc.exe"], "military", "Umbrella olive and red."),
# Dino Crisis came out in 2000 and 2002, before XInput: the GOG re-release reads a pad (the wiki: XInput limited or hackable), but the pack sends the keys
G("Dino Crisis", ["4249130"], ["4249130_launcher.exe"], "click", "military", "The pad sends the keyboard keys the game uses: tank controls on the arrows.",
  b={"South": ("k:C", "Action / shoot", "hand"), "East": ("k:V", "Run (hold) / cancel", "fast"), "North": ("k:Z", "Status: items, files", "bag"),
     "LeftBumper": ("k:B", "Quick turn", "rotate"), "RightBumper": ("k:S", "Ready weapon", "crosshair"), "Start": ("k:Q", "Pause", None),
     "DpadUp": ("k:UP", None, None), "DpadDown": ("k:DOWN", None, None), "DpadLeft": ("k:LEFT", None, None), "DpadRight": ("k:RIGHT", None, None)},
  left="arrows", intro="The stick and {dpad} are tank controls: left and right turn, up walks, down backs away. {south} acts and fires, hold {east} to run, {rb} readies the weapon.",
  n="Released in 2000, before XInput, so the pad sends keys. The keys follow the Resident Evil pack for the same engine family (C action, V run and cancel, Z status, B quick turn, S ready weapon, Q pause), not checked in Dino Crisis itself.",
  pad=" The game predates XInput, so the pad sends the keys its controls use.", rt="k:S", rtl="Ready weapon", lt="k:Z", ltl="Status"),
G("Dino Crisis 2", ["4249140"], ["4249140_launcher.exe"], "click", "military", "The pad sends the keyboard keys the game uses: the arrows move.",
  b={"South": ("k:C", "Action / shoot", "hand"), "East": ("k:V", "Run (hold) / cancel", "fast"), "North": ("k:Z", "Status: items, files", "bag"),
     "LeftBumper": ("k:B", "Quick turn", "rotate"), "RightBumper": ("k:S", "Ready weapon", "crosshair"), "Start": ("k:Q", "Pause", None),
     "DpadUp": ("k:UP", None, None), "DpadDown": ("k:DOWN", None, None), "DpadLeft": ("k:LEFT", None, None), "DpadRight": ("k:RIGHT", None, None)},
  left="arrows", intro="The stick and {dpad} move on the arrows. {south} acts and fires, hold {east} to run, {rb} readies the weapon.",
  n="Released in 2002, before XInput, so the pad sends keys. The keys follow the Resident Evil pack for the same engine family (C action, V run and cancel, Z status, B quick turn, S ready weapon, Q pause), not checked in Dino Crisis 2 itself.",
  pad=" The game predates XInput, so the pad sends the keys its controls use.", rt="k:S", rtl="Ready weapon", lt="k:Z", ltl="Status"),
# --- Devil May Cry
# Devil May Cry 3 Special Edition came out in June 2006, but the wiki gives XInput only as hackable: the pad plays the keyboard
G("Devil May Cry 3: Special Edition", ["6550"], ["dmc3se.exe"], "click", "moonlit", "The pad sends the keyboard keys the game uses: the arrows move.",
  b={"South": ("k:SPACE", "Jump", "arrow"), "West": ("k:Z", "Melee", "dagger"), "North": ("k:X", "Gun", "crosshair"), "East": ("k:C", "Style", None),
     "LeftBumper": ("k:LEFTSHIFT", "Lock on", "crosshair"), "RightBumper": ("k:A", "Devil Trigger", None), "Start": ("k:ESC", "Menu", None), "Select": ("k:TAB", "Status", None),
     "DpadUp": ("k:UP", None, None), "DpadDown": ("k:DOWN", None, None), "DpadLeft": ("k:LEFT", None, None), "DpadRight": ("k:RIGHT", None, None)},
  left="arrows", intro="The stick and {dpad} move on the arrows. {south} jumps, {west} swings the melee weapon, {north} fires, {lb} locks on.",
  n="Released in June 2006; the wiki gives XInput as hackable only, so the pad sends keys. The keys are a guess at the keyboard defaults, not checked on an install: open the game's keyboard options and make the buttons here match.",
  pad=" The game reads the pad only through DirectInput, so the pad sends the keys its controls use.", rt="k:LEFTSHIFT", rtl="Lock on", lt="k:A", ltl="Devil Trigger"),
P("Devil May Cry 4", ["45700"], ["devilmaycry4_dx9.exe", "devilmaycry4_dx10.exe"], "moonlit", "crimson and midnight.", n=XI),
P("Devil May Cry 4: Special Edition", ["329050"], ["devilmaycry4specialedition.exe"], "moonlit", "crimson and midnight."),
P("Devil May Cry 5", ["601150"], ["devilmaycry5.exe"], "moonlit", "crimson and midnight."),
P("Devil May Cry HD Collection", ["631510"], ["dmclauncher.exe"], "moonlit", "crimson and midnight."),
P("DmC: Devil May Cry", ["220440"], ["dmc-devilmaycry.exe"], "industrial", "Limbo grey and pink."),
# --- Dead Rising (Dead Rising 2 has its pack)
P("Dead Rising", ["427190"], ["deadrising.exe"], "horror", "mall-zombie yellow and red."),
P("Dead Rising 3", ["265550"], ["deadrising3.exe"], "horror", "mall-zombie yellow and red."),
P("Dead Rising 4", ["543460"], ["deadrising4.exe"], "horror", "mall-zombie yellow and red."),
P("Dead Rising Deluxe Remaster", ["2527390"], ["drdr.exe"], "horror", "mall-zombie yellow and red."),
G("Dead Rising 2: Off the Record", ["45770"], ["deadrising2otr.exe"], "click", "horror", "The left stick walks on WASD and the right stick points; RT and A click, LT right-clicks. Overlays in mall-zombie yellow and red.",
  b={"South": ("k:SPACE", "Jump", "arrow"), "West": ("k:E", "Use", "hand")}, left="wasd",
  n="The wiki gives controller support as hackable only, so this pack plays it with the mouse and keyboard. The keys are typical defaults, not checked on an install."),
# --- Lost Planet and other Capcom shooters
PS("Lost Planet: Extreme Condition", ["6510"], ["lostplanetdx9.exe"], "scifi", "E.D.N. III ice blue.", n=XI),
PS("Lost Planet: Extreme Condition Colonies Edition", ["45720"], ["lpclauncher.exe"], "scifi", "E.D.N. III ice blue.", n=PART),
PS("Lost Planet 2", ["45750"], [], "industrial", "E.D.N. III rust and orange.", n=PART),
PS("Lost Planet 3", ["226720"], ["lostplanetgame.exe"], "industrial", "E.D.N. III rust and orange."),
PS("Dark Void", ["45710"], [], "scifi", "jetpack blue and steel.", n=PART),
P("Dark Void Zero", ["45730"], ["darkvoidzero.exe"], "pixel", "8-bit jetpack blue and steel."),
PS("Bionic Commando (2009)", ["21670"], ["bionic_commando.exe"], "military", "Ascension City olive and orange."),
P("Bionic Commando Rearmed", ["21680"], ["bcr.exe"], "military", "arm-swing olive and orange.", n=PART),
PS("Exoprimal", ["1286320"], ["exoprimal.exe"], "scifi", "dinosaur-hunt blue and orange.", n=XI),
PS("Pragmata", ["3357650"], ["pragmata.exe"], "scifi", "lunar white and blue."),
P("Remember Me", ["228300"], ["rememberme.exe"], "scifi", "Neo-Paris violet and gold."),
# --- Monster Hunter
P("Monster Hunter: World", ["582010"], ["monsterhunterworld.exe"], "parchment", "guild brown and gold."),
P("Monster Hunter Rise", ["1446780"], ["monsterhunterrise.exe"], "samurai", "Kamura red and gold."),
P("Monster Hunter Wilds", ["2246340"], ["monsterhunterwilds.exe"], "parchment", "guild brown and gold.", n=PART),
P("Monster Hunter Stories", ["2356560"], ["mhst.exe"], "nature", "Stories green and gold."),
P("Monster Hunter Stories 2: Wings of Ruin", ["1277400"], ["monster hunter stories 2/game.exe"], "nature", "Stories green and gold."),
P("Monster Hunter Stories 3: Twisted Reflection", ["2852190"], ["monster_hunter_stories_3_twisted_reflection.exe"], "nature", "Stories green and gold.", n=PART),
# --- Dragon's Dogma, Okami, Onimusha, Kunitsu-Gami, Ghost Trick, Clock Tower
P("Dragon's Dogma: Dark Arisen", ["367500"], ["ddda.exe"], "parchment", "Gransys parchment and gold."),
P("Dragon's Dogma 2", ["2054970"], ["dd2.exe"], "parchment", "Vermund parchment and gold."),
P("Okami HD", ["587620"], ["okami.exe"], "samurai", "sumi ink and vermilion.", n="The wiki lists controller support as always on and XInput as limited: some buttons may need the game's own options."),
P("Onimusha: Warlords HD", ["761600"], ["onimusha/game.exe"], "samurai", "oni crimson and gold."),
P("Onimusha 2: Samurai's Destiny", ["3046600"], ["onimusha2.exe"], "samurai", "oni crimson and gold."),
P("Onimusha: Way of the Sword", ["2638890"], ["onimushawots.exe"], "samurai", "oni crimson and gold."),
P("Kunitsu-Gami: Path of the Goddess", ["2510710"], ["kunitsugami.exe"], "samurai", "shrine vermilion and gold."),
P("Ghost Trick: Phantom Detective", ["1967430"], ["ghost trick.exe"], "moonlit", "ghost-world blue and red."),
P("Clock Tower: Rewind", ["2794770"], ["clocktower.exe"], "horror", "mansion crimson and bone."),
# --- Ace Attorney (Phoenix Wright: Ace Attorney Trilogy has its pack)
P("Apollo Justice: Ace Attorney Trilogy", ["2187220"], ["gs456.exe"], "casino", "courtroom green and gold."),
P("Ace Attorney Investigations Collection", ["2401970"], ["gk12.exe"], "casino", "courtroom green and gold."),
P("The Great Ace Attorney Chronicles", ["1158850"], ["tgaac.exe"], "casino", "Meiji-era courtroom green and gold."),
# --- Street Fighter, Marvel vs. Capcom and the arcade collections
P("Street Fighter 6", ["1364780"], ["streetfighter6.exe"], "fighter", "arcade yellow and red."),
P("Street Fighter V", ["310950"], ["streetfighterv.exe"], "fighter", "arcade yellow and red."),
P("Street Fighter IV", ["21660"], ["sf4launcher.exe"], "fighter", "arcade yellow and red.", n=PART),
P("Super Street Fighter IV: Arcade Edition", ["45760"], ["ssfiv.exe"], "fighter", "arcade yellow and red."),
P("Street Fighter X Tekken", ["209120"], ["sftk.exe"], "fighter", "arcade yellow and red."),
P("Street Fighter 30th Anniversary Collection", ["586200"], ["sf30thanniversarycollection.exe"], "fighter", "arcade yellow and red."),
P("Marvel vs. Capcom: Infinite", ["493840"], ["mvci.exe"], "fighter", "arcade yellow and red."),
P("Ultimate Marvel vs. Capcom 3", ["357190"], ["umvc3.exe"], "fighter", "arcade yellow and red."),
P("Marvel vs. Capcom Fighting Collection: Arcade Classics", ["2634890"], ["marvelvscapcomfightingcollection.exe"], "fighter", "arcade yellow and red."),
P("Capcom Fighting Collection", ["1685750"], ["capcomfightingcollection.exe"], "fighter", "arcade yellow and red."),
P("Capcom Fighting Collection 2", ["2400430"], ["capcomfightingcollection2.exe"], "fighter", "arcade yellow and red."),
P("Capcom Arcade Stadium", ["1515950"], ["capcomarcadestadium.exe"], "fighter", "arcade yellow and red."),
P("Capcom Arcade 2nd Stadium", ["1755910"], ["capcomarcade2ndstadium.exe"], "fighter", "arcade yellow and red."),
P("Capcom Beat 'Em Up Bundle", ["885150"], ["cbeub.exe"], "fighter", "arcade yellow and red."),
P("Dungeons & Dragons: Chronicles of Mystara", ["229480"], ["managame.exe"], "parchment", "Mystara parchment and gold."),
P("Ghosts 'n Goblins Resurrection", ["1375400"], ["makaimura_gg_re.exe"], "moonlit", "graveyard blue and crimson."),
P("Strider (2014)", ["235210"], ["striderx64_r.exe"], "scifi", "Kazakh cyber red and steel."),
# --- Mega Man and the retro collections
P("Mega Man 11", ["742300"], ["mega man 11/game.exe"], "scifi", "Mega Man blue and yellow."),
P("Mega Man Dual Override", ["4223290"], [], "scifi", "Mega Man blue and yellow.", n=SOON),
P("Mega Man Legacy Collection", ["363440"], ["suzy/proteus.exe"], "pixel", "Mega Man blue and yellow."),
P("Mega Man Legacy Collection 2", ["495050"], ["mmlc2.exe"], "pixel", "Mega Man blue and yellow."),
P("Mega Man X Legacy Collection", ["743890"], ["rxc1.exe"], "pixel", "Mega Man X blue and silver."),
P("Mega Man X Legacy Collection 2", ["743900"], ["rxc2.exe"], "pixel", "Mega Man X blue and silver."),
P("Mega Man Zero/ZX Legacy Collection", ["999020"], ["mzzxlc.exe"], "pixel", "Zero red and silver."),
P("Mega Man Battle Network Legacy Collection Vol. 1", ["1798010"], ["mmbn_lc1.exe"], "pixel", "NetNavi blue and yellow."),
P("Mega Man Battle Network Legacy Collection Vol. 2", ["1798020"], ["mmbn_lc2.exe"], "pixel", "NetNavi blue and yellow."),
P("Mega Man Star Force Legacy Collection", ["3500390"], ["megaman_starforce_legacy_collection.exe"], "pixel", "Star Force violet and yellow.", n=PART),
P("Mega Man X DiVE Offline", ["2183650"], ["mega_man_x_dive_offline/game.exe"], "scifi", "Mega Man X blue and silver."),
P("DuckTales: Remastered", ["237630"], ["ducktales.exe"], "paper", "Scrooge's money-bin gold and teal."),
P("The Disney Afternoon Collection", ["525040"], ["capcom_disney_afternoon.exe"], "paper", "cartoon blue and yellow."),
# --- Capcom-published games from other studios, and the rest
P("Satazius", ["203990"], ["satazius.exe"], "pixel", "shmup violet and cyan.", n=PART),
G("Flock!", ["21640"], ["flock.exe"], "click", "nature", "The right stick points; RT and A click, LT right-clicks.", n="Steam lists partial controller support, the wiki none."),
G("Age of Booty", ["21600"], [], "click", "nature", "The right stick points; RT and A click, LT right-clicks.", n="A naval strategy game played with the mouse; the wiki and Steam list no controller support."),
]
