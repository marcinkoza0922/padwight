# Game setup guidelines

Conventions for choosing where actions go when setting up a game's controller bindings, for pack authors and the built-in library. They're defaults, not rules: a game's own conventions or a clearer layout win.

## Look the game up first

Before writing or changing a pack, check the local references in `pcgamingwiki/` (not tracked; its `README.md` has the lookups):

- **`games.tsv` and the article in `pages/`**: Steam app ids, release date, engine, and the Input section's native controller facts (XInput, DualShock/DualSense, Nintendo, Steam Input), which the rules below depend on.
- **`executables.tsv`**: executable names from Discord's game-detection list, joined to the wiki by Steam app id. Names are lowercased; executable rules match without case, so any case works. Use the file name only, not a leading folder such as `control/`, unless the name is one other games share (an engine's or runtime's generic `nw.exe`, `game.exe`, `xrEngine.exe`): then keep the game's install folder in front of it (`game dev tycoon/nw.exe`), so the rule picks out this game only.

These are community data, so they're a starting point, not proof. Say in the commit what came from them and what was checked on an install.

## Games with native controller support

- If the game is known to have **native XInput support**, even if it's imperfect, **don't change or add to its control scheme**. This program can't do better than a native scheme, so leave the bindings alone. Shooters are the exception: they still get gyro and flick stick profiles (see [Shooters](#shooters)).
- **Games released before 2006 don't count as native XInput**, even if the store page lists controller support. XInput came out in December 2005, so an older game has at best spotty DirectInput support, which is unreliable. Map the gamepad to the keyboard and mouse bindings, as for a game with no controller support. Treat a store page's controller badge as a hint only, and check the release date first.
- **Check for native DualShock and Nintendo support too.** A game may support a DualShock 4, DualSense or Switch Pro Controller without XInput, or alongside it. Tick the matching kind under **Controllers this setup supports** on the Details tab, so the game sees that controller as itself and its own scheme holds for it. The same rules apply: explicit support counts, a store badge alone doesn't, and games released before 2006 don't count. The [first setup tutorial](../manual/tutorial-first-setup.md#3-check-for-native-controller-support) walks through the setting.
- Enhancements are still welcome where the game leaves a gap:
  - **Onscreen keyboard** for text input, such as naming a character, where the developer didn't plan for a controller.
  - **Mouse mode** for setup menus that only work with the mouse.
  - **Media controls**, so the player can listen to music or podcasts while playing, for example a roguelite card game.

## Games with a console release

- **If the game came out on a console too, model the scheme on the console version's controls.** Players who know the game know that layout, and the console team already fitted the game to a pad. Look up the console manual or a controls page for it (the Xbox version first, since its layout is closest to the pad Padwight presents), then map each console button to the PC key that does the same thing.
- **The console layout wins over the defaults in this guide** where they differ. The Prince of Persia packs put the special action and block on RT and the time powers on LT, as the Xbox releases did, rather than following the right-mouse-button-on-LT rule.
- **Fill the gaps with this guide.** A PC version often has keys the console didn't (quick save, a landscape camera, a console-only feature dropped), and those go where the rest of this guide puts them.
- **Some console versions can't be copied.** A console edition with its own rebuilt interface (RimWorld Console Edition) has controls that don't exist on PC; borrow its ideas, such as its menus on the shoulder buttons and triggers, but build the scheme on the PC game's keys.

## Games with more than one version

Classic PC games often come in more than one version: the original, a modern remaster or re-release, and sometimes a port. Often only one of them has controller support. Each version gets its own pack, and the two packs are handled differently:

- **A version with native controller support is a gamepad passthrough.** Every button, stick and trigger goes to the game as it is, the way the [Against the Storm pack](../../packs/against-the-storm.padpack) does it (`{gamepad = "South"}` and so on for each input). The Guide overlay still sits on top, and the notes say to set the buttons in the game's own options. Don't add bindings, shortcuts, layers or modes: they would only fight the game's scheme. The exception is a shooter, which gets gyro and flick stick profiles beside the passthrough (see [Shooters](#shooters)).
- **Keep the menus the same across versions.** Both packs should look and sound like the same game. Use the same overlay font, colors, keyboard and numpad styles, icons and sound cues in each, so a player who switches versions sees no change in the overlays.
- **A version without controller support follows the rest of this guide.** Build its scheme as usual, with the same presentation as the other version.

## Movement and camera

- **WASD movement** should generally be assigned to the **left stick**. Strategy games that use WASD or the arrow keys to pan the camera can also assign that to the left stick.
- **Camera or cursor control** should almost always be assigned to the **right stick**. As a rule of thumb, if the mouse controls it, it goes on the right stick.
- **Walk or slow movement** should be bound to the **left stick** as a zone covering the first **50 to 70 percent** of the stick's travel, so a light push walks and a full push runs. The right value depends on the controller and game. The Deus Ex pack uses 60 percent (see [the Deus Ex tutorial](../manual/tutorial-deus-ex.md)), which works well enough as a starting point.
- **Click-to-move games** (a pointer on screen, and the character walks to where the left mouse button is clicked, as in Diablo-style action RPGs) put movement on the **left stick** as a **mouse region** with a **zone that holds the left mouse button from the first push** (`min = 0.0`, `max = 1.0`; a zone starting at 0 presses as soon as the stick moves). Center the region on the character, which is usually the middle of the screen, and make it a circle about a third of the screen tall, so a full push lands a short walk away in that direction. Size its width for the screen's shape (`width = 0.34, height = 0.6` is round on 16:9), and don't make it so big that a full push clicks on the interface at the screen's edges. The character then walks, and attacks or picks up what the pointer lands on, while the stick is pushed, and stops when it's let go. A mouse region puts the pointer back in its middle on release, which is over the character, so nothing else gets clicked. This needs a camera that keeps the character in the middle; a party game whose camera scrolls freely over the map (Baldur's Gate, Fallout) has no fixed spot to put the region, so its left stick pans the camera instead.
- **Click-to-move games still need a normal pointer.** The region only reaches its own circle, so put an ordinary **mouse pointer** on the **right stick** for the inventory, the menus and aiming skills, with **A** as left click. The [Torchlight II pack](../../packs/torchlight-ii.padpack) does both.
- **Deadzone shape** stays on circle except for a ring that should only walk in straight lines, where cross keeps the sideways part clean. Use **Rotate output** and **Invert X** only for a game whose own stick is oriented differently.
- **Sprint** should be bound to a **long press of A** with a short long-press window.

## Actions

- **Primary fire** should almost always be the **right trigger**.
- **Secondary fire** depends on the game. It can be the **left trigger**, or the **right bumper**.
- **Melee** depends on the game. **Right stick click** is generally acceptable, as is one of the **face buttons**.
- **Use** (bound to E, F, or sometimes Space) should be assigned to **A** or **X**.
- **Reload** should typically be assigned to **X** or **Y**. **Right Bumper** can also work.
- **Jump** should be assigned to **A** or **B**. If the game has a *Use* action, Use goes on **A** and Jump goes on **B**.
- **Touchpad** actions should generally be mouse-style. Not every controller has a touchpad, so don't rely on it. Even on a DualShock it's a niche tool, so don't build the scheme around it.
- **Crouch** should almost always be a **toggle**. It should favour **left stick click**. If **B** is free, it may be preferred there instead.
- **Zoom / aim**, if not on the right mouse button, should be on **right stick press** or **D-pad Up**.
- **Mirrored** actions, such as switching to the next or previous item, or rotating the camera, should typically be on **D-pad Left/Right** or the **shoulder buttons**.
- **Lean** can go on **D-pad Left/Right**. An acceptable alternative is a layer on a face button or shoulder button, where the **left stick** acts as the lean while the layer is held.
- **Scroll** (mouse wheel) can go on the **D-pad** or the **shoulder buttons**.
- **Related actions** should share the same button and use different gestures: a multi tap or a long press. For example, if pick up is a long press of **A**, open is a short press of **A**.
- **Changing ammo type** should share the button with **Reload**, as a long press of it.

## Shooters

- **Response curves** for shooters should loosely resemble those found in console shooters. Match the general shape and feel: a radial dead zone, a power curve on the right stick, and sensitivity in the same ballpark as a console game.
- **Aim assist** can't be used with this daemon, so don't try to reproduce it. Almost everything else from the console feel can be matched: the dead zone, the curve shape, per-axis sensitivity, and invert-Y.
- **Starting values** for a shooter's right stick: a small radial deadzone just above the pad's drift, a power curve near 2, and an anti-deadzone matched to the game's own deadzone only when the game reads the gamepad. Don't add an anti-deadzone to a mouse-mode stick, which has no deadzone of its own to beat.
- **Gyro in a gamepad-only game** should use the turn-speed stick mode on the right stick, with **Game's curve** set to the game's stick response exponent so the controller turns the view as far as it turns, like a mouse would. Set the anti-deadzone to the game's deadzone. Note in the pack which values were assumed.
- **Gyro in a mouse game** should use real-world sensitivity (1× to start) with the game's full turn entered, so the same setting feels the same in every pack.
- **Zoom / aim down sights** should be a **layer on the same button that aims** (the left trigger, or whatever the game's aim is on), held over the right stick, that swaps in a more precise **response curve**: a lower or steeper power curve, so small deflections move the view less while full deflection still turns as fast. It's released as soon as the aim button is let go. **If the aim input is a toggle, skip the layer:** layers can't be toggled, so a layer on a toggled aim button would hold only while the button is physically down, not while the player is aiming. Prefer changing the curve to the sensitivity: a player who adjusts the base sensitivity in quick settings would otherwise find the aim jumping when they aim down sights, because the layer's sensitivity no longer follows theirs. Quick Settings lists a row for each layer and mode that sets its own stick speed or gyro sensitivity, so the player can tune that one too.
- **Hold or toggle for aim** depends on the game. If the game uses a toggle, make it a toggle. If the game uses a hold, make it a hold when it's on the **left trigger**, and a toggle when it's on any other input.
- **Every gyro profile needs a clutch and a recenter.** The clutch turns gyro off while held, so the player can reposition the controller without moving the aim or fighting jitter at rest. The recenter sets the current tilt as straight ahead and clears drift. Neither should take a button from the base layout, since most games already use the whole controller. Put them on **combos or gestures** instead (for example, a chord of two shoulder buttons for the clutch, a long press of a stick click for the recenter), where they can't conflict with the scheme. Add the **Gyro on / off** action the same way, and optionally a menu item for it. Back paddles also work for these on controllers that have them, but they can't be relied on, so the combo or gesture is the one the scheme counts on, for players who want gyro switched on from a menu.
- **Gyro and flick stick profiles** should always be added for shooters, so the player can try them, even when the game has gamepad support and the rest of the scheme is passthrough. They should **never be the default**. The default is the game's own scheme.
- **If the game is known to support mouse and gamepad at the same time**, map **gyro to mouse**. The gyro then aims with the mouse while the sticks and buttons keep playing the game's own scheme.
- **Otherwise, try a mouse joystick approach**: map **gyro to the right stick**, shaped to emulate mouse movement as closely as possible, so the game's mouse aiming feels the same with a controller. Build the gyro and flick stick profiles on top of it.

## Strategy games

This covers every game played with a pointer on a map or a board: real-time and turn-based strategy, city builders, tycoon games and colony builders.

- **Ship a gyro variant** next to the plain profile, so the player can try a tilt-controlled cursor. Copy the plain profile and add `requires = ["gyro"]`, gyro-to-mouse at 1x to start, and a Select + Start chord for the clutch, like the [StarCraft pack](../../packs/starcraft.padpack) ("Gameplay + Gyro"). The gyro is a fine pointer next to the right stick, which keeps moving the cursor, so no button changes. Name it `<Game> + Gyro` and say so at the top of its notes.
- **A game with native controller support is exempt.** Its scheme is a passthrough (Factorio, Against the Storm), and a gyro-driven mouse would fight it.
- **The rules point to the plain profile.** Gyro is an extra and is never the default.

## Hotkeys and menus

- Useful hotkeys that are only used occasionally can go in a menu. Choose the menu type by what the menu holds:
  - **Carousel** for mode switches. Carousels suit mutually exclusive selections, where exactly one item is on at a time.
  - **Directional menu** for 4 or fewer options. Add matching **menu combos** (see [menus.md](../manual/menus.md)) so the items are quick to reach and easy to discover.
  - **Button menu** for 5 to 8 options. Also add menu combos for its items.
  - **Radial menu** for hotkeys on the number keys or function keys, when the game uses most of them. Prefer putting the menu on one of the **shoulder buttons**. Include every relevant hotkey as an item.
  - **List menu** for separate keys for in-game screens such as Map, Inventory, or Stats. Put the list on **Select** (see *System buttons*). This is the main exception to the directional menu rule.
  - **Grid menu as a dashboard** for a game with many loosely organized hotkeys. Use **cell merges** to give more important actions more space, so their weight matches how often they're used.
- If there is a **holster** or **put away** action, assign it to the same input that took the item out, with a **long press**. For example, if a shoulder button brings up a radial menu to take items out, a long press of that shoulder button holsters them.
- Whatever is on the **right mouse button** should go on the **left trigger**. The exception is when the right mouse button is a *Use* action, in which case **A** or **X** is preferred.
- **In-game menus** in PC games without controller support usually only work with the mouse. Some accept the arrow keys. Supporting arrow keys is often hard without a separate profile or mode, which can be unintuitive, but it may still be an acceptable option.

## Profiles, modes and layers

- **Profiles should not intermingle with each other.** Each profile should be a complete, self-contained scheme.
- **Macros** are generally left to the user. Don't add them to a pack unless the game clearly needs one.
- **Notes** are important. Use them for anything a player wouldn't guess from the layout. For example, in Deus Ex, Escape exits menus, including in-game menus such as robot recharging or keypads. Escape is also the system menu toggle, which is often bound to **Start**. So Start is how you exit in-game menus, and the note should say so.
- **Custom configs** go in the config folder. Don't add them to the guidelines or to packs for personal preferences.
- If a game needs a different control scheme for different in-game contexts (vehicles, minigames, the game's own menus), use **modes** instead of profiles. Only one mode is on at a time, so the player can't stack incompatible schemes and lose the way back.
- Use **layers** only to extend or fine-tune the scheme they're held over: a Lean layer that makes the left stick lean while a button is held, or a slower mouse curve while the aim button is held. Layers can't be toggled; a layer that should stay on is a mode.

## Vehicles and other contexts

- Vehicles should work out of the box if the game supports them. If they don't, they probably need **separate modes**, which the player switches to and back from themselves.
- If a game requires switching between several control schemes regularly, such as on foot and in vehicles, use **modes** switched through a **carousel menu**.
- The same goes for any context where the controls are radically different from the default, not just vehicles. Examples include:
  - **Minigames**, such as hacking, lockpicking, or card games, that use the sticks or buttons in a completely different way.
  - **Swimming, flying, climbing, or mounts**, where movement works differently from on foot.
  - **Turrets, sniping sequences, or cutscenes with interactive input**, where the player is controlling something other than their character.
  - **The game's own menus** when they need a different scheme from the rest of the game.
- Use a **mode** for these contexts when the player should stay in them until they choose to leave. A mode is only one on at a time, so the player can't stack schemes and lose their way back.
- Where the context is only a short step, such as a single lockpick, a **layer** may be enough, as long as it extends the scheme and doesn't replace it.

## System buttons

- **Back / Select / View** should typically open the in-game menu, often **Tab**. If the game has separate keys for the journal, inventory, map, and so on, Select should open a list menu with all of them.
- **Start** should be the system menu, usually **Escape**. If the system menu fails to pause the game, a **triple tap of Start** can act as pause instead. This is rare.
- **Quick save** should be a **double tap of Start**. **Quick load** should be a **long press of Start**.
- **Guide** should always show its own overlay in the corner with pseudo-global options. It should also show an info overlay explaining which keys are bound to what, using glyphs.

## Presentation

- Keyboard, numpad, and menu overlays should be colored and styled to match the game's theme as closely as possible.
- Animations and sounds should be assigned to match the game as well.
- **Use every fitting theming tool, but with restraint.** Craft the look to suit the game, and reach for what helps: fonts, colors, frames, textures, 3D buttons, filters, icons and sounds. Just because a theming feature exists doesn't mean the pack should use it. Each one should earn its place by making the overlay read more like the game or easier to use. A plain overlay that reads clearly beats a busy one with every effect switched on, and a texture or filter that competes with the text or the keys it sits over is a fault, not a flourish. When in doubt, leave it out, and check the result at text size with the overlay previews before keeping it.

### Text, icons and sounds

Menus and info overlays are read in a glance, mid-game. Set their text so that glance works. The tools are in the token picker (**Insert…** next to any text that takes tokens) and described in the [pack format](../pack-format.md#text).

- **Format text for legibility and emphasis.** Don't leave every label the same weight. Make the action's name stand out with `{b}…{/b}`, and mute or italicize the detail around it. Use `{color:red}` for what is dangerous or destructive (Quit, Delete, Drop all) and `{color:green}` for what is safe or restorative. `{u}…{/u}` marks a shortcut or a term the player has to find, and `{box}…{/box}` frames a key to hold. Keep it to a couple of accents a menu: when everything is emphasized, nothing is. Bold and italic need a font with those faces; check the font the pack sets draws them.
- **The app's own packs add to the built-in set instead of carrying assets.** A library pack (`packs/*.padpack`) carries no `[assets]`: its icons go in `assets/icons`, its sounds in `assets/sounds` (as a new `SoundKind`) and its fonts in `assets/fonts`, so every game can use them. Give them general names (`dagger`, `clank`), not the game's, and use only work that is fully open source: your own, CC0, or under a permissive license (OFL for fonts, ISC or MIT for icons) with its notice kept beside it. A test fails if a library pack carries assets or names an icon that isn't built in. Before drawing or synthesizing a new one, look through what's already there; another game's icon may fit. Pack assets (`[assets]`, below) are for packs made outside the app.
- **Use fonts with intent.** The overlay font should suit the game; use `{font:name}…{/font}` for the odd word that should stand apart, such as a title or a rune. One extra font a menu is plenty. If the game has a typeface of its own (or a free one close to it), carry it in the pack (`[assets.fonts]`, or **Details → Pack files → Add font…**) as a TTF or OTF, but only if its license lets you share it. Use a static file whose family name is the font's plain name (a variable font's default instance can be named "Kreon Light" and fail to resolve).
- **Put an icon in front of every menu item it helps.** An icon says what an item does faster than its label, and a row of identical text is hard to scan. Write the label as `{icon:potion} Heal`. Use the same icon for the same thing across the menus, and leave the icon off an item rather than reaching for one that doesn't fit.
- **Provide your own icons.** The built-in set is large (weapons, tools, buildings, time controls, and the icons each library pack brought), but a game may need more: a spell, a tool, a map, a faction. Draw the ones the game needs and carry them in the pack (`[assets.icons]`, or **Details → Pack files → Add icon…**). Match the game's look: a thin outline for a clean interface, a chunky filled shape for a cartoonish one.
- **An AI agent writing a pack should create icons that fit.** When the built-in icons don't suit an item, write an SVG for it instead of picking the nearest wrong one. Use a square `viewBox` (`0 0 24 24`), a few simple paths, and `fill='currentColor'` or `stroke='currentColor'` so it takes the text's color. No scripts, links or embedded pictures, and keep it well under 64 KB. Look at it at text size (`padwight debug overlay menu-styled`) before keeping it.
- **Add sound cues that suit the game.** The built-in sounds are tones, ticks, recordings and a few made for particular games (a Geiger counter, a blade, a camera shutter, a card). When none fits (a clank for a medieval menu, a radio click for a military one, a soft chime for a farming game), carry a short WAV in the pack (`[assets.sounds]`, or **Details → Pack files → Add sound…**) and point the overlay's cues at it. Keep cues short (a fraction of a second for steps, up to a second or so for picks), quiet, and not tiring over hundreds of repeats. Only use sounds you made or have the right to share, since a pack is shared.
- **An AI agent writing a pack should synthesize sounds when the built-in ones don't fit.** Generate a short mono WAV (16-bit PCM, 44.1 kHz) with a script: a decaying sine or noise burst, pitched and shaped to the sound wanted. Use the built-in kinds where they work; add files only when they add something. For a library pack, add the function to `scripts/synth-sounds.py` and register the new kind (see the bullet on the app's own packs).
