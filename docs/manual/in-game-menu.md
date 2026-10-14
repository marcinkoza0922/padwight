# Changing controls while you play

Hold Guide and press Start to open the in-game menu over the game you're playing. (That's the Quick Settings shortcut of the Guide mode. It can be moved on **App settings › Controller shortcuts**, and Guide can be something else; see [The Guide button](guide-button.md).) It's built into Padwight, so it works for every game. Move with the Left Stick or D-pad, press A to choose, and press B to go back or close it.

## Quick Settings

- Left Stick and Right Stick speed, for sticks that move the mouse
- gyro sensitivity, when gyro moves the mouse
- Invert Y, which flips every up-and-down setting in the profile at once
- profile, to change to another profile of the setup
- gyro on or off (turning it back on brings back the gyro as it was when you opened the menu)
- Left Stick and Right Stick dead zones
- rumble strength, for every game (also on the settings window's Controllers page)
- overlay size, for the on-screen keyboard and numpad, the media controls and the menus
- Nintendo button layout, which swaps the A/B and X/Y labels (and, on a Nintendo pad, what the face buttons send) for this setup
- Turn remapping off, a way out if a mapping gets in the way: press A twice, and the controller goes straight to the game until you turn remapping back on from the tray or the settings window (or restart Padwight)

Left and right change a number by about 10%, or by half with RB held, and keep changing it while held. A number you changed shows what it was when you opened the menu. A flips a switch or picks the next profile.

## Switch Profile

**Switch profile** on the Guide menu lists the profiles of the game in front of you (or General's, outside a game). The one you're on says "(current)", and one your controller can't play says what it needs, such as "(needs gyro)". Pick one to switch to it and go back to the game.

**New profile…** at the bottom makes another profile and switches to it. It can start as a copy of the profile you're on, from one of the templates the settings window offers, or, in a game, as a copy of one of General's profiles. It gets a free name, such as `Gameplay 2`. A new profile is a change like any other in the menu: it's written to the config file only when you save.

## Edit Controls

Opens the full editor for the game in front of you: its buttons (and back paddles, on a controller that has them), sticks, gyro, combos, modes, layers, macros, menus and info overlays. Changes apply straight away, but they're only written to the config file when you save.

- **Menus**: add a menu, add items and pick each one's action (an item is named after its action), set an item's own button in a button menu, move items up and down, change the menu's kind with left and right, and remove items or the menu. Naming a menu, and a menu that a button opens, are left to the settings window.
- **Info overlays**: choose whether each one is always on screen, and move it round the screen with left and right.

Deleting a combo, zone, macro, menu item or menu asks you to press A a second time. LB and RB jump a page up or down a long list.

If you leave with unsaved changes, a question appears over the page: **A** saves, **X** discards, and **B** keeps editing. After saving or discarding, B (back) takes you to the Guide overlay you came from, and Guide closes the menu.

## When no setup matches the window

If a window is in front but no setup matches it, a change creates a new setup for that window. It's named after the program, such as `eldenring` for `eldenring.exe`, and starts from General's active profile with your change. General stays as it was.

> **Tip:** the top line of the menu shows where you are, such as `Menu › Edit Controls › A button`.
