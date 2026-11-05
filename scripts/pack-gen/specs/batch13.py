from helpers import *
# Older games sold off Steam (GOG, or no store id the wiki knows) with no controller support.
# Ids are GOG product ids only in the notes: the pack has no Steam id, so store-check reports it as off Steam.
# Executables are not in Discord's list for these titles, so exes stay empty.
S = []
OLD = "Sold on GOG, not Steam, so there is no Steam id and no executable is known."
OLDN = "Sold on GOG, not Steam, so there is no Steam id and no executable is known. This is an old game: the pad plays it through the pointer."
S += [
# --- strategy and city builders (pointer and WASD)
G("Anno 1503", [], [], "strat", "parchment", "Anno 1503: the pointer builds and trades, the stick pans the island. GOG id 1438074791.", n=OLD),
G("Anno 1602: Creation of a New World", [], [], "strat", "parchment", "Anno 1602: the pointer builds and trades, the stick pans the island. GOG id 1438168222.", n=OLD),
G("Caesar II", [], [], "strat", "parchment", "Caesar II: the pointer places buildings and the stick pans the city. GOG id 1386577474.", n=OLD),
G("Caesar", [], [], "strat", "parchment", "Caesar: the pointer places buildings and the stick pans the city. GOG id 1469336312.", n=OLD),
G("Europa Universalis", [], [], "strat", "parchment", "Europa Universalis: the pointer picks provinces and the stick pans the map. GOG id 1423228477.", n=OLD),
G("Panzer General II", [], [], "strat", "military", "Panzer General II: the pointer moves and attacks units on the hex map. GOG id 1207658871.", n=OLD),
G("Emperor: Rise of the Middle Kingdom", [], [], "strat", "parchment", "Emperor: Rise of the Middle Kingdom: the pointer picks provinces and the stick pans the map. GOG id 1598055675.", n=OLD),
G("Dark Reign 2", [], [], "strat", "military", "Dark Reign 2: the pointer selects units and the stick pans the map. GOG id 1207658911.", n=OLD),
G("Battle Isle 2", [], [], "strat", "scifi", "Battle Isle 2: the pointer moves units across the hex map. GOG id 1207661073.", n=OLD),
G("Battle Isle 3", [], [], "strat", "scifi", "Battle Isle 3: the pointer moves units across the hex map. GOG id 1207661103.", n=OLD),
G("Battle Isle: The Andosia War", [], [], "strat", "scifi", "Battle Isle: The Andosia War: the pointer moves units across the hex map. GOG id 1207658954.", n=OLD),
G("Conquests of Camelot: The Search for the Grail", [], [], "strat", "parchment", "Conquests of Camelot: the pointer picks and moves knights and the stick pans the map. GOG id 1516146567.", n=OLD),
G("Cultures 2: The Gates of Asgard", [], [], "strat", "nature", "Cultures 2: the pointer places buildings and the stick pans the settlement. GOG id 1207663813.", n=OLD),
G("Gangsters: Organized Crime", [], [], "strat", "noir", "Gangsters: Organized Crime: the pointer picks gangsters and orders and the stick pans the city. GOG id 1207659108.", n=OLD),
G("Chaos Overlords", [], [], "strat", "parchment", "Chaos Overlords: the pointer picks units and the stick pans the map. GOG id 1207659228.", n=OLD),
G("Pax Imperia: Eminent Domain", [], [], "strat", "scifi", "Pax Imperia: Eminent Domain: the pointer picks ships and planets and the stick pans the map. GOG id 1434378430.", n=OLD),
G("Imperialism II: The Age of Exploration", [], [], "strat", "parchment", "Imperialism II: the pointer picks provinces and the stick pans the map. GOG id 1207659153.", n=OLD),
# --- adventure, puzzle and RPG (pointer)
G("Alex Builds His Farm", [], [], "click", "calm", "Alex Builds His Farm: the pointer places and works the farm. Educational and simple.", n=OLDN),
G("The Pink Panther's Passport to Peril", [], [], "click", "casino", "The Pink Panther's Passport to Peril: the pointer clicks scenes and objects.", n=OLDN),
G("The Pink Panther in Hokus Pokus Pink", [], [], "click", "casino", "The Pink Panther in Hokus Pokus Pink: the pointer clicks scenes and objects.", n=OLDN),
G("The Walt Disney World Explorer", [], [], "click", "calm", "The Walt Disney World Explorer: the pointer clicks around the park and its exhibits. Educational.", n=OLDN),
G("Freddy Pharkas: Frontier Pharmacist", [], [], "click", "noir", "Freddy Pharkas: the pointer walks Freddy and clicks objects and people. Sierra's point-and-click adventure.", n=OLDN),
G("Akalabeth: World of Doom", [], [], "click", "pixel", "Akalabeth: the pointer and the stick move the party through the dungeon. GOG id 1207666073.", n=OLDN, left="wasd"),
G("Death Gate", [], [], "click", "mystic", "Death Gate: the pointer clicks the first-person view and the items, and the stick turns. GOG id 1869199616.", n=OLDN, left="wasd"),
G("Crusader: No Remorse", [], [], "click", "scifi", "Crusader: No Remorse: the pointer aims and fires, the stick moves. GOG id 1207658933.", n=OLDN, left="wasd"),
G("Crusader: No Regret", [], [], "click", "scifi", "Crusader: No Regret: the pointer aims and fires, the stick moves. GOG id 1207658982.", n=OLDN, left="wasd"),
G("Chessmaster 9000", [], [], "click", "calm", "Chessmaster 9000: the pointer moves the pieces. GOG id 1207658756.", n=OLDN),
G("Dr. Brain: Action Reaction", [], [], "click", "pixel", "Dr. Brain: Action Reaction: the pointer clicks the puzzles and the arcade stages. Wiki says hackable.", n=OLDN),
]
