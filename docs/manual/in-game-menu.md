# Changing controls while you play

Hold Guide and press Start to open the in-game menu over the game you're playing. (That's the Quick Settings shortcut of the Guide mode. It can be moved on **App settings › Controller shortcuts**, and Guide can be something else; see [The Guide button](guide-button.md).) It's built into Padwight, so it works for every game. Move with the Left Stick or D-pad, press A to choose, and press B to go back or close it.

## Quick Settings

- Left Stick and Right Stick speed, for sticks that move the mouse
- gyro sensitivity, when gyro moves the mouse
- Invert Y, which flips every up-and-down setting in the profile at once
- profile, to change to another profile of the setup
- gyro on or off (turning it back on brings back the gyro as it was when you opened the menu)
- Left Stick and Right Stick dead zones
- rumble strength, for every game (also on the settings window's Overview page)
- overlay size, for the on-screen keyboard and numpad, the media controls and the menus
- Nintendo button layout, which swaps the A/B and X/Y labels (and, on a Nintendo pad, what the face buttons send) for this setup
- Turn remapping off, a way out if a mapping gets in the way: press A twice, and the controller goes straight to the game until you turn remapping back on from the tray or the settings window (or restart Padwight)

Left and right change a number by about 10%, or by half with RB held, and keep changing it while held. A number you changed shows what it was when you opened the menu. A flips a switch or picks the next profile.

## Switch Profile

**Switch profile** on the Guide menu lists the profiles of the game in front of you (or General's, outside a game). The one you're on says "(current)", and one your controller can't play says what it needs, such as "(needs gyro)". Pick one to switch to it and go back to the game.

**New profile…** at the bottom makes another profile and switches to it. It can start as a copy of the profile you're on, from one of the templates the settings window offers, or, in a game, as a copy of one of General's profiles. It gets a free name, such as `Gameplay 2`. A new profile is a change like any other in the menu: it's written to the config file only when you save.

## Edit Controls

Opens the full editor for the profile you're on and the game in front of you. Its first page lists what can be edited: **Buttons** (and back paddles, on a controller that has them), **Sticks**, **Triggers**, **Gyro**, **Touchpad**, **Combos**, **Modes**, **Layers**, **Macros**, **Menus**, **Info overlays**, **Log overlays**, the **Guide overlay**, the **Profile** and the **Game**. Each opens its own list, and each item there opens its settings. Almost anything the settings window does for a game can be done here, down to each stick's response curve, each layer's own sticks and gyro, a menu's look and the game's window rules. Changes apply straight away, but they're only written to the config file when you save.

- On a row with a value, left and right change it. A chooses. B goes back a page.
- Names, labels, overlay text and colors are typed on the on-screen keyboard: its keys go into the field instead of the game. Start sets the text; hold B to put the keyboard away without changing it.
- Pages about a stick, trigger, gyro or the touchpad show pictures that follow your controller as you use it. A menu being edited is drawn beside its pages, and an info or log overlay being edited shows where it goes.
- Deleting anything asks you to press A a second time. LB and RB jump a page up or down a long list.

The wiki's *Editing controls in game* page lists every page of the editor.

If you leave with unsaved changes, a question appears over the page: **A** saves, **X** discards, and **B** keeps editing. After saving or discarding, B (back) takes you to the Guide overlay you came from, and Guide closes the menu.

## When no setup matches the window

If a window is in front but no setup matches it, a change creates a new setup for that window. It's named after the program, such as `eldenring` for `eldenring.exe`, and starts from General's active profile with your change. General stays as it was.

> **Tip:** the top line of the menu shows where you are, such as `Menu › Edit Controls › A button`.
