from helpers import *
# Nihon Falcom and Gust developed games with a Windows release (wiki publisher or developer: Nihon Falcom, Gust Co. Ltd.).
# P = passthrough, for games whose wiki entry says the game has controller support (see PT_NOTE).
# G = generated pointer or keyboard pack, for games with no native controller support, or that predate XInput (Dec 2005).
# Games with no Steam id and no executable in the wiki (DOS and early Windows releases, the 2013 Zero no Kiseki) are left out
# until their ids are known; see the batch14 note in docs/development/pack-qa.tsv.
S = []
S += [
# --- Atelier (Gust), calm alchemy theme
P("Atelier Ayesha: The Alchemist of Dusk DX", ["1152300"], [], "calm", "alchemy lab green and gold.", n="Partial support in the wiki: some menus may still want the keyboard."),
P("Atelier Escha & Logy: Alchemists of the Dusk Sky DX", ["1152310"], [], "calm", "alchemy lab green and gold."),
P("Atelier Firis: The Alchemist and the Mysterious Journey", ["527290"], [], "calm", "alchemy lab green and gold."),
P("Atelier Firis: The Alchemist and the Mysterious Journey DX", ["1502980"], [], "calm", "alchemy lab green and gold."),
P("Atelier Karia: The Night Kingdom & the Guide of Memories", ["4532500"], [], "mystic", "night-kingdom violet and gold.", n="Not out yet: the wiki gives a February 2027 date, so this pack is ready before the game is."),
P("Atelier Lulua: The Scion of Arland", ["1045620"], [], "calm", "alchemy lab green and gold."),
P("Atelier Lydie & Suelle: The Alchemists and the Mysterious Paintings", ["756590"], [], "calm", "alchemy lab green and gold."),
P("Atelier Lydie & Suelle: The Alchemists and the Mysterious Paintings DX", ["1502990"], [], "calm", "alchemy lab green and gold."),
P("Atelier Marie Remake: The Alchemist of Salburg", ["2138090"], [], "calm", "alchemy lab green and gold."),
P("Atelier Meruru: The Apprentice of Arland DX", ["936190"], [], "calm", "alchemy lab green and gold.", n="Partial support in the wiki: some menus may still want the keyboard."),
P("Atelier Resleriana: The Red Alchemist & the White Guardian", ["3259600"], [], "mystic", "red and white guardian violet and gold."),
P("Atelier Rorona: The Alchemist of Arland DX", ["936160"], [], "calm", "alchemy lab green and gold.", n="Partial support in the wiki: some menus may still want the keyboard."),
P("Atelier Ryza: Ever Darkness & the Secret Hideout", ["1121560"], ["atelier ryza/atelier_ryza.exe"], "nature", "island green and sun gold."),
P("Atelier Ryza: Ever Darkness & the Secret Hideout DX", ["3365030"], [], "nature", "island green and sun gold."),
P("Atelier Ryza 2: Lost Legends & the Secret Fairy", ["1257290"], ["atelier ryza 2/atelier_ryza_2.exe"], "nature", "island green and sun gold.", n="Partial support in the wiki: some menus may still want the keyboard."),
P("Atelier Ryza 2: Lost Legends & the Secret Fairy DX", ["3365040"], [], "nature", "island green and sun gold."),
P("Atelier Ryza 3: Alchemist of the End & the Secret Key", ["1999770"], ["atelier ryza 3/atelier_ryza_3.exe"], "nature", "island green and sun gold."),
P("Atelier Ryza 3: Alchemist of the End & the Secret Key DX", ["3365050"], [], "nature", "island green and sun gold."),
P("Atelier Shallie: Alchemists of the Dusk Sea DX", ["1152320"], [], "calm", "alchemy lab green and gold.", n="Partial support in the wiki: some menus may still want the keyboard."),
P("Atelier Sophie: The Alchemist of the Mysterious Book", ["527270"], [], "calm", "alchemy lab green and gold.", n="Partial support in the wiki: some menus may still want the keyboard."),
P("Atelier Sophie: The Alchemist of the Mysterious Book DX", ["1502970"], [], "calm", "alchemy lab green and gold."),
P("Atelier Sophie 2: The Alchemist of the Mysterious Dream", ["1621310"], [], "calm", "alchemy lab green and gold."),
P("Atelier Totori: The Adventurer of Arland DX", ["936180"], [], "calm", "alchemy lab green and gold.", n="Partial support in the wiki: some menus may still want the keyboard."),
P("Atelier Yumia: The Alchemist of Memories & the Envisioned Land", ["3123410"], [], "calm", "alchemy lab green and gold."),
P("Nelke & the Legendary Alchemists: Ateliers of the New World", ["886820"], [], "calm", "alchemy lab green and gold."),
# Atelier Resleriana: Forgotten Alchemy has no controller support, so the keyboard keys are the pad's
G("Atelier Resleriana: Forgotten Alchemy and the Polar Night Liberator", ["2594920"], ["atelierresleriana/atelierresleriana.exe", "atelierreslerianagl/atelierresleriana.exe"], "click", "mystic",
  "The pad sends the keyboard keys the game uses: arrows walk and the face buttons act.", b={"South": ("k:Z", "Confirm", None), "East": ("k:X", "Cancel", None), "West": ("k:C", "Action", None), "DpadUp": None, "DpadDown": None},
  left="arrows", intro="The arrow keys move. {south} confirms, {east} cancels, {west} acts, and {start} opens the menu.",
  n="The game has no controller support, so the pad sends keys. The keys are the usual defaults for the series, not checked on an install.",
  pad=" The game has no controller support, so the pad sends the keys its controls use."),
# --- Ar Nosurge / Ciel Nosurge / Blue Reflection (Gust), sci-fi and mystic
P("Ar Nosurge: Ode to an Unborn Star DX", ["1477490"], [], "scifi", "starship blue and white."),
P("Ciel Nosurge: Requiem for a Lost Star DX", ["1477480"], [], "scifi", "starship blue and white."),
P("Blue Reflection", ["658260"], [], "mystic", "magical girl pastel and violet."),
P("Blue Reflection: Second Light", ["1423600"], [], "mystic", "magical girl pastel and violet."),
P("Blue Reflection Quartet", ["3825390"], [], "mystic", "magical girl pastel and violet."),
# --- Gust action and other
P("Fairy Tail", ["1233260"], ["fairy tail/fairy_tail.exe"], "parchment", "guild red and gold."),
P("Fairy Tail 2", ["3002850"], [], "parchment", "guild red and gold."),
P("Nights of Azure", ["527280"], [], "horror", "gothic crimson and bone.", n="Partial support in the wiki: some menus may still want the keyboard."),
P("Nights of Azure 2: Bride of the New Moon", ["636570"], [], "horror", "gothic crimson and bone.", n="Partial support in the wiki: some menus may still want the keyboard."),
# --- Falcom: Trails series (Legend of Heroes)
P("The Legend of Heroes: Trails in the Sky SC", ["251290"], ["ed6_win2.exe"], "parchment", "Erebonia grey and blue, the Bracer's guild."),
P("The Legend of Heroes: Trails in the Sky the 3rd", ["436670"], ["trails in the sky the 3rd/ed6_win3_dx9.exe"], "parchment", "Erebonia grey and blue.", n="Partial support in the wiki: some menus may still want the keyboard."),
P("Trails in the Sky 1st Chapter", ["3375780"], ["trails in the sky 1st chapter/sora_1st.exe"], "parchment", "Liberl blue and gold."),
P("Trails in the Sky 2nd Chapter", ["4225980"], [], "parchment", "Liberl blue and gold."),
P("The Legend of Heroes: Trails of Cold Steel", ["538680"], ["trails of cold steel/ed8.exe"], "military", "Erebonia military grey and red."),
P("The Legend of Heroes: Trails of Cold Steel II", ["748490"], [], "military", "Erebonia military grey and red."),
P("The Legend of Heroes: Trails of Cold Steel III", ["991270"], [], "military", "Erebonia military grey and red."),
P("The Legend of Heroes: Trails of Cold Steel IV", ["1198090"], [], "military", "Erebonia military grey and red."),
P("The Legend of Heroes: Trails through Daybreak", ["2138610"], ["the legend of heroes trails through daybreak/kuro.exe"], "parchment", "Crossbell grey and gold."),
P("The Legend of Heroes: Trails through Daybreak II", ["2113920"], [], "parchment", "Crossbell grey and gold."),
P("The Legend of Heroes: Trails from Zero", ["1668510"], ["the legend of heroes trails from zero/zero.exe"], "parchment", "Crossbell grey and gold."),
P("The Legend of Heroes: Trails to Azure", ["1668520"], [], "parchment", "Crossbell grey and gold."),
P("The Legend of Heroes: Trails into Reverie", ["1668540"], [], "parchment", "Crossbell grey and gold."),
P("The Legend of Heroes: Trails Beyond the Horizon", ["3316940"], [], "parchment", "Crossbell grey and gold."),
P("The Legend of Nayuta: Boundless Trails", ["1668530"], [], "nature", "forest green and gold."),
P("Ys vs. Trails in the Sky: Alternative Saga", ["1794880"], [], "parchment", "Ys and Trails crossover, red and gold."),
# --- Falcom: Ys series
P("Ys I & II Chronicles+", ["223810", "223870"], ["ys i/ys1plus.exe"], "mystic", "Adol red and gold, Ys ancient gold."),
P("Ys: Memories of Celceta", ["587110"], [], "mystic", "Ys ancient gold and red."),
P("Ys Origin", ["207350"], ["ys origin/yso_win.exe"], "mystic", "Ys ancient gold and red.", n="Partial support in the wiki: some menus may still want the keyboard."),
P("Ys Seven", ["587100"], [], "mystic", "Ys ancient gold and red.", n="Partial support in the wiki: some menus may still want the keyboard."),
P("Ys VIII: Lacrimosa of DANA", ["579180"], ["ys viii lacrimosa of dana/ys8.exe"], "mystic", "Ys ancient gold and red."),
P("Ys IX: Monstrum Nox", ["1351630"], [], "horror", "Ys dark red and bone."),
P("Ys X: Nordics", ["2731870"], [], "mystic", "Ys ancient gold and red."),
P("Ys X: Proud Nordics", ["3949290"], [], "mystic", "Ys ancient gold and red."),
# --- Falcom: Zwei and Tokyo Xanadu
P("Zwei: The Arges Adventure", ["427680"], [], "military", "Zwei battle grey and gold.", n="Partial support in the wiki: some menus may still want the keyboard."),
P("Zwei: The Ilvard Insurrection", ["427700"], [], "military", "Zwei battle grey and gold."),
P("Tokyo Xanadu eX+", ["587260"], ["tokyo xanadu ex+/tokyoxanadu.exe"], "scifi", "Tokyo night neon blue and pink."),
# --- Falcom: older Windows games with no native controller support (pre-XInput or none)
KB("The Legend of Heroes: Trails in the Sky", ["251150"], ["ed6_win.exe", "ed6_win_dx9.exe"], "parchment", "Liberl blue and gold.", n="Released 2004, before XInput, so the pad sends keys."),
KB("Ys VI: The Ark of Napishtim", ["312540"], [], "parchment", "Ys ancient gold and red.", n="Released 2003, before XInput, so the pad sends keys."),
KB("Ys: The Oath in Felghana", ["207320"], ["ys the oath in felghana/ysf_win_dx9.exe"], "parchment", "Ys ancient gold and red.", n="Released 2005, before XInput, so the pad sends keys."),
KB("Gurumin: A Monstrous Adventure", ["322290"], [], "pixel", "pixel-art gold and blue.", n="Released 2004, before XInput, so the pad sends keys."),
KB("Xanadu Next", ["312560"], [], "scifi", "Xanadu blue and gold.", n="Released 2005, before XInput, so the pad sends keys."),
]
