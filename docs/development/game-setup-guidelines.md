# Game setup guidelines

Conventions for choosing where actions go when setting up a game's controller bindings, for pack authors and the built-in library. They're defaults, not rules: a game's own conventions or a clearer layout win.

## Games with native controller support

- If the game is known to have **native XInput support**, even if it's imperfect, **don't change or add to its control scheme**. This program can't do better than a native scheme, so leave the bindings alone.
- **Games released before 2006 don't count as native XInput**, even if the store page lists controller support. XInput came out in December 2005, so an older game has at best spotty DirectInput support, which is unreliable. Map the gamepad to the keyboard and mouse bindings, as for a game with no controller support. Treat a store page's controller badge as a hint only, and check the release date first.
- **Check for native DualShock and Nintendo support too.** A game may support a DualShock 4, DualSense or Switch Pro Controller without XInput, or alongside it. Tick the matching kind under **Controllers this setup supports** on the Details tab, so the game sees that controller as itself and its own scheme holds for it. The same rules apply: explicit support counts, a store badge alone doesn't, and games released before 2006 don't count. The [first setup tutorial](../manual/tutorial-first-setup.md#3-check-for-native-controller-support) walks through the setting.
- Enhancements are still welcome where the game leaves a gap:
  - **Onscreen keyboard** for text input, such as naming a character, where the developer didn't plan for a controller.
  - **Mouse mode** for setup menus that only work with the mouse.
  - **Media controls**, so the player can listen to music or podcasts while playing, for example a roguelite card game.

## Games with more than one version

Classic PC games often come in more than one version: the original, a modern remaster or re-release, and sometimes a port. Often only one of them has controller support. Each version gets its own pack, and the two packs are handled differently:

- **A version with native controller support is a gamepad passthrough.** Every button, stick and trigger goes to the game as it is, the way the [Against the Storm pack](../../packs/against-the-storm.padpack) does it (`{gamepad = "South"}` and so on for each input). The Guide overlay still sits on top, and the notes say to set the buttons in the game's own options. Don't add bindings, shortcuts, layers or modes: they would only fight the game's scheme.
- **Keep the menus the same across versions.** Both packs should look and sound like the same game. Use the same overlay font, colors, keyboard and numpad styles, icons and sound cues in each, so a player who switches versions sees no change in the overlays.
- **A version without controller support follows the rest of this guide.** Build its scheme as usual, with the same presentation as the other version.

## Movement and camera

- **WASD movement** should generally be assigned to the **left stick**. Strategy games that use WASD or the arrow keys to pan the camera can also assign that to the left stick.
- **Camera or cursor control** should almost always be assigned to the **right stick**. As a rule of thumb, if the mouse controls it, it goes on the right stick.
- **Walk or slow movement** should be bound to the **left stick** as a zone covering the first **50 to 70 percent** of the stick's travel, so a light push walks and a full push runs. The right value depends on the controller and game. The Deus Ex pack uses 60 percent (see [the Deus Ex tutorial](../manual/tutorial-deus-ex.md)), which works well enough as a starting point.
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
- **Zoom / aim down sights** can be a **layer** held over the right stick, or over the zoom button, that swaps in a more fitting response curve, usually a slower and more precise one. This matches the way layers extend a scheme, and it's released as soon as the aim button is let go.
- **Hold or toggle for aim** depends on the game. If the game uses a toggle, make it a toggle. If the game uses a hold, make it a hold when it's on the **left trigger**, and a toggle when it's on any other input.
- **Gyro and flick stick profiles** should always be added for shooters, so the player can try them. They should **never be the default**. The default is the game's own scheme.

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

### Text, icons and sounds

Menus and info overlays are read in a glance, mid-game. Set their text so that glance works. The tools are in the token picker (**Insert…** next to any text that takes tokens) and described in the [pack format](../pack-format.md#text).

- **Format text for legibility and emphasis.** Don't leave every label the same weight. Make the action's name stand out with `{b}…{/b}`, and mute or italicize the detail around it. Use `{color:red}` for what is dangerous or destructive (Quit, Delete, Drop all) and `{color:green}` for what is safe or restorative. `{u}…{/u}` marks a shortcut or a term the player has to find, and `{box}…{/box}` frames a key to hold. Keep it to a couple of accents a menu: when everything is emphasized, nothing is. Bold and italic need a font with those faces; check the font the pack sets draws them.
- **Use fonts with intent.** The overlay font should suit the game; use `{font:name}…{/font}` for the odd word that should stand apart, such as a title or a rune. One extra font a menu is plenty. If the game has a typeface of its own (or a free one close to it), carry it in the pack (`[assets.fonts]`, or **Details → Pack files → Add font…**) as a TTF or OTF, but only if its license lets you share it.
- **Put an icon in front of every menu item it helps.** An icon says what an item does faster than its label, and a row of identical text is hard to scan. Write the label as `{icon:potion} Heal`. Use the same icon for the same thing across the menus, and leave the icon off an item rather than reaching for one that doesn't fit.
- **Provide your own icons.** The built-in icons are weapons, vehicles, a few items and a handful of symbols. Most games need more: a spell, a tool, a map, a faction. Draw the ones the game needs and carry them in the pack (`[assets.icons]`, or **Details → Pack files → Add icon…**). Match the game's look: a thin outline for a clean interface, a chunky filled shape for a cartoonish one.
- **An AI agent writing a pack should create icons that fit.** When the built-in icons don't suit an item, write an SVG for it instead of picking the nearest wrong one. Use a square `viewBox` (`0 0 24 24`), a few simple paths, and `fill='currentColor'` or `stroke='currentColor'` so it takes the text's color. No scripts, links or embedded pictures, and keep it well under 64 KB. Look at it at text size (`padwight debug overlay menu-styled`) before keeping it.
- **Add sound cues that suit the game.** The built-in sounds are tones and ticks. When none fits (a clank for a medieval menu, a radio click for a military one, a soft chime for a farming game), carry a short WAV in the pack (`[assets.sounds]`, or **Details → Pack files → Add sound…**) and point the overlay's cues at it. Keep cues short (a fraction of a second for steps, up to a second or so for picks), quiet, and not tiring over hundreds of repeats. Only use sounds you made or have the right to share, since a pack is shared.
- **An AI agent writing a pack should synthesize sounds when the built-in ones don't fit.** Generate a short mono WAV (16-bit PCM, 44.1 kHz) with a script: a decaying sine or noise burst, pitched and shaped to the sound wanted. Use the built-in kinds where they work; add files only when they add something.
