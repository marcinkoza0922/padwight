from helpers import *
# Games Activision published, or that Steam credits to Activision (including the Sierra, Infocom and Vivendi back catalogue Activision now sells), not covered by an earlier pack.
# Selection: the wiki's publisher or developer field names Activision, the game has a live Steam page of type game, and Steam's publisher field names Activision.
# P = passthrough, for games with controller support that came out after XInput (Dec 2005); PS = passthrough plus a gyro profile, for shooters.
# T = strategy with a pointer; C / CW = pointer (and WASD) pack; F = mouse-and-keyboard shooter with a gyro profile.
# ctl= ticks the pad kinds the game supports itself (the wiki's PlayStation column, and DualSense for the current Call of Duty games).
# Left out: games whose Steam page is gone or only an advertising stub (007: Quantum of Solace, 007 Legends, Blood Stone, Blur, Cabela's x5, Deadpool,
# Duck Dynasty, Enemy Territory: Quake Wars, Family Guy, Fast & Furious: Showdown, Ice Age: Arctic Games, Monster Jam, NASCAR The Game 2013, Shrek Forever After,
# Soldier of Fortune: Payback, Spider-Man: Shattered Dimensions, the Amazing Spider-Man games, TMNT: Mutants in Manhattan, The Legend of Korra,
# The Walking Dead: Survival Instinct, Tony Hawk's Pro Skater HD, the four Transformers shooters, Wolfenstein (2009), World Series of Poker, the History
# Channel games, The Movies, Empires: Dawn of the Modern World) and games where Steam credits another publisher (Destiny 2, Day of Defeat, Earthworm Jim, SiN,
# Quake III, Wolfenstein 3D and Enemy Territory, the Jedi Knight games, Lego Star Wars III, White Night, Time Commando, Little Big Adventure 2, Pizza Syndicate,
# Pizza Connection 2, The Manhole, Battle of Britain), plus Call of Duty: Warzone 2.0 and Black Ops 7, which are the same Steam app as Call of Duty HQ.
# Games that share an executable name (Sierra's SierraLauncher.exe, SWAT.exe, marvel.exe) name Steam's install folder in the rule (folder/file).
# bootstrapper.exe, which the new Call of Duty games share, is left out.
# Steam ids are each game's base app; demos, DLC and editions are not rules.
S = []
PART = "The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard."
XI = "The wiki lists controller support but is not sure the game reads XInput: if the pad does nothing in the game, set the controller type in its options or run it through Steam Input."
SOON = "Not out yet: Steam gives no release date and the wiki lists controller support, so this pack assumes the controller support of the earlier games and is ready before the game is."
FOLDER = " The executable rule names Steam's install folder in front of the file name (installdir from Steam's app info), because other games share the file name; it is not checked on an install."
GUESS = "Typical defaults for the genre, not checked against the game."
def T(name, ids, exes, theme, mood, n="", **kw):
    return G(name, ids, exes, "strat", theme, f"The left stick pans, the right stick points; RT and A click, LT right-clicks. Overlays in {mood}.", n=(n + " " if n else "") + GUESS, **kw)
def C(name, ids, exes, theme, mood, n="", **kw):
    return G(name, ids, exes, "click", theme, f"The right stick points; A and RT click, LT right-clicks. Overlays in {mood}.", n=n or "A mouse-driven game; no keys are mapped beyond the mouse.", **kw)
def CW(name, ids, exes, theme, mood, n="", **kw):
    return G(name, ids, exes, "click", theme, f"The left stick walks on WASD and the right stick points or looks; RT and A click, LT right-clicks. Overlays in {mood}.", left="wasd", n=(n + " " if n else "") + GUESS, **kw)
CODN = PART.replace("The wiki or Steam gives only partial or limited controller support: some menus may still want the keyboard.", "")
DS = ["dualshock", "dualsense"]
S += [
PS("Call of Duty: Advanced Warfare", ["209650"], ["s1_sp64_ship.exe", "s1_mp64_ship.exe"], "scifi", "exo-suit steel and orange."),
PS("Call of Duty: Black Ops II", ["202970"], ["t6sp.exe", "t6mp.exe", "t6zm.exe"], "scifi", "near-future grey and orange."),
PS("Call of Duty: Black Ops III", ["311210"], ["blackops3.exe"], "scifi", "cyber-core grey and gold."),
PS("Call of Duty: Black Ops 6", ["4384550"], ["cod-cod24.exe"], "noir", "Gulf War-era brown and red.", ctl=DS),
PS("Call of Duty: Black Ops Cold War", ["1985810"], ["blackopscoldwar.exe"], "coldsteel", "Cold War grey and red.", ctl=DS),
PS("Call of Duty: HQ", ["1938090"], ["cod.exe"], "military", "olive drab and orange.", slug="call-of-duty-hq", ctl=DS,
   n="This is the Steam app for Warzone, Black Ops 7 and the other current Call of Duty releases that share the launcher."),
PS("Call of Duty: Ghosts", ["209160"], ["iw6sp64_ship.exe", "iw6mp64_ship.exe"], "industrial", "ghost grey and orange."),
PS("Call of Duty: Infinite Warfare", ["292730"], ["iw7_ship.exe"], "scifi", "space-war blue and white."),
PS("Call of Duty: Modern Warfare (2019)", ["2000950"], ["modernwarfare.exe"], "military", "olive drab and orange.", slug="call-of-duty-modern-warfare-2019", ctl=DS),
PS("Call of Duty: Modern Warfare II (2022)", ["3595230"], ["cod22-cod.exe", "sp22-cod.exe"], "military", "olive drab and orange.", slug="call-of-duty-modern-warfare-ii-2022", ctl=DS),
PS("Call of Duty: Modern Warfare III (2023)", ["3595270"], ["cod23-cod.exe", "sp23-cod.exe", "mp23-cod.exe"], "military", "olive drab and orange.", slug="call-of-duty-modern-warfare-iii-2023", ctl=DS),
PS("Call of Duty: Modern Warfare 4", ["4435490"], ["cod26-cod.exe", "sp26-cod.exe"], "military", "olive drab and orange.", ctl=DS),
PS("Call of Duty: Modern Warfare Remastered", ["393080"], ["h1_sp64_ship.exe"], "military", "olive drab and orange."),
PS("Call of Duty: Vanguard", ["1985820"], ["vanguard.exe"], "military", "WWII khaki and red.", ctl=DS),
PS("Call of Duty: WWII", ["476600"], ["s2_sp64_ship.exe", "s2_mp64_ship.exe"], "military", "WWII khaki and red."),
PS("Singularity", ["42670"], ["singularity.exe"], "scifi", "Katorga-12 rust and blue."),
F("TimeShift", ["10130"], ["timeshift.exe"], "scifi", "time-warp blue and silver."),
F("SWAT 3: Tactical Game of the Year Edition", ["560370"], ["swat 3 tactical game of the year edition/swat.exe"], "military", "tactical black and grey.", exenote=FOLDER),
P("Crash Bandicoot 4: It's About Time", ["1378990"], ["crashbandicoot4.exe"], "nature", "jungle green and orange.", ctl=["dualshock"]),
P("Crash Bandicoot N. Sane Trilogy", ["731490"], ["crashbandicootnsanetrilogy.exe"], "nature", "jungle green and orange.", ctl=["dualshock"]),
P("Spyro Reignited Trilogy", ["996580"], ["spyro-win64-shipping.exe", "spyro.exe"], "mystic", "dragon purple and gold.", ctl=["dualshock"]),
P("Spyro: A Realm Beyond", ["4245560"], [], "mystic", "dragon purple and gold.", n=SOON),
P("Sekiro: Shadows Die Twice", ["814380"], ["sekiro.exe"], "samurai", "Sengoku ink and crimson.", ctl=DS),
P("Transformers: Devastation", ["338930"], [], "industrial", "Cybertron steel and red."),
P("Tony Hawk's Pro Skater 1 + 2", ["2395210"], ["thps12.exe"], "fighter", "skate-park orange and blue.", n=PART),
P("Tony Hawk's Pro Skater 3 + 4", ["2545710"], ["thps34.exe"], "fighter", "skate-park orange and blue.", n=PART, ctl=["dualshock"]),
P("Prototype", ["10150"], ["prototypef.exe"], "horror", "outbreak red and black.", n=PART),
P("Prototype 2", ["115320"], ["prototype2.exe"], "horror", "outbreak red and black.", n=PART),
P("GUN", ["2610"], ["gun.exe"], "parchment", "Old West dust and tan.", n=PART + " " + XI, slug="gun"),
P("Teenage Mutant Ninja Turtles: Out of the Shadows", ["228560"], ["tmnt-oots.exe"], "nature", "sewer green and orange.", n=PART),
P("Ghostbusters (2016)", ["425310"], ["ghostbusters.exe"], "calm", "proton green and white.", n=PART + " The wiki lists no controller support while Steam lists partial support."),
P("Marvel: Ultimate Alliance (2016)", ["433300"], ["marvel - ultimate alliance/marvel.exe"], "fighter", "comic red and blue.", n=PART, exenote=FOLDER),
P("Marvel: Ultimate Alliance 2", ["433320"], ["alliance.exe"], "fighter", "comic red and blue.", n=PART),
CW("Street Legal 1: REVision", ["1571280"], ["street legal 1 revision/sl_launcher.exe"], "industrial", "street-racing orange and grey",
   b={"South": ("k:SPACE", "Handbrake", None), "LeftBumper": ("k:Q", "Shift down", None), "RightBumper": ("k:E", "Shift up", None)},
   n="Space, Q and E for handbrake and shifting are guesses at the usual keys.", exenote=FOLDER),
CW("Street Legal Racing: Redline v2.3.1", ["497180"], ["streetlegal_redline.exe"], "industrial", "street-racing orange and grey", slug="street-legal-racing-redline",
   b={"South": ("k:SPACE", "Handbrake", None), "LeftBumper": ("k:Q", "Shift down", None), "RightBumper": ("k:E", "Shift up", None)},
   n="Space, Q and E for handbrake and shifting are guesses at the usual keys."),
T("Caesar IV", ["517810"], ["caesar 4/sierralauncher.exe"], "parchment", "Roman marble and gold", b={"North": ("k:SPACE", "Pause", "pause")}, n="Space pausing is a guess.", exenote=FOLDER),
T("Call to Power II", ["572050"], ["ctp2.exe"], "parchment", "empire gold and blue", b={"North": ("k:SPACE", "End turn", "hourglass")}, n="End turn on Space is a guess."),
C("3D Ultra Minigolf Adventures", ["10140"], ["mga.exe"], "nature", "fairway green and blue"),
C("Aces of the Galaxy", ["10120"], ["acesofthegalaxy.exe"], "scifi", "space blue and orange"),
C("Police Quest: SWAT 2", ["607120"], ["police quest - swat 2/swat.exe"], "military", "tactical black and grey", n="Real-time tactical play with the mouse; no keys are mapped beyond the mouse.", exenote=FOLDER),
C("Return to Krondor", ["564960"], ["rtk.exe"], "parchment", "Midkemia stone and gold"),
C("Vampire: The Masquerade - Redemption", ["559680"], ["vampire.exe"], "horror", "blood red and black"),
C("Zork: Grand Inquisitor", ["570680"], ["zengine.exe"], "parchment", "Underground Empire amber and brown"),
C("Betrayal Collection", ["559640"], ["betrayal collection/sierralauncher.exe"], "parchment", "Midkemia stone and gold", exenote=FOLDER),
C("Zork Anthology", ["570580"], ["zork anthology/sierralauncher.exe"], "typewriter", "text-adventure green and black", n="These are text adventures: {guide} + {west} opens the on-screen keyboard for typing commands.", exenote=FOLDER),
C("Gabriel Knight: Sins of the Father", ["495700"], ["gabriel knight/sierralauncher.exe"], "noir", "New Orleans noir and amber", exenote=FOLDER),
C("Gabriel Knight 3: Blood of the Sacred, Blood of the Damned", ["497360"], ["gabriel knight 3/sierralauncher.exe"], "noir", "Rennes-le-Chateau noir and amber", exenote=FOLDER),
C("The Beast Within: A Gabriel Knight Mystery", ["496760"], ["gabriel knight 2/sierralauncher.exe"], "horror", "Bavarian noir and amber", exenote=FOLDER),
C("King's Quest Collection", ["10100"], ["king's quest collection/sierralauncher.exe"], "parchment", "Daventry green and gold", exenote=FOLDER),
C("Phantasmagoria", ["501990"], ["phantasmagoria/sierralauncher.exe"], "horror", "mansion red and black", exenote=FOLDER),
C("Phantasmagoria 2: A Puzzle of Flesh", ["501970"], ["phantasmagoria 2/sierralauncher.exe"], "horror", "mansion red and black", exenote=FOLDER),
C("Police Quest Collection", ["494740"], ["police quest collection/sierralauncher.exe"], "noir", "police blue and grey", exenote=FOLDER),
C("Police Quest: SWAT", ["560350"], ["police quest - swat/sierralauncher.exe"], "military", "tactical black and grey", exenote=FOLDER),
C("Quest for Glory 1-5", ["502750"], ["quest for glory collection/sierralauncher.exe"], "mystic", "heroic violet and gold", exenote=FOLDER),
C("Return to Zork", ["585240"], ["return to zork/sierralauncher.exe"], "parchment", "Underground Empire amber and brown", exenote=FOLDER),
C("Space Quest Collection", ["10110"], ["space quest collection/sierralauncher.exe"], "scifi", "janitor-in-space blue and orange", exenote=FOLDER),
C("Spycraft: The Great Game", ["569220"], ["spycraft - the great game/sierralauncher.exe"], "noir", "Cold War grey and red", exenote=FOLDER),
C("Zork Nemesis: The Forbidden Lands", ["571060"], ["zork nemesis the forbidden lands/sierralauncher.exe"], "horror", "alchemist amber and brown", exenote=FOLDER),
]
