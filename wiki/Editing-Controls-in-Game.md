# Editing controls in game

Edit Controls, in the [in-game menu](In-Game-Menu), changes the game you're playing without leaving it. It edits the profile you're on and the things the whole setup shares (its modes, layers, macros, menus, overlays and details), so other setups and General aren't touched. Changes apply straight away. They're written to the config file only when you save.

Almost anything the settings window does for a game can be done here. What's left to the settings window: a look's background picture, a cue's sound file from a pack, a pack's own details (author, version), and importing or exporting packs.

## Moving around

- **D-pad** or **Left Stick** move the cursor. **A** chooses. **B** goes back a page.
- On a row with a value (a number, a switch, a choice), **left** and **right** change it. **A** steps it forward too. Pages with values stay in one column and scroll, so left and right are free for changing them.
- **LB** and **RB** jump a page up or down a long list.
- Deleting anything takes a second press of **A**.
- A row that adds something is tinted lime, and a row that removes or deletes something is tinted red (see [colours](In-Game-Menu#colours-and-glyphs)). The top line shows where you are, such as `Guide › Edit Controls › Buttons`.

## Typing

Names, labels, overlay text, window rules and colors are typed on the on-screen keyboard. The field's name and its text show above the keys, and the keys go into the field, not the game. **A** types the key under the cursor, **X** deletes (hold it to keep deleting), **Y** types a space, and hold **LT** for Shift. **Start** (or the keyboard's Enter key) sets the text. Hold **B**, or press Guide, to put the keyboard away without changing anything.

## Pictures

Pages about a stick, a trigger, the gyro or the touchpad show the settings window's pictures beside the list, and they follow your controller as you use it: where the stick is and what its deadzone and curve do with it, how far the trigger is pulled against its press point and zones, how fast you're turning on the gyro's curve, and your fingers on the touchpad. On a Left Stick page, only the D-pad moves the cursor, so you can watch the stick.

A menu, info overlay or log overlay you are editing shows on screen where it goes, under the editor, and follows each change. Hold the left trigger to hide the editor and see the overlay whole; the trigger is chosen in the app's Shortcuts tab, in the Menu controls card ("Show the overlay being edited"), or switched off. A new menu with no items has nothing to show yet.

A menu being edited is drawn beside its pages as it will look, and an info or log overlay being edited shows on screen where it goes. A log overlay shows sample presses.

## The top page

**Buttons**, **Sticks**, **Triggers**, **Gyro**, **Touchpad**, **Combos**, **Modes**, **Layers**, **Macros**, **Menus**, **Info overlays**, **Log overlays**, **Guide overlay**, **Profile** and **Game**. Each opens its own list.

## Actions

Anything that holds an action (a button, a gesture, a combo, a trigger, a zone, a ring sector, a touchpad click or swipe, a menu item, a macro step) opens the same page. Its first row says what it does now. Below that:

- **Gamepad button…** sends another pad button.
- **Gestures…** (a button's page) opens its double tap, triple tap and long press.
- Mouse clicks, the wheel in four directions, the on-screen keyboard and numpad, a screenshot, recording, the media controls, Quick Settings and force quit. Choose **Disabled** to make it do nothing.
- **Keys…** opens the full keyboard, a row at a time. Tick keys to press them together, such as Ctrl and C.
- **Switch to a mode…**, **Hold a layer…**, **Play a macro…**, **Open a menu…**, **Show an info overlay…** and **Show a log overlay…** list what the game has.
- **Several at once…** presses several actions together. Its page lists them, each with its own action page, and adds or removes them.
- **Make it a toggle** and **Make it turbo** wrap the action. A wrapped action's page shows what it wraps on a page of its own, so you can change that and keep the wrapper. A toggle can start on, a turbo has its rate (and how often it plays a macro), and **Stop toggling** or **Stop turbo** takes the wrapper off. A macro can repeat while held.
- **Not set** clears a gesture, or a button a mode or layer leaves to the profile.

A menu item can't hold a layer or be a turbo. A macro step presses keys, buttons, clicks or the wheel.

## Sticks

**Kind** is Off, Gamepad stick, Mouse pointer, Scroll wheel, Button ring or Flick stick, each with its settings:

- **Gamepad stick**: which stick it sends, Invert Y, curve.
- **Mouse pointer**: speed, Invert Y, a response preset (Linear, Gentle, Aggressive, Wide, Extra wide, Precision, Turn boost) and its curve, acceleration, ramp time and outer boost, then vertical speed and smoothing.
- **Scroll wheel**: speed, Invert Y, curve.
- **Button ring**: 4, 8 or 12 sectors, where the first one points, how far out the ring starts, stickiness, each sector's action, and fills for WASD and the arrows.
- **Flick stick**: turn size, flick point, flick time, turning smoothing, dead angle, and up-and-down look with its speed.

Every kind has a deadzone, how far out its direction buttons press, and **Zones**.

## Triggers and zones

A trigger is Off, a Gamepad trigger (which one it sends), or a Button with an action and a press point. A zone holds its action while a stick or trigger is pushed within its range: **From** and **To** move its edges (the start can't pass the end). A new zone covers the top half of the travel.

## Gyro

**Mode** is Off, Mouse, Stick (turning speed), Stick (position) or Steering, each with its settings: sensitivity and real-world sensitivity with the game's full turn, the stick it sends, how far is a full push, anti-deadzone and locking at the edge. Then how turning is read, which axes it moves, each axis's speed and inversion, the response preset and its curve, when it's on (always, while held, off while held, or toggled, with the input picked from every button and trigger, and a rumble on toggle), what recenters it, which stick pauses it, the jitter and smoothing thresholds, and drift re-measuring.

## Touchpad

What fingers do (Off, Mouse pointer, Gamepad stick, Menu zones or Swipes) with each kind's settings, what clicking does, and **Split into halves**, which gives the right half its own page.

## Combos and gestures

A combo ticks its buttons (two or more), has an action, and can be deleted. A button's gestures are on its action page. Their timings are on the **Profile** page.

## Modes and layers

**Add mode** or **Add layer** makes one. Its page has its name, its buttons (a button it doesn't set is left to the profile), its combos, the profile's combos it turns off, and its own sticks, triggers, gyro and touchpad: each says whether it's the profile's or its own. **Set it here** starts it from the profile's, and **Use the profile's** gives it back. Then whether other buttons do nothing while it's on, what shows while it's on (its name, its bindings, an info overlay, or nothing), after how long, the bindings sheet's heading, and the name label's look. A mode or layer something still uses can't be deleted.

## Macros

A macro has its name, its steps, and **Add a step…**: a tap, holding down, letting go, a wait, moving a stick, or a fighting-game motion (quarter circles, half circles, dragon punch). A step's page sets what it presses and for how long, or where the stick goes (by direction, or exactly), and moves or deletes it.

## Menus

A menu has its name, its items, its kind (radial, directional, list, buttons, carousel or grid) with the kind's settings (the stick that aims it, boxes, the buttons that pick, what turns a carousel, a grid's columns and starting item), what closes it, whether it stays up after a choice, and its look. An item has its label, its action, and what its menu's kind gives it: a button to pick it with, its share of a radial menu, or its size in a grid. Items move up and down.

## Info and log overlays

An info overlay has its name, whether it's always on screen, how long it shows when the game starts and after it's let go, its heading, its cells row by row (each typed, with the same `{tokens}` as the settings window), what its `{current_input}` cells follow, its position and its look.

A log overlay has its name, whether it's always on screen, how long it stays, what it follows (a controller, which inputs, how many per line and the pause that starts a new one), its lines, where the newest goes, fading, merged repeats, labels, hold times, its position and its look.

## Looks

A look page sets where an overlay sits and how far it's moved from there, its size, width and height (or fitted to its contents), the widest and tallest it gets, its corners, the colors of its background, items and selected item (from swatches, or typed as `#rrggbb`, with opacity), its text (case, spacing, size), and its decorations: outlines, shadow, gradient, glow, lines between items, a corner frame, scanlines and a vignette.

## Guide overlay

The notes for the Notes overlay, a line at a time, and the rows of the Mappings list: each row's text, whether it's shown, sharing it with other inputs (and splitting a shared row), and rows of your own.

## Profile

Its name, what a controller needs to play it (gyro, back paddles, a touchpad), and how long taps count together, a long press takes, and combo buttons wait for each other. Making and switching profiles is in [Switch Profile](In-Game-Menu#switch-profile) on the Guide menu.

## Game

Its name, the window rules that pick it (what they match, the value, the profile they switch to, on or off), its profiles (rename, copy, move, delete; deleting the one you're on moves to another), the Nintendo button layout, its font, its own looks for the keyboard, numpad, media controls and in-game menu, and its own sounds (each cue's sound, volume, pitch, length and effects), rumble (each buzz's strength and length) and motion. Renaming the game, a profile or any item keeps everything that names it pointing at it.
