# On-screen keyboard, numpad and overlays

Padwight can draw on top of a game, even a fullscreen one, without taking keyboard focus. Your key presses still go to the game underneath.

This needs a desktop that supports it: KDE Plasma, Sway, Hyprland or labwc. GNOME doesn't. In a Flatpak, it doesn't show on Sway, labwc or Hyprland.

## On-screen keyboard

Open it with the **On-screen keyboard** action, or from the Overview page. Control it with the controller:

- **D-pad or Left Stick**: moves between keys. Keeps moving while you hold it.
- **A**: presses the selected key. Hold A to hold the key down.
- **Shift, Ctrl, Alt, AltGr, Super**: stay on for the next key only, so you can type a shortcut like Ctrl + C. Press one again to let go of it.
- **X**: Backspace. Hold it to keep deleting.
- **Y, Start**: Space and Enter.
- **LB, RB**: move the text cursor left and right.
- **Hold LT**: holds Shift.
- **Hold B, or press Guide**: closes the keyboard.

The keys are labeled in your keyboard layout, and change while Shift is on. On the App settings page, choose **Basic** keys (what typing into a game needs, the default) or **Full** ones (function keys and editing keys too).

## On-screen numpad

A small pad with the digits 0 to 9, a dot, a comma, a minus, Backspace and Enter, for codes and number fields. Move with the D-pad or the Left Stick. A presses a key, X is backspace, Start is Enter, and holding B or pressing Guide closes it. It works whatever the Num Lock setting is.

## Info overlays

Text and button pictures drawn on screen, such as a cheat sheet. An overlay can be always on, or shown when an action or a layer asks for it. Button pictures match the controller in use: Xbox, PlayStation or Nintendo.

## Log overlays

A small list of the buttons you pressed lately, and what each one did. It's useful when you're checking what a mapping does.

## Media controls

Controls the music or video player that's running. The D-pad left and right seek, up and down change the system's volume (the player's own, if the system's can't be changed), X mutes or unmutes, the bumpers change track, A plays or pauses, Y chooses the player, and B closes.

> **Tip:** each overlay has its own position, size and colors. Set them on the App settings page, or drag it into place with **Edit on screen** (see below).

## Editing an overlay on screen

**Edit on screen**, beside an overlay's or menu's **Appearance**, shows it full size over everything, where it will appear in game, with a panel of settings at one side. Drag the overlay to move it: it takes the nearest of the nine screen spots and grows from there. Drag an edge or corner to resize it. With **Snap to the grid** on, the edges you move land on the grid (8 to 64 px), or back on the spot itself; hold Alt to place it freely, and use the arrow keys to nudge it (Shift for 10 px). The panel's **Look** tab has the same settings as Appearance, and **Content** a menu's items or an info overlay's cells, so what you add shows on the overlay as you type. Click an item of a menu or a cell of an info overlay on screen to open it in **Content**, ready to type in. Other overlays can be drawn faintly where they go, so you can place this one around them: tick them under **Also show** on the **Layout** tab (the setup's menus, info and log overlays, and the keyboard, numpad, media controls and in-game menu in the look the setup uses). The list scrolls when there are many. The info and log overlays are ticked at first. **Flash**, on the same tab, makes the overlay pulse in yellow for a moment, for when it is hard to make out against a dark wallpaper. An info or log overlay at the same spot as the one you're editing isn't listed, since on screen the two stack there. A setup's own look for the keyboard, numpad, media controls or in-game menu (on its **Details** tab) has its own **Edit on screen** too. **Undo** and **Save & apply** work as they do in the settings window; **Done** or Esc closes the editor, and Ctrl+S saves. Fold the panel with **–** to see what's behind it, or move it to the other side with **⇆**. On a desktop without compositing, the screen behind the editor shows black.

## Giving a game's overlays their own look

Under each overlay's colors, **Decorations** lists extras that are off until you tick them: an outline around the panel, its shadow, a gradient that fades the background into a second color, an outline on every item, a glow around the selected item, and a color for the panel's text. A setup can have its own look on its Details tab, so a fantasy game's menus can glow gold on dark red while a shooter's stay flat and square.

If overlays make your computer stutter, the Performance card on the App settings page turns decorations, motion and sounds off everywhere.
