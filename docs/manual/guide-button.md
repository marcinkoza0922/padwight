# The Guide button

The Guide button (the PlayStation, Xbox or Home button) works the same in every game. Padwight doesn't pass it to the game or to Steam, so a power button is safe to press.

## The Guide overlay

Tap Guide to open the overlay, and tap it again to close it. It opens when you let go, so a quick press with another button is a shortcut, not an overlay.

The overlay has four parts:

- **Notes** on the left: what the game's author wants you to know, such as limits in the control scheme, or when to switch modes.
- **The menu** in the middle: Quick Settings, Edit Controls, the on-screen keyboard and numpad, mouse mode, media controls, Switch Profile (to pick another profile of the game, or make a new one) and, while Steam runs, the Steam overlay.
- **Mappings** on the right: what each button, trigger and stick does in the current profile and [mode](modes.md). Its text can be changed, and rows can be hidden or merged by the author on the profile's **Guide** tab. Each [layer](layers.md) a button holds gets a section of its own below, with what the layer changes. When the setup has modes, press Left or Right (D-pad or Left Stick) to look at what the other modes do; the heading says which mode is shown, and "(not on)" when it isn't the one you're in. A long list shows a page at a time: push the Right Stick up or down to turn the pages.
- **Shortcuts** in the bottom left corner: the Guide shortcuts listed below.

Move the menu with the D-pad or the Left Stick (hold it to repeat), choose with A, and close it with B.

## Shortcuts

Hold Guide and press one of these, or, with the overlay open, press it alone. Either way the overlay closes and the shortcut runs.

- **X**: on-screen keyboard.
- **Y**: on-screen numpad.
- **LB**: media controls.
- **RB**: take a screenshot.
- **L3**: start or stop a screen recording.
- **Start**: the in-game menu, with Quick Settings and Edit Controls (see [Changing controls while you play](in-game-menu.md)). Hold Guide and press Start, or press Start alone with the overlay open.
- **Hold Select for 2 seconds**: force quit the focused window.

Pressing Guide (or whatever opens the overlay, if you moved it) also closes anything else on screen, whichever way it was opened: the on-screen keyboard or numpad, the media controls, a menu, the in-game menu (it asks first if you changed something and haven't saved), or the offer to add a game (as "Not now").

Anything else pressed with Guide held does nothing, so it never reaches the game underneath. These are the defaults, and the same in every game. You can move them on **App settings › Controller shortcuts** (see Moving the shortcuts, below). You can also bind the same actions to other buttons on a profile's **Buttons** tab.

## Mouse mode

Hold Guide and press Right Stick (or press Right Stick with the overlay open) to turn on mouse mode. (Both of those are the defaults; see Moving the shortcuts, below.) The overlay closes, and the bottom left corner lists the controls:

- **Right Stick**: moves the mouse.
- **Left Stick**: scrolls. Pressing it clicks the middle button.
- **RT / LT**: left click / right click.
- **LB / RB**: back / forward.
- **D-pad Right, Down, Left**: Enter, Tab and Escape.
- **D-pad Up**: holds Shift, so a click or key with it is shifted.
- **X**: holds Ctrl. **Y**: holds Alt. **B**: Delete.

Press Guide again (or whatever opens the overlay) to leave mouse mode. Your profile and [mode](modes.md) are still the ones you chose. Other buttons do nothing in mouse mode, except those that switch modes. Mouse mode is one of the built-in modes, so a button set to **Switch mode…** "Mouse mode" turns it on too.

## Moving the shortcuts

Open **App settings** and pick the **Controller shortcuts** tab. What you set there applies to every setup. It can't be changed for one game.

Each shortcut sits on one of these:

- **Button**: a single button.
- **Double tap**, **Triple tap** or **Long press**: a gesture of a single button.
- **Combo**: two to four buttons pressed together, such as Start + Select.
- **Off**: nothing. A shortcut you switch off stays off, and nothing warns you about it, except the one that opens the overlay (below).

Next to each button, **Set by pressing** waits for you to press it on the controller, so you don't need to find its name in the list. **Reset** puts a shortcut back, and **Put every shortcut back** at the bottom resets them all.

### What opens the Guide overlay

By default it's the Guide button. A controller without one (many don't have one) can use something else, such as Start + Select as a combo, or a long press of Select.

- As a **button**, it works as Guide does: tap it to open or close the overlay, or hold it and press a shortcut. The button no longer does anything else, so a game never gets it.
- As a **gesture** or **combo**, it opens and closes the overlay. The shortcuts are then pressed with the overlay open, since there is nothing to hold.
- Whatever it is, it also leaves mouse mode, as Guide does.

The Guide button itself is reserved for this: no other shortcut can sit on it, and a profile can't map it. If you move the opener somewhere else, Guide does nothing and still never reaches the game or Steam.

If you set the opener to **Off**, Padwight lets you, but warns you on the tab: without it the overlay, its menu, the Guide shortcuts and mouse mode can't be reached.

A profile that already uses the input you chose would do both things at once. The tab lists those setups and the profile in each, and a ⚠ shows beside them in the sidebar. Change either the opener or that profile's mapping.

### The Guide shortcuts and mouse mode's keys

The tab lists the shortcuts of the Guide mode (keyboard, numpad, media controls, screenshot, recording, force quit and mouse mode) and the keys of mouse mode (Shift, Enter, Tab, Escape, Ctrl, Alt, Delete, middle click, mouse back and forward). The sticks and triggers in mouse mode stay as they are.

When two shortcuts of the same mode share an input, or a shortcut shares one with what opens the overlay, a ⚠ marks both rows. They still save, but only one of them can work. A combo clashes with its own buttons used alone, and with a bigger or smaller combo that shares them.

The same goes for the panels the shortcuts open. The on-screen keyboard, numpad and media controls close with the opener held and their own shortcut again (wherever you moved it), and the opener alone closes any panel, a double tap, long press or combo included.

Wherever you put the opener, the hints in the overlays that say "press Guide" show it instead: its glyph for a button, glyphs joined by + for a combo, or "double tap" and a glyph. The same goes for `{guide}` in the text of your own info overlays.

### Confirm and back

The last group on the tab, **Menu buttons**, sets which buttons choose and go back (A and B unless you move them). They work in the Guide overlay's menu, the on-screen keyboard and numpad, the media controls, the in-game menu, the library offer, and menus that are a list, grid or carousel. Hints on screen show your buttons.

Each is one button, and only buttons those panels don't use for anything else are offered: A, B, Select, the stick clicks and the back paddles. The button you pick and A (or B) trade places, so with Confirm on Select, Select chooses and A does what Select did, which is nothing. Putting Confirm on B and Back on A swaps the two, as on a Nintendo controller.

A menu that sets its own close button keeps it, and menus that read the face buttons themselves (directional and button menus) aren't changed. The tab warns when Confirm and Back share a button, or when one sits on a Guide shortcut, because the overlay would hear both.

## Force quit

Holding Select for 2 seconds closes the program in the focused window. It asks the program to close, and if it doesn't, stops it after 3 seconds. Letting go early cancels it. The desktop and Padwight itself are never closed.

## Screenshots and recordings

Screenshots are saved to `Pictures/Screenshots`, in a folder for the game. Recordings are made with gpu-screen-recorder and saved to `Videos/Recordings`, in a folder for the game. If gpu-screen-recorder isn't installed, Padwight shows a message saying so.

## Steam

Steam's overlay is reached from the menu, and only while Steam is running. Guide itself never reaches Steam.
